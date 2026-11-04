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
- Sorting is client-side over the fetched rows only (`model/row-sort.ts`); the
  footer says so when a result is truncated. Server-side sort would re-run the
  query with ORDER BY.
- Export writes the bytes as a JSON number array over IPC
  (`src-tauri/src/export.rs`); large exports would be faster with a raw body.

### Tables view

- "Open data" fetches `SELECT * … LIMIT 500` (`OPEN_DATA_LIMIT` in
  `features/tables/model/generate-sql.ts`). Page through more, and sort on the
  server instead of in the grid.
- A per-table data tab with editing (needs a primary key; see "Edit result
  cells" above).
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

### Console autocomplete

- Tables and columns come from `IntrospectScope::Catalog`, loaded when the
  console opens and after a run with CREATE / ALTER / DROP. Not covered yet:
  - temp tables (the session's `pg_temp_N` schema is left out),
  - `pg_catalog` / `information_schema` tables,
  - unqualified tables from search-path schemas after the first one,
  - functions, types and sequences,
  - a refresh from the console itself after another client changes the schema
    (the Tables view's Refresh button reloads autocomplete too).

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
