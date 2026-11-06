import type { ReactNode } from "react";

/** One setting: label and description on the left, its control on the right. */
export function SettingRow({
  id,
  label,
  description,
  children,
}: {
  id: string;
  label: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-6 border-b border-border/60 py-3.5 last:border-b-0">
      <div className="min-w-0 max-w-[46ch]">
        <label htmlFor={id} className="text-sm font-medium">
          {label}
        </label>
        <p className="mt-0.5 text-[13px] leading-snug text-muted-foreground">{description}</p>
      </div>
      <div className="shrink-0 pt-0.5">{children}</div>
    </div>
  );
}
