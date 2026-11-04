// libpg-query, Postgres's own parser compiled to WebAssembly, shared by the
// syntax checker and the statement splitter so the module loads once.

export type PgParser = typeof import("libpg-query");

// Loaded on first use so the 1.7 MB parser stays out of app startup.
let pending: Promise<PgParser> | null = null;
let loaded: PgParser | null = null;

/** The parser, ready for its `*Sync` functions. */
export function loadPgParser(): Promise<PgParser> {
  pending ??= import("libpg-query")
    .then(async (pg) => {
      await pg.loadModule();
      loaded = pg;
      return pg;
    })
    .catch((error: unknown) => {
      pending = null; // Try again next time.
      throw error;
    });
  return pending;
}

/** The parser if it has finished loading, for callers that can't wait. */
export function pgParserIfLoaded(): PgParser | null {
  return loaded;
}
