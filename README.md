<p align="center">
  <img src="design/logo/swiftql-logo.svg" width="112" alt="SwiftQL logo">
</p>

<h1 align="center">SwiftQL</h1>

<p align="center">
  A fast, native desktop client for PostgreSQL and SQLite.<br>
  Write SQL, scroll through millions of rows, and explore your schema, from an 8&nbsp;MB installer.
</p>

<p align="center">
  <img src="docs/screenshots/console.png" alt="The SQL console with a query and its results" width="900">
</p>

## Why SwiftQL

- **Fast to open, light on memory.** Built with Rust and Tauri, so it uses the
  system webview instead of shipping a whole browser.
- **Results of any size.** Rows are kept in the backend and the grid loads them
  as you scroll. A query returning two million rows doesn't freeze the app.
- **Safe by default.** Passwords go in your OS keyring, connections can use
  verified TLS or an SSH tunnel, and destructive statements ask before they run.

## Features

### SQL console

- Tabs that are saved between sessions.
- A CodeMirror editor with highlighting for each database's SQL dialect,
  Postgres syntax checking as you type, and autocomplete for your tables,
  columns and functions.
- Run the statement under the cursor (`Ctrl+Enter`) or the whole script
  (`Ctrl+Shift+Enter`). Each statement reports its timing, row count and any
  server notices.
- Stop a running query at any time; the rows already read are kept.
- Auto-commit or manual transactions, with Commit and Rollback in the header.
- Confirmation before `DROP`, `TRUNCATE`, and `DELETE` or `UPDATE` without a
  `WHERE` clause.

### Results

- A virtualized grid that pages through the result while it is still loading.
- Sorting over the whole result, not just what is on screen, with NULLs last
  and exact numeric ordering.
- Copy or export the whole result or a selection as CSV, TSV, Markdown or
  JSON.
- Postgres-aware display of NULL, booleans, timestamps, intervals, arrays,
  ranges, JSON, `NaN` and `Infinity`.

### Tables and Graph

- Browse schemas, tables and views with their columns, keys, indexes and row
  estimates, and open any table's data in one click.
- An ER diagram of your foreign keys that you can arrange and export as PNG or
  SVG.

<p align="center">
  <img src="docs/screenshots/tables.png" alt="The Tables view" width="440">
  <img src="docs/screenshots/graph.png" alt="The ER diagram" width="440">
</p>

### Connections

- Connect with a form or paste a `postgres://` URL, including URLs with
  several hosts.
- **TLS** in every libpq mode (`disable`, `prefer`, `require`, `verify-ca`,
  `verify-full`), a custom CA, and client certificates.
- **SSH tunnels** with password, key file or ssh-agent (including Pageant on
  Windows). Pick a host straight from your `~/.ssh/config`, ProxyJump
  included. Unknown hosts show their fingerprint for you to confirm, and a
  changed host key is always refused.
- Passwords saved in the OS keyring, asked for every time, or read from
  `.pgpass` and `PGPASSWORD`.

<p align="center">
  <img src="docs/screenshots/connection.png" alt="The connection form with an SSH tunnel" width="700">
</p>

### SQLite

- Pick SQLite when you add a connection, then open any `.db` / `.sqlite`
  file, or create a new empty database from the connection form. SQLite is built into the app, so there's nothing to install.
- Open a file read-only to look without changing anything.
- The console, results grid, Tables view and ER diagram all work as they do
  for Postgres; attached databases show up as schemas.
- Stop and the query timeout interrupt long queries, and foreign keys are
  enforced.

<p align="center">
  <img src="docs/screenshots/connection-driver.png" alt="Choosing PostgreSQL or SQLite for a new connection" width="700">
</p>

### LISTEN / NOTIFY

- Listen on channels and watch notifications arrive live, with JSON payloads
  pretty-printed.
- Send a `NOTIFY` from the app. Listening uses its own connection, so a long
  query in the console never delays notifications.

### Settings

Light and dark themes, UI font size and density, editor font and keyword case,
how NULLs, booleans and timestamps are shown, row and memory limits for
results, query timeout, default SSL mode and SSH host-key policy. Search
settings, reset one section or all of them, and open them anytime with
`Ctrl+,`.

## Install

Download the latest installer from the
[Releases](../../releases) page:

| Platform | Files |
| --- | --- |
| Windows 10/11 (x64) | `.msi` or `-setup.exe` |
| macOS (Intel and Apple Silicon) | `.dmg` |
| Linux (x64) | `.deb`, `.rpm` or `.AppImage` |

## Keyboard shortcuts

On macOS, use `⌘` instead of `Ctrl`.

| Action | Shortcut |
| --- | --- |
| Run the statement under the cursor | `Ctrl+Enter` |
| Run the whole script | `Ctrl+Shift+Enter` |
| New tab | `Ctrl+T` |
| Close tab | `Ctrl+W` |
| Find in the editor | `Ctrl+F` |
| Open Settings | `Ctrl+,` |

## Build from source

You need [Rust](https://rustup.rs) (stable), [Bun](https://bun.sh), and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```sh
bun install
bun tauri dev        # run the app in development
bun tauri build      # release build and installers
```

Other useful commands:

```sh
bun run build        # type-check and build the frontend
bun run gen:bindings # regenerate src/bindings.ts after changing Rust types
cargo test --workspace
```

The SQLite tests run against temporary files and need nothing else. The
Postgres integration tests are `#[ignore]`d and need a server:

```sh
SWIFTQL_PG_URL=postgres://user:pass@localhost/db cargo test -p driver-postgres -- --ignored
```

TLS tests also read `SWIFTQL_PG_TLS_URL`, which must point to a server with
`ssl=on`.

## Project layout

| Path | What it holds |
| --- | --- |
| `src/` | React frontend. `src/lib/ipc.ts` is its only bridge to the backend. |
| `src-tauri/` | The Tauri app: commands, sessions and result buffers. |
| `crates/driver-core` | The database-agnostic driver API. |
| `crates/driver-postgres` | The PostgreSQL driver. |
| `crates/driver-sqlite` | The SQLite driver (rusqlite, SQLite built in). |
| `crates/ssh-tunnel` | SSH tunnels used by the drivers. |
| `crates/config-store` | Settings, saved connections and keyring secrets. |

## License

MIT, see [LICENSE](LICENSE).
