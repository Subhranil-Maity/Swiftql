# SwiftQL TODO

## App

### Startup performance in release mode

- Measure startup in a release build (`bun tauri build`). Dev starts are slower
  because the Vite dev server serves unbundled modules; the loading screen in
  `index.html` covers the wait, but check how long it shows in a real build.

### Console tab strip: smooth mouse-wheel scrolling

- Scrolling the overflowing console tab strip with a mouse wheel still jumps
  per notch instead of gliding; trackpad scrolling is smooth. The easing lives
  in `src/features/console/hooks/use-horizontal-wheel-scroll.ts`.

### Console results

- Load more than the first 1000 rows of a result (a "load more" control, or
  fetch on scroll; needs cursor paging, see driver-core).
- Make the row limit a setting (`ROW_LIMIT` in
  `src/features/console/hooks/use-run-query.ts`).
- Edit result cells (needs `ColumnMeta::source_table` and a primary key).
- Show the session's transaction status (idle / in transaction / failed) in the
  workspace; `SessionInfo.txStatus` is already updated after every run.

### Console autocomplete

- Tables and columns come from `IntrospectScope::Catalog`, loaded when the
  console opens and after a run with CREATE / ALTER / DROP. Not covered yet:
  - temp tables (the session's `pg_temp_N` schema is left out),
  - `pg_catalog` / `information_schema` tables,
  - unqualified tables from search-path schemas after the first one,
  - functions, types and sequences,
  - a manual refresh, e.g. after another client changes the schema.

## driver-core

### Server messages and timing

- Done: notices (`Connection::take_notices`, Postgres `RAISE NOTICE` / `WARNING`)
  and per-statement timing (measured by the Tauri `run_statements` command)
  show in the console's Messages tab.

### LISTEN / NOTIFY

- The Postgres connection task ignores notifications (`AsyncMessage::Notification`).
  Add a way to subscribe and show them; `Capabilities::listen_notify` is false.

### Multi-statement scripts (not urgent)

- The console splits scripts client-side (libpg-query) and runs them one
  statement at a time, stopping at the first error. Still wanted at the driver
  level: run a whole script in one round trip and get several result sets back
  (`Capabilities::multiple_result_sets`).
