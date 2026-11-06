# SwiftQL

A fast desktop SQL client for PostgreSQL, built with Tauri 2, React and Rust.

## Features

- **SQL console**: tabs, CodeMirror editor with Postgres syntax checking and
  schema autocomplete, statement-by-statement runs with timing and server
  notices.
- **Results**: a virtualized grid that pages through results of any size (up
  to a configurable cap held in memory), sorting over the whole result, copy and
  export (CSV, JSON, ...).
- **Tables & Graph**: browse tables, views and their details; an ER diagram of
  foreign keys with PNG/SVG export.
- **Connections**: TLS (disable / prefer / require / verify-ca / verify-full,
  client certificates), SSH tunnels (password, key, agent, hosts from
  `~/.ssh/config`, ProxyJump), passwords stored in the OS keyring, `.pgpass`.
- **LISTEN / NOTIFY**: subscribe to channels and watch notifications live.
- **Settings**: appearance, editor, query & results, connections & security.

## Development

Requirements: [Rust](https://rustup.rs) (stable), [Bun](https://bun.sh), and
the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```sh
bun install
bun tauri dev        # run the app
bun run build        # type-check and build the frontend
bun run gen:bindings # regenerate src/bindings.ts after changing Rust types
bun tauri build      # release build and installers
```

### Tests

```sh
cargo test --workspace
```

The Postgres integration tests are `#[ignore]`d and need a server:

```sh
SWIFTQL_PG_URL=postgres://user:pass@localhost/db cargo test -p driver-postgres -- --ignored
```

TLS tests also read `SWIFTQL_PG_TLS_URL` (a server with `ssl=on`).

## Layout

- `src/` React frontend (`src/lib/ipc.ts` is the only bridge to the backend)
- `src-tauri/` Tauri app: commands, sessions, result buffers
- `crates/driver-core` the database-agnostic driver API
- `crates/driver-postgres` the PostgreSQL driver
- `crates/ssh-tunnel` SSH tunnels used by drivers
- `crates/config-store` settings, saved connections, keyring secrets

## License

MIT, see [LICENSE](LICENSE).
