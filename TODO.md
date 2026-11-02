# SwiftQL TODO

## App

### Startup performance in release mode

- Measure startup in a release build (`bun tauri build`). Dev starts are slower
  because the Vite dev server serves unbundled modules; the loading screen in
  `index.html` covers the wait, but check how long it shows in a real build.

## driver-core

### Server messages and timing

- Postgres sends `NOTICE` / `WARNING` messages (`RAISE NOTICE`, `VACUUM`, ...) that
  users expect to see in the UI. Add a way to surface them, e.g. a
  `notices: Vec<Notice>` field on `ExecOutcome` or a channel on `Connection`.
  - `Notice { severity, message, code: Option<String>, detail, hint }`
- Report execution time per statement (server-side duration, or measured
  from send to first row / completion).

### Multi-statement scripts

- `Connection::execute` returns a single `ExecOutcome`, but users run scripts like
  `SELECT 1; SELECT 2; UPDATE ...`. Decide before the Postgres driver grows
  around it:
  - return `Vec<ExecOutcome>` (simple, but every result set must be ready first)
  - return a stream of outcomes (fits `RowStream`; one result set at a time)
  - split statements on the client and call `execute` once per statement
    (needs a dialect-aware splitter: strings, comments, `$$` bodies)
- Decide what happens when one statement fails midway: stop, or report the
  error and continue.
