//! Optional Turso (libSQL) backing store.
//!
//! When configured (see [`load_config`]/[`save_config`]), every add/remove
//! is mirrored to a remote `people` table, and a background task applies
//! changes made from *other* clients back into this app.
//!
//! Turso's Change Data Capture (<https://docs.turso.tech/tursodb/cdc>) is a
//! **queryable table** (`turso_cdc`), not a push/webhook feed — there is no
//! server-initiated notification for remote libSQL clients. The closest
//! thing to "realtime without polling" Turso actually offers is cheaply
//! querying `turso_cdc` for rows newer than the last seen `change_id` over
//! an already-open connection, which is what [`poll_changes`] does. The
//! caller is expected to call this on a modest interval (seconds, not a
//! tight loop) rather than re-fetching the whole table.

use color_eyre::Result;
use libsql::{Builder, Connection};

use crate::person::{Gender, Person};

const ENV_PATH: &str = ".env.production";
const KEY_URL: &str = "TURSO_DATABASE_URL";
const KEY_TOKEN: &str = "TURSO_AUTH_TOKEN";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbConfig {
    pub url: String,
    pub token: String,
}

impl DbConfig {
    pub fn is_usable(&self) -> bool {
        !self.url.trim().is_empty() && !self.token.trim().is_empty()
    }
}

/// Parse a `.env.production`-style file (`KEY=VALUE` lines, `#` comments)
/// at the given path. Returns a default/empty config when absent.
pub fn load_config_at(path: &std::path::Path) -> DbConfig {
    let Ok(text) = std::fs::read_to_string(path) else {
        return DbConfig::default();
    };
    let mut cfg = DbConfig::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"').to_string();
            match key.trim() {
                KEY_URL => cfg.url = value,
                KEY_TOKEN => cfg.token = value,
                _ => {}
            }
        }
    }
    cfg
}

/// Overwrite the file at `path` with the given config.
pub fn save_config_at(path: &std::path::Path, cfg: &DbConfig) -> Result<()> {
    let contents = format!("{KEY_URL}={}\n{KEY_TOKEN}={}\n", cfg.url, cfg.token);
    std::fs::write(path, contents)?;
    Ok(())
}

/// Parse `.env.production` in the current working directory.
pub fn load_config() -> DbConfig {
    load_config_at(std::path::Path::new(ENV_PATH))
}

/// Overwrite `.env.production` in the current working directory.
pub fn save_config(cfg: &DbConfig) -> Result<()> {
    save_config_at(std::path::Path::new(ENV_PATH), cfg)
}

pub async fn connect(cfg: &DbConfig) -> Result<Connection> {
    let db = Builder::new_remote(cfg.url.clone(), cfg.token.clone())
        .build()
        .await?;
    let conn = db.connect()?;
    ensure_schema(&conn).await?;
    Ok(conn)
}

async fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS people (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            class TEXT NOT NULL,
            school TEXT NOT NULL,
            gender TEXT NOT NULL,
            grp INTEGER NOT NULL,
            avatar_seed INTEGER NOT NULL
        )",
        (),
    )
    .await?;
    // Older libSQL-engine databases don't understand this pragma — that's
    // fine, `poll_changes` detects the failure and the caller falls back to
    // a full `fetch_all` refresh instead of incremental CDC.
    let _ = conn
        .execute("PRAGMA capture_data_changes_conn('full')", ())
        .await;
    Ok(())
}

fn gender_to_sql(gender: Gender) -> &'static str {
    match gender {
        Gender::Male => "Male",
        Gender::Female => "Female",
    }
}

fn gender_from_sql(s: &str) -> Gender {
    if s == "Female" { Gender::Female } else { Gender::Male }
}

pub async fn fetch_all(conn: &Connection) -> Result<Vec<Person>> {
    let mut rows = conn
        .query(
            "SELECT id,name,class,school,gender,grp,avatar_seed FROM people ORDER BY id",
            (),
        )
        .await?;
    let mut people = Vec::new();
    while let Some(row) = rows.next().await? {
        people.push(row_to_person(&row)?);
    }
    Ok(people)
}

