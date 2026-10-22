# Randomly Divide

A terminal UI for randomly, verifiably-fairly splitting a roster of people
into 10 groups, with optional realtime sync to a [Turso](https://turso.tech)
database.

## Pages

`F1`/`F2`/`F3`/`F4` switch pages. `q` quits (Add Person modal: `Esc` instead,
so `q` can be typed into a name).

- **Entry** — roster list + `a` opens **Add Person**.
- **Groups** — `j`/`k` pick the group half (1-5 / 6-10), `1`-`5` pick a group
  in that half, `Shift+1`..`Shift+9` select a member row, `o`/`p` jump the
  member list to top/bottom, `d`/`Del`/`Backspace` removes the selected
  person.
- **Stats** — bar charts of people per group and per school.
- **Settings** — Turso `DATABASE_URL`/auth token, `Ctrl+S` save & connect,
  `Ctrl+R` rotate the secret (clears the token field for re-entry).

## Add Person modal

`a` on the Entry page opens a floating window: name, class (free text, e.g.
`8-B`), school, and gender on the left; a hashed identicon avatar preview on
the right. `Ctrl+R` rerolls the avatar, `Esc` cancels, `Enter` on the last
field submits — the person is assigned to a uniformly random group via a
ChaCha12 CSPRNG and saved.

## Storage

Every change is written to `people.csv` immediately. If a Turso database is
configured (see below), writes are also mirrored there, and the two stay
synced while the app runs.

## Turso setup

Open **Settings** (`F4`), fill in the database URL and auth token, `Ctrl+S`
to save and connect. This writes `.env.production` (gitignored — never
commit it) with `TURSO_DATABASE_URL` / `TURSO_AUTH_TOKEN`, which is read
automatically on the next launch.

### How the realtime sync actually works

Turso's [Change Data Capture](https://docs.turso.tech/tursodb/cdc) is a
**queryable table** (`turso_cdc`), not a push/webhook feed — there is no
server-initiated notification for remote libSQL clients. This app enables
CDC on connect and polls `turso_cdc` for new rows every few seconds over the
already-open connection — much cheaper than re-fetching the whole table, but
it is still a (very infrequent) poll, not true push. If your database
doesn't support the CDC pragma (older libSQL-engine databases), the app
falls back automatically to a deduplicated full-table refresh on the same
interval.
