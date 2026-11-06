# SwiftQL TODO

## v1 (in progress)

Implemented; waiting for manual verification. Items still open are unticked
and say what they need.

### 1. Release and security basics

- [x] Content Security Policy in `src-tauri/tauri.conf.json` (it is `null` now):
      self only, `'wasm-unsafe-eval'` for libpg-query, inline styles for
      CodeMirror / glide, `data:`/`blob:` images for driver SVGs and PNG export.
- [x] Capabilities: keep them minimal, add `dialog:allow-open` for file pickers.
- [x] Metadata: product name and window title "SwiftQL", window 1280×800
      (min 960×600), `Cargo.toml` description and authors, real README.
- [x] App icon: italic SQL with speed dashes (`design/logo/swiftql-logo.svg`;
      `bun tauri icon design/logo/swiftql-logo.svg` regenerates `src-tauri/icons`).
- [x] Version 1.0.0 in `package.json`, `tauri.conf.json`, every `Cargo.toml`.
- [ ] LICENSE: an MIT file is in place as a placeholder; confirm the license.
- [x] Remove unused deps: `marked`, `react-responsive-carousel`, `lodash`.
- [x] Exports: the backend opens the save dialog and writes the file
      (`save_export`), taking a raw IPC body. Remove `write_export_file`, which
      writes to any path the frontend sends (`src-tauri/src/export.rs`).
- [x] Make sure no secret reaches error messages or logs; log level `info` in
      release builds.
- [x] CI: clippy, `cargo test --workspace`, `bun run build`, PG integration
      tests against a service container.
- [ ] CI: a second Postgres service with `ssl=on` for the TLS tests.
- [x] Release workflow (tauri-action, Windows/macOS/Linux), draft releases.
- [ ] Auto-update (`tauri-plugin-updater`) and code signing: need the updater
      signing key and OS certificates (`.github/workflows/release.yml`).
- [ ] Measure startup in a release build (`bun tauri build`). Dev starts are
      slower because Vite serves unbundled modules; the loading screen in
      `index.html` covers the wait.

### 2. Results past the first 1000 rows (most important)

- [x] Backend result buffer (`src-tauri/src/results/`): rows are read into a
      per-result buffer until the end, the `query.maxRowsHeld` /
      `query.maxResultMb` cap, or Stop. Rows already read are kept on Stop.
      This also bounds the memory pile-up noted in
      `driver-postgres/src/stream.rs`.
- [x] `fetch_rows(result_id, offset, limit)`; the grid pages through the whole
      result as it scrolls, while it is still loading.
- [x] `sort_result`: sort the whole buffer in the backend (type-aware, NULLs
      last) instead of the fetched rows only (`model/row-sort.ts`).
- [x] `export_result` / `copy_result` work on the whole result.
- [x] Release buffers on re-run, tab close and disconnect; LRU per session.
- [x] Remove `ROW_LIMIT` (`use-run-query.ts`) and `OPEN_DATA_LIMIT`
      (`features/tables/model/generate-sql.ts`); the footer shows progress,
      "limit reached" or "stopped after N rows".

### 3. Bugs

- [x] The last ~500ms of console typing is lost on quit: flush pending saves
      (console tabs, graph layout) on close (`stores/console-tabs.ts`).
- [x] Save failures are only logged (`console-tabs.ts`, `graph-layout.ts`):
      show a toast with Retry.
- [x] `E''` string with a backslash-escaped quote ends the string early
      (`driver-postgres/src/sql.rs`).
- [x] A cancel that reaches the server just after a statement finished can hit
      the next one (`src-tauri/src/query.rs`): only cancel while a statement is
      in flight.
- [x] COMMIT / ROLLBACK inside `CALL` isn't seen by transaction tracking
      (`driver-postgres/src/tx.rs`): read the real status after CALL.
- [x] NaN and ±Infinity arrive as null (`console/model/cell-format.ts`): send
      them as text.
- [x] Theme lives in localStorage (`stores/theme.ts`, `src-tauri/src/lib.rs`):
      store it in `Settings.general.theme`.
- [x] Error checks rely on English server messages (`connection.rs`
      multi-statement, `error.rs` 57014 timeout vs cancel).
- [x] Connect timeout / keepalive / query timeout settings are ignored
      (`driver-postgres/src/config.rs`).
- [x] Test connection can't ask for a one-off password (`src-tauri/src/connect.rs`).
- [x] Native file pickers for file fields (`target-fields.tsx`,
      `extra-option-field.tsx`).
- [x] `~/.pgpass` / `PGPASSWORD` not supported (`config.rs`).
- [x] URLs with several hosts are rejected (`config.rs`).
- [x] Show the session's transaction status (idle / in transaction / failed)
      with Commit / Rollback; `SessionInfo.txStatus` is already updated.

### 4. TLS (rustls)

- [x] `driver-postgres/src/tls.rs` with rustls: disable / prefer / require /
      verify-ca (chain only) / verify-full; CA file or OS + webpki roots.