fn row_to_person(row: &libsql::Row) -> Result<Person> {
    let id: i64 = row.get(0)?;
    let name: String = row.get(1)?;
    let class: String = row.get(2)?;
    let school: String = row.get(3)?;
    let gender: String = row.get(4)?;
    let group: i64 = row.get(5)?;
    let avatar_seed: i64 = row.get(6)?;
    Ok(Person {
        id: id as u64,
        name,
        class,
        school,
        gender: gender_from_sql(&gender),
        group: group as u8,
        avatar_seed: avatar_seed as u64,
    })
}

pub async fn upsert_person(conn: &Connection, person: &Person) -> Result<()> {
    conn.execute(
        "INSERT INTO people (id,name,class,school,gender,grp,avatar_seed) VALUES (?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, class=excluded.class, school=excluded.school,
            gender=excluded.gender, grp=excluded.grp, avatar_seed=excluded.avatar_seed",
        (
            person.id as i64,
            person.name.clone(),
            person.class.clone(),
            person.school.clone(),
            gender_to_sql(person.gender).to_string(),
            person.group as i64,
            person.avatar_seed as i64,
        ),
    )
    .await?;
    Ok(())
}

pub async fn delete_person(conn: &Connection, id: u64) -> Result<()> {
    conn.execute("DELETE FROM people WHERE id = ?1", [id as i64])
        .await?;
    Ok(())
}

/// One CDC row translated into an application-level change.
pub enum ChangeEvent {
    Upsert(Person),
    Delete(u64),
}

/// Poll `turso_cdc` for rows affecting `people` newer than
/// `since_change_id`. Returns `Ok(None)` when CDC isn't available on this
/// database (pragma unsupported, or `turso_cdc` missing) so the caller can
/// fall back to [`fetch_all`].
pub async fn poll_changes(
    conn: &Connection,
    since_change_id: i64,
) -> Result<Option<(i64, Vec<ChangeEvent>)>> {
    let query = "SELECT change_id, change_type, id,
            bin_record_json_object(table_columns_json_array('people'), after) AS after_json
        FROM turso_cdc
        WHERE table_name = 'people' AND change_id > ?1 AND change_type != 2
        ORDER BY change_id";

    let mut rows = match conn.query(query, [since_change_id]).await {
        Ok(rows) => rows,
        Err(_) => return Ok(None),
    };

    let mut max_id = since_change_id;
    let mut changes = Vec::new();
    while let Some(row) = rows.next().await? {
        let change_id: i64 = row.get(0)?;
        let change_type: i64 = row.get(1)?;
        max_id = max_id.max(change_id);
        if change_type == -1 {
            let id: i64 = row.get(2)?;
            changes.push(ChangeEvent::Delete(id as u64));
        } else if let Ok(Some(json)) = row.get::<Option<String>>(3) {
            if let Ok(person) = person_from_json(&json) {
                changes.push(ChangeEvent::Upsert(person));
            }
        }
    }
    Ok(Some((max_id, changes)))
}

fn person_from_json(json: &str) -> Result<Person> {
    #[derive(serde::Deserialize)]
    struct Raw {
        id: i64,
        name: String,
        class: String,
        school: String,
        gender: String,
        grp: i64,
        avatar_seed: i64,
    }
    let raw: Raw = serde_json::from_str(json)?;
    Ok(Person {
        id: raw.id as u64,
        name: raw.name,
        class: raw.class,
        school: raw.school,
        gender: gender_from_sql(&raw.gender),
        group: raw.grp as u8,
        avatar_seed: raw.avatar_seed as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_env_file_yields_unusable_default_config() {
        let path = std::env::temp_dir().join("randomly-divide-db-test-missing.env");
        let _ = std::fs::remove_file(&path);
        let cfg = load_config_at(&path);
        assert!(!cfg.is_usable());
    }

    #[test]
    fn save_then_load_round_trips_url_and_token() {
        let path = std::env::temp_dir().join("randomly-divide-db-test-roundtrip.env");
        let cfg = DbConfig {
            url: "libsql://example.turso.io".to_string(),
            token: "secret-token".to_string(),
        };
        save_config_at(&path, &cfg).unwrap();
        let loaded = load_config_at(&path);
        std::fs::remove_file(&path).unwrap();

        assert_eq!(loaded, cfg);
        assert!(loaded.is_usable());
    }
}
