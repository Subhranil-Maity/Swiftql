import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

import { CONNECTION_COLORS, type ConnectionColor, colorVar } from "../model/colors";

export function ColorSwatches({
  value,
  onChange,
  labelledBy,
}: {
  value: ConnectionColor;
  onChange: (color: ConnectionColor) => void;
  labelledBy: string;
}) {
  return (
    <ToggleGroup
      aria-labelledby={labelledBy}
      value={[value]}
      // A swatch can't be un-pressed: ignore the empty selection.
      onValueChange={(next) => next[0] && onChange(next[0] as ConnectionColor)}
      spacing={1}
      size="sm"
    >
      {CONNECTION_COLORS.map((c) => (
        <ToggleGroupItem key={c.id} value={c.id} aria-label={c.label} className="px-1.5">
          <span
            aria-hidden
            className="size-4 rounded-full ring-offset-2 ring-offset-background group-aria-pressed/toggle:ring-2 group-aria-pressed/toggle:ring-foreground/70"
            style={{ background: colorVar(c.id) }}
          />
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}
