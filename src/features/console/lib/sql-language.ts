import { PostgreSQL, type SQLDialect, StandardSQL, sql } from "@codemirror/lang-sql";
import type { Extension } from "@uiw/react-codemirror";

const DIALECTS: Record<string, SQLDialect> = {
  postgres: PostgreSQL,
};

/** SQL highlighting and keyword completion in the driver's dialect. */
export function sqlLanguage(driverId: string): Extension {
  return sql({
    dialect: DIALECTS[driverId] ?? StandardSQL,
    // Keyword completions in upper case (SELECT, FROM, ...).
    // TODO: make keyword case a user setting once the editor has settings
    upperCaseKeywords: true,
    // TODO: pass the connection's schema so tables and columns complete
  });
}
