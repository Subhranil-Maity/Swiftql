import { useDeferredValue, useMemo, useState } from "react";

import { groupTables } from "@/features/tables/model/filter";
import type { SchemaModel } from "@/stores/schema-model";

/** The search box's text and the schema groups it leaves. */
export function useTableFilter(model: SchemaModel | undefined) {
  const [query, setQuery] = useState("");
  // Typing stays responsive in databases with thousands of tables.
  const deferred = useDeferredValue(query);

  const groups = useMemo(() => (model ? groupTables(model, deferred) : []), [model, deferred]);
  const counts = useMemo(() => {
    const schemas = model?.schemas.length ?? 0;
    const tables = model?.schemas.reduce((n, s) => n + (s.tables?.length ?? 0), 0) ?? 0;
    const matches = groups.reduce((n, g) => n + g.entries.length, 0);
    return { schemas, tables, matches };
  }, [model, groups]);

  return { query, setQuery, searching: deferred.trim() !== "", groups, counts };
}
