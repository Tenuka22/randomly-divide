use std::path::Path;

use color_eyre::Result;

use crate::person::Person;

/// Load all people from `path`. A missing file is treated as an empty roster
/// (it will be created on first save), matching a fresh install.
pub fn load(path: &Path) -> Result<Vec<Person>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut reader = csv::Reader::from_path(path)?;
    let mut people = Vec::new();
    for record in reader.deserialize() {
        let person: Person = record?;
        people.push(person);
    }
    Ok(people)
}

/// Rewrite the whole CSV file from the current in-memory roster. The roster
/// is small (a few hundred rows at most) so a full rewrite on every change
/// keeps the persistence logic simple and crash-safe (no partial appends).
pub fn save(path: &Path, people: &[Person]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    for person in people {
        writer.serialize(person)?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_loads_as_empty_roster() {
        let path = std::env::temp_dir().join("randomly-divide-test-missing.csv");
        let _ = std::fs::remove_file(&path);
        let people = load(&path).unwrap();
        assert!(people.is_empty());
    }

    #[test]
    fn save_then_load_round_trips_exactly() {
        let path = std::env::temp_dir().join("randomly-divide-test-roundtrip.csv");
        let people = vec![
            Person { id: 1, name: "Amal".into(), school: "Southlands".into(), group: 3 },
            Person { id: 2, name: "Nimal".into(), school: "Convent".into(), group: 10 },
        ];
        save(&path, &people).unwrap();
        let loaded = load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(loaded.len(), people.len());
        for (a, b) in loaded.iter().zip(people.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.name, b.name);
            assert_eq!(a.school, b.school);
            assert_eq!(a.group, b.group);
        }
    }
}
