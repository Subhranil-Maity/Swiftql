import { KeyRoundIcon, LinkIcon } from "lucide-react";

import type { ColumnRole } from "@/features/tables/model/table-entry";
import { cn } from "@/lib/utils";

/**
 * Primary key (amber key) or foreign key (slate-blue link). Always the same
 * width, empty or not, so it works as a gutter that lines keys up.
 * A column in both shows the key; its name carries the foreign-key tint.
 */
export function KeyGlyph({ role, className }: { role: ColumnRole; className?: string }) {
  return (
    <span className={cn("flex w-3.5 shrink-0 justify-center", className)}>
      {role.primary ? (
        <KeyRoundIcon role="img" aria-label="Primary key" className="size-3 text-key" />
      ) : role.foreign ? (
        <LinkIcon role="img" aria-label="Foreign key" className="size-3 text-ref" />
      ) : null}
    </span>
  );
}
