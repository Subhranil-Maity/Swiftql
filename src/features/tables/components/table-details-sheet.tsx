import type { ReactNode } from "react";
import { ArrowRightIcon } from "lucide-react";

import { ColumnRow } from "@/components/schema/column-row";
import { TableKindIcon, tableKindLabel } from "@/components/schema/table-kind-icon";
import { TableStats } from "@/components/schema/table-stats";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useTableActions } from "@/features/tables/hooks/use-table-actions";
import { columnRole, entryKey, type TableEntry } from "@/features/tables/model/table-entry";
import type { FkAction, ForeignKey } from "@/lib/ipc";
import type { SchemaModel } from "@/stores/schema-model";

const ACTION_LABELS: Record<FkAction, string> = {
  noAction: "no action",
  restrict: "restrict",
  cascade: "cascade",
  setNull: "set null",
  setDefault: "set default",
};

/** Foreign keys in other tables that point at `entry`. */
function incomingKeys(model: SchemaModel, entry: TableEntry) {
  const found: { from: TableEntry; fk: ForeignKey }[] = [];
  for (const schema of model.schemas) {
    for (const table of schema.tables ?? []) {
      for (const fk of table.foreignKeys ?? []) {
        if (fk.refSchema === entry.schema && fk.refTable === entry.table.name) {
          found.push({ from: { key: entryKey(schema.name, table.name), schema: schema.name, table }, fk });
        }
      }
    }
  }
  return found;
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-1.5">
      <h3 className="text-xs font-medium text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

/** A link to another table; the sheet then shows that one. */
function TableLink({ schema, table, onNavigate }: { schema: string; table: string; onNavigate: (key: string) => void }) {
  return (
    <button
      type="button"
      onClick={() => onNavigate(entryKey(schema, table))}
      className="rounded-sm font-medium text-foreground underline-offset-2 outline-none hover:underline focus-visible:ring-2 focus-visible:ring-ring/50"
    >
      {schema}.{table}
    </button>
  );
}

const cols = (names: string[]) => <span className="font-mono text-xs">({names.join(", ")})</span>;

/** Everything known about one table: columns, keys, references and indexes. */
export function TableDetailsSheet({
  entry,
  model,
  onNavigate,
  onClose,
  showInGraph = true,
}: {
  /** Null closes the sheet. */
  entry: TableEntry | null;
  model: SchemaModel | undefined;
  /** A foreign key's table was clicked. */
  onNavigate: (key: string) => void;
  onClose: () => void;
  showInGraph?: boolean;
}) {
  const actions = useTableActions();
  const table = entry?.table;
  const incoming = entry && model ? incomingKeys(model, entry) : [];

  return (
    <Sheet open={entry !== null} onOpenChange={(open) => !open && onClose()}>
      <SheetContent side="right" className="w-[440px] gap-0 sm:max-w-[440px]">
        {entry && table && (
          <>
            <SheetHeader className="border-b">
              <SheetTitle className="flex items-center gap-2 pr-8">
                <TableKindIcon kind={table.kind} />
                <span className="truncate">{table.name}</span>
              </SheetTitle>
              <SheetDescription>
                {tableKindLabel(table.kind)} in {entry.schema}
              </SheetDescription>
              {table.comment && <p className="text-sm">{table.comment}</p>}
              <TableStats table={table} />
            </SheetHeader>
            <ScrollArea className="min-h-0 flex-1">
              <div className="flex flex-col gap-5 p-4">
                <Section title={`Columns · ${table.columns?.length ?? 0}`}>
                  <div>
                    {(table.columns ?? []).map((column) => (
                      <div key={column.name} className="border-b border-border/60 py-0.5 last:border-0">
                        <ColumnRow column={column} role={columnRole(table, column)} />
                        {(column.default || column.comment) && (
                          <div className="pb-1 pl-5 text-xs text-muted-foreground">
                            {column.comment && <p>{column.comment}</p>}
                            {column.default && (
                              <p className="truncate font-mono text-[11px]" title={column.default}>
                                default {column.default}
                              </p>
                            )}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </Section>

                {table.primaryKey && table.primaryKey.length > 0 && (
                  <Section title="Primary key">{cols(table.primaryKey)}</Section>
                )}

                {table.foreignKeys && table.foreignKeys.length > 0 && (
                  <Section title="References">
                    <ul className="flex flex-col gap-2">
                      {table.foreignKeys.map((fk) => (
                        <li key={fk.name} className="flex flex-col gap-0.5">
                          <span className="flex flex-wrap items-center gap-1.5">
                            {cols(fk.columns)}
                            <ArrowRightIcon aria-label="references" className="size-3 text-ref" />
                            <TableLink schema={fk.refSchema} table={fk.refTable} onNavigate={onNavigate} />
                            {cols(fk.refColumns)}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {fk.name} · on delete {ACTION_LABELS[fk.onDelete]} · on update{" "}
                            {ACTION_LABELS[fk.onUpdate]}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </Section>
                )}

                {incoming.length > 0 && (
                  <Section title="Referenced by">
                    <ul className="flex flex-col gap-1">
                      {incoming.map(({ from, fk }) => (
                        <li key={`${from.key}:${fk.name}`} className="flex flex-wrap items-center gap-1.5">
                          <TableLink schema={from.schema} table={from.table.name} onNavigate={onNavigate} />
                          {cols(fk.columns)}
                        </li>
                      ))}
                    </ul>
                  </Section>
                )}

                {table.indexes && table.indexes.length > 0 && (
                  <Section title={`Indexes · ${table.indexes.length}`}>
                    <ul className="flex flex-col gap-2">
                      {table.indexes.map((index) => (
                        <li key={index.name} className="flex flex-col gap-0.5">
                          <span className="flex items-center gap-2">
                            <span className="truncate font-medium">{index.name}</span>
                            <span className="text-xs text-muted-foreground">
                              {index.primary ? "primary" : index.unique ? "unique" : ""}
                            </span>
                          </span>
                          <code className="font-mono text-[11px] break-all text-muted-foreground">
                            {index.definition}
                          </code>
                        </li>
                      ))}
                    </ul>
                  </Section>
                )}
              </div>
            </ScrollArea>
            <SheetFooter className="flex-row justify-end border-t">
              {showInGraph && (
                <Button variant="ghost" size="sm" onClick={() => actions.showInGraph(entry)}>
                  Show in graph
                </Button>
              )}
              <Button size="sm" onClick={() => actions.openData(entry)}>
                Open data
              </Button>
            </SheetFooter>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}