- [x] Client certificate auth (PEM cert + key).
- [x] `sslmode`, `sslrootcert`, `sslcert`, `sslkey` in URLs.
- [x] Re-enable SSL modes and add the CA field in the connection form
      (`tls-fields.tsx`, `form-state.ts`).
- [ ] Integration tests per mode (`SWIFTQL_PG_TLS_URL`); verified by hand for now.

### 5. SSH tunnels (russh)

- [x] New crate `crates/ssh-tunnel`: direct-tcpip channels (no local port),
      password / key + passphrase / agent (OpenSSH pipe, Pageant,
      `SSH_AUTH_SOCK`), keepalive, ProxyJump chains.
- [x] Host keys: read `~/.ssh/known_hosts` and an app-owned known_hosts;
      policy ask / acceptNew / strict; a changed key is always refused.
- [x] Use hosts from `~/.ssh/config` (Host, HostName, User, Port, IdentityFile,
      ProxyJump, Include), resolved at connect time.
- [x] Password / passphrase prompt variants (`config-store/src/connections.rs`).
- [x] Cancel through the tunnel (`cancel_query_raw`).
- [x] SSH section in the connection form, host-key confirm dialog.

### 6. LISTEN / NOTIFY

- [x] driver-core: `Notification`, `subscribe_notifications`, `listen`,
      `unlisten`, `notify`; `Capabilities::listen_notify`.
- [x] driver-postgres: forward `AsyncMessage::Notification`
      (`connection.rs`) into a broadcast channel.
- [x] Tauri: dedicated listener connection, `listen_channel`,
      `unlisten_channel`, `send_notify`, `NotificationReceived` event.
- [x] Notifications workspace view (channels, live log, payload view, send
      form, unread badge). LISTEN run in the console shows there too.
- [ ] Also list console notifications in the console's Messages tab.

### 7. Settings

- [x] `SETTINGS_VERSION` 2 with a migration from 1
      (`config-store/src/settings.rs`).
- [x] Appearance: theme, UI font size, density, NULL / boolean / timestamp
      display, number grouping.
- [x] Editor: font, size, tab size, wrap, keyword case, line numbers,
      autocomplete, lint, active line.
- [x] Query & results: page size, max rows / MB held, query timeout, confirm
      destructive statements, auto / manual transactions.
- [x] Connections & security: connect timeout, keepalive, check on startup,
      default SSL mode, SSH host-key policy, save passwords by default,
      notification buffer, keyring status, clear trusted SSH hosts.
- [x] Settings page with section nav, search, reset section / all, `Ctrl+,`.

## After v1

### Console

- Edit result cells / a per-table data tab with editing (needs
  `ColumnMeta::source_table` and a primary key).
- Query parameters (`$1`, `$2`, ...; `driver-postgres/src/connection.rs`).
- Query history view (`app-sidebar.tsx`).
- Smooth mouse-wheel scrolling of the tab strip (it jumps per notch; trackpad
  is fine; `console/hooks/use-horizontal-wheel-scroll.ts`).

### Console autocomplete

- Tables and columns come from `IntrospectScope::Catalog`, loaded when the
  console opens and after a run with CREATE / ALTER / DROP. Not covered yet:
  - temp tables (the session's `pg_temp_N` schema is left out),
  - `pg_catalog` / `information_schema` tables,
  - unqualified tables from search-path schemas after the first one,
  - functions, types and sequences,
  - a refresh from the console itself after another client changes the schema
    (the Tables view's Refresh button reloads autocomplete too),
  - quoting reserved words used as names,
  - a hint that completions may be out of date.

### Tables view

- Partitions are hidden everywhere (the parent shows the summed size and row
  estimate of its direct partitions). Show them, and nested partitions.
- Indexes are only listed in the details sheet; INCLUDE columns are mixed into
  the key columns (`indnkeyatts` is PG 11+).
- Virtualize the list for databases with many thousands of tables.

### Graph view

- Edges are foreign keys only. Add view-dependency edges (`pg_depend` /
  `pg_rewrite`) so views show what they read from.
- Partitions and indexes aren't drawn.
- New tables are placed by a fresh elk run, which can land them on top of
  cards the user dragged; place them in free space instead.
- PNG export of very large diagrams lowers the pixel ratio to stay under the
  browser's canvas limit; offer a tiled export or SVG only past that.

### driver-core

- Multi-statement scripts: run a whole script in one round trip and get several
  result sets back (`Capabilities::multiple_result_sets`). The console splits
  scripts client-side (libpg-query) for now.
- Admin API: roles and grants, active sessions, kill session, table stats.
- Bulk import / export (Postgres: COPY).
- Types: numeric typmod, column nullability, composite / multirange types,
  `money` fractional digits from `lc_monetary`.

## Done

- Server messages and timing: notices (`Connection::take_notices`, Postgres
  `RAISE NOTICE` / `WARNING`) and per-statement timing (measured by the Tauri
  `run_statements` command) show in the console's Messages tab.
