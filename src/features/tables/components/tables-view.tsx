import { type KeyboardEvent, useMemo, useRef, useState } from "react";

import { ScrollArea } from "@/components/ui/scroll-area";
import { ModelLoadState } from "@/features/tables/components/model-load-state";
import { SchemaGroup } from "@/features/tables/components/schema-group";
import { TableContextMenu } from "@/features/tables/components/table-context-menu";
import { TableDetailsSheet } from "@/features/tables/components/table-details-sheet";
import { TableHoverCard } from "@/features/tables/components/table-hover-card";
import { TableListItem } from "@/features/tables/components/table-list-item";
import { TablesEmpty } from "@/features/tables/components/tables-empty";
import { TablesToolbar } from "@/features/tables/components/tables-toolbar";
import { type TreeItem, useListKeyboard } from "@/features/tables/hooks/use-list-keyboard";
import { useOpenTableData } from "@/features/tables/hooks/use-open-table-data";
import { useSchemaModel } from "@/features/tables/hooks/use-schema-model";
import { useTableFilter } from "@/features/tables/hooks/use-table-filter";
import { type TableEntry, tableEntries } from "@/features/tables/model/table-entry";
import { useWorkspace } from "@/features/workspace/model/workspace-context";
import type { SchemaModel } from "@/stores/schema-model";

const schemaId = (schema: string) => `schema:${schema}`;

/** With many schemas, only the search-path ones start open. */
function initiallyCollapsed(model: SchemaModel): Set<string> {
  if (model.schemas.length <= 3) return new Set();
  return new Set(model.schemas.map((s) => s.name).filter((name) => !model.searchPath.includes(name)));
}

/** Every table, grouped by schema: hover for details, double-click or Enter to open its data. */
export function TablesView() {
  const { sessionId } = useWorkspace();
  const { model, status, refresh } = useSchemaModel(sessionId);
  const { query, setQuery, searching, groups, counts } = useTableFilter(model);
  const { openTableData } = useOpenTableData();
  const searchRef = useRef<HTMLInputElement>(null);

  // Null until the user toggles a schema; the default follows the model.
  const [collapsed, setCollapsed] = useState<Set<string> | null>(null);
  const collapsedNow = useMemo(
    () => collapsed ?? (model ? initiallyCollapsed(model) : new Set<string>()),
    [collapsed, model],
  );
  const isOpen = (schema: string) => searching || !collapsedNow.has(schema);
  const setOpen = (schema: string, open: boolean) => {
    const next = new Set(collapsedNow);
    if (open) next.delete(schema);
    else next.add(schema);
    setCollapsed(next);
  };

  const byKey = useMemo(
    () => new Map(model ? tableEntries(model).map((e) => [e.key, e] as const) : []),
    [model],
  );
  const [detailsKey, setDetailsKey] = useState<string | null>(null);
  const details = detailsKey ? (byKey.get(detailsKey) ?? null) : null;

  const items = useMemo<TreeItem[]>(
    () =>
      groups.flatMap((g) => {
        const open = searching || !collapsedNow.has(g.schema);
        const head: TreeItem = { id: schemaId(g.schema), parentId: null, expandable: true, expanded: open };
        if (!open) return [head];
        return [
          head,
          ...g.entries.map((e) => ({ id: e.key, parentId: head.id, expandable: false, expanded: false })),
        ];
      }),
    [groups, searching, collapsedNow],
  );
  const open = (entry: TableEntry) => void openTableData(entry.schema, entry.table.name);
  const tree = useListKeyboard({
    items,
    onActivate: (id) => {
      const entry = byKey.get(id);
      if (entry) open(entry);
    },
    onToggle: (id, expanded) => {
      const group = groups.find((g) => schemaId(g.schema) === id);
      if (group) setOpen(group.schema, expanded);
    },
  });

  // Ctrl/Cmd+F or "/" jumps to the filter from anywhere in the view.
  const onKeyDown = (event: KeyboardEvent) => {
    const isFind = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f";
    const isSlash = event.key === "/" && !(event.target instanceof HTMLInputElement);
    if (isFind || isSlash) {
      event.preventDefault();
      searchRef.current?.focus();
      searchRef.current?.select();
    } else if (event.key === "ArrowDown" && event.target === searchRef.current && items[0]) {
      event.preventDefault();
      tree.focus(items[0].id);
    }
  };

  let body;
  if (!model) {
    body = <ModelLoadState error={status.loading ? null : status.error} onRetry={refresh} />;
  } else if (counts.tables === 0 || groups.length === 0) {
    body = <TablesEmpty query={query} total={counts.tables} onClear={() => setQuery("")} />;
  } else {
    body = (
      <ScrollArea className="min-h-0 flex-1">
        <TableContextMenu
          resolve={(target) => {
            const key = target.closest("[data-table-key]")?.getAttribute("data-table-key");
            return key ? byKey.get(key) : undefined;
          }}
          onViewDetails={(entry) => setDetailsKey(entry.key)}
        >
          <div
            ref={tree.containerRef}
            role="tree"
            aria-label="Tables"
            onKeyDown={tree.onKeyDown}
            className="flex max-w-[720px] flex-col px-2 py-2"
          >
            {groups.map((group) => (
              <SchemaGroup
                key={group.schema}
                schema={group.schema}
                count={group.entries.length}
                open={isOpen(group.schema)}
                onOpenChange={(next) => setOpen(group.schema, next)}
                treeProps={tree.itemProps(schemaId(group.schema))}
              >
                {group.entries.map((entry) => (
                  <TableListItem
                    key={entry.key}
                    entry={entry}
                    onOpen={() => open(entry)}
                    treeProps={tree.itemProps(entry.key)}
                  />
                ))}
              </SchemaGroup>
            ))}
          </div>
        </TableContextMenu>
      </ScrollArea>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col" onKeyDown={onKeyDown}>
      <TablesToolbar
        query={query}
        onQueryChange={setQuery}
        searchRef={searchRef}
        counts={counts}
        searching={searching}
        loading={status.loading}
        error={model ? status.error : null}
        onRefresh={refresh}
      />
      {body}
      {/* TODO: virtualize the list for databases with many thousands of tables */}
      <TableHoverCard find={(key) => byKey.get(key)} />
      <TableDetailsSheet
        entry={details}
        model={model}
        onNavigate={setDetailsKey}
        onClose={() => setDetailsKey(null)}
      />
    </div>
  );
}
