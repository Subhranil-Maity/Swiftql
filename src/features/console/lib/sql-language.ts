import { PostgreSQL, type SQLDialect, StandardSQL, sql } from "@codemirror/lang-sql";
import type { Extension } from "@uiw/react-codemirror";

import { catalogNamespace, defaultSchema, tableColumns } from "@/features/console/lib/sql-completion";
import type { Catalog } from "@/stores/schema-catalog";

const DIALECTS: Record<string, SQLDialect> = {
  postgres: PostgreSQL,
};

/**
 * SQL highlighting and completion in the driver's dialect: keywords, plus the
 * connected database's schemas, tables and columns once `catalog` has loaded.
 */
export function sqlLanguage(
  driverId: string,
  catalog: Catalog | undefined,
  upperCaseKeywords = true,
): Extension {
  const support = sql({
    dialect: DIALECTS[driverId] ?? StandardSQL,
    // Keyword completions in upper (SELECT) or lower (select) case; Settings → Editor.
    upperCaseKeywords,
    schema: catalog ? catalogNamespace(catalog) : undefined,
    defaultSchema: catalog ? defaultSchema(catalog) : undefined,
  });
  if (!catalog) return support;
  return [support, support.language.data.of({ autocomplete: tableColumns(() => catalog) })];
}
