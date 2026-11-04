import {
  EyeIcon,
  GlobeIcon,
  LayersIcon,
  type LucideIcon,
  TableCellsSplitIcon,
  TableIcon,
} from "lucide-react";

import type { TableKind } from "@/lib/ipc";
import { cn } from "@/lib/utils";

const KINDS: Record<TableKind, { icon: LucideIcon; label: string }> = {
  table: { icon: TableIcon, label: "table" },
  view: { icon: EyeIcon, label: "view" },
  materializedView: { icon: LayersIcon, label: "materialized view" },
  foreignTable: { icon: GlobeIcon, label: "foreign table" },
  partitionedTable: { icon: TableCellsSplitIcon, label: "partitioned table" },
};

/** "table", "materialized view", ... in lower case, for running text. */
export function tableKindLabel(kind: TableKind): string {
  return KINDS[kind].label;
}

/** The kind's icon, labelled for screen readers. */
export function TableKindIcon({ kind, className }: { kind: TableKind; className?: string }) {
  const { icon: Icon, label } = KINDS[kind];
  return (
    <Icon
      role="img"
      aria-label={label}
      className={cn("size-3.5 shrink-0 text-muted-foreground", className)}
    />
  );
}
