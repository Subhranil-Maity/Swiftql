// The result grid's two-line header: the column name, its type underneath,
// and the sort arrow on the right. Plugged into Glide's `drawHeader`.
import type { DrawHeaderCallback } from "@glideapps/glide-data-grid";

import { MONO, SANS } from "@/features/console/lib/grid-theme";
import type { SortState } from "@/features/console/model/row-sort";

export const HEADER_HEIGHT = 44;
const PAD_X = 10;
/** Space kept for the arrow on a sorted column. */
export const SORT_ARROW_SPACE = 16;
const NAME_FONT = `500 12px ${SANS}`;
const TYPE_FONT = `11px ${MONO}`;

/** `text` cut to `max` pixels with an ellipsis. */
function fit(ctx: CanvasRenderingContext2D, text: string, max: number): string {
  if (ctx.measureText(text).width <= max) return text;
  let lo = 0;
  let hi = text.length;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (ctx.measureText(`${text.slice(0, mid)}…`).width <= max) lo = mid;
    else hi = mid - 1;
  }
  return `${text.slice(0, lo)}…`;
}

function drawArrow(ctx: CanvasRenderingContext2D, x: number, y: number, up: boolean, color: string) {
  ctx.save();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  const tip = up ? y - 4.5 : y + 4.5;
  const tail = up ? y + 4.5 : y - 4.5;
  const wing = up ? tip + 3.5 : tip - 3.5;
  ctx.beginPath();
  ctx.moveTo(x, tail);
  ctx.lineTo(x, tip);
  ctx.moveTo(x - 3.5, wing);
  ctx.lineTo(x, tip);
  ctx.lineTo(x + 3.5, wing);
  ctx.stroke();
  ctx.restore();
}

/**
 * `types[i]` is the type of source column i; Glide columns carry their
 * source index as `id`.
 */
export function makeDrawHeader(
  types: string[],
  sort: SortState,
  colors: { accent: string; typeText: string },
): DrawHeaderCallback {
  return ({ ctx, column, rect, theme, isSelected }) => {
    const source = Number(column.id);
    const sorted = sort?.column === source;
    const right = rect.x + rect.width - PAD_X - (sorted ? SORT_ARROW_SPACE : 0);
    const max = Math.max(0, right - rect.x - PAD_X);

    ctx.save();
    ctx.beginPath();
    ctx.rect(rect.x, rect.y, rect.width, rect.height);
    ctx.clip();
    ctx.textBaseline = "middle";
    ctx.textAlign = "left";

    ctx.font = NAME_FONT;
    ctx.fillStyle = isSelected ? theme.textHeaderSelected : theme.textHeader;
    ctx.fillText(fit(ctx, column.title, max), rect.x + PAD_X, rect.y + 16);

    const type = types[source];
    if (type) {
      ctx.font = TYPE_FONT;
      ctx.fillStyle = isSelected ? theme.textHeaderSelected : colors.typeText;
      ctx.fillText(fit(ctx, type, max), rect.x + PAD_X, rect.y + 31);
    }

    if (sorted && sort) {
      const color = isSelected ? theme.textHeaderSelected : colors.accent;
      drawArrow(ctx, rect.x + rect.width - PAD_X - 4, rect.y + rect.height / 2, sort.direction === "asc", color);
    }
    ctx.restore();
  };
}
