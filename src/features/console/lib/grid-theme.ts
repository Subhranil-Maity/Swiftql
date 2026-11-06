// Theme for the results grid. Glide draws on a canvas, where `var(--x)`
// doesn't resolve, and it mixes colours itself, which needs plain rgba. So the
// app's CSS variables (src/index.css) are read and converted here; call again
// when the theme changes.
import type { Theme } from "@glideapps/glide-data-grid";

type Rgba = [number, number, number, number];

let probe: CanvasRenderingContext2D | null = null;

/** Any CSS colour (oklch included) as [r, g, b, a] with a in 0..1. */
function toRgba(color: string): Rgba {
  probe ??= (() => {
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 1;
    return canvas.getContext("2d", { willReadFrequently: true });
  })();
  if (!probe) return [0, 0, 0, 1];
  probe.clearRect(0, 0, 1, 1);
  probe.fillStyle = "#000";
  probe.fillStyle = color;
  probe.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = probe.getImageData(0, 0, 1, 1).data;
  return [r, g, b, a / 255];
}

function rgba([r, g, b, a]: Rgba, alpha = 1): string {
  return `rgba(${r}, ${g}, ${b}, ${+(a * alpha).toFixed(3)})`;
}

/** `over` laid on `base` at `amount`, as one opaque colour. Translucent fills
 * would darken each time Glide repaints a damaged cell over itself. */
function mix(base: Rgba, over: Rgba, amount: number): string {
  const t = amount * over[3];
  const c = (i: number) => Math.round(base[i] * (1 - t) + over[i] * t);
  return `rgb(${c(0)}, ${c(1)}, ${c(2)})`;
}

export const MONO = "'Geist Mono Variable', ui-monospace, monospace";
export const SANS = "'Geist Variable', sans-serif";

export type GridPalette = {
  theme: Partial<Theme>;
  /** Every other row, for following a row across. */
  zebra: string;
  /** The connection colour: sort arrows, selection. */
  accent: string;
  /** The type line under each header name. */
  typeText: string;
};

/** Reads the colours in effect at `element`, which sits inside the workspace's `--tab-accent`. */
export function gridPalette(element: Element = document.documentElement): GridPalette {
  const css = getComputedStyle(element);
  const read = (name: string, fallback = "#888") => toRgba(css.getPropertyValue(name).trim() || fallback);

  const background = read("--background");
  const foreground = read("--foreground");
  const muted = read("--muted-foreground");
  const chrome = read("--muted");
  const border = read("--border");
  const ring = read("--ring");
  const accent = read("--tab-accent", css.getPropertyValue("--ring").trim());

  return {
    zebra: mix(background, foreground, 0.025),
    accent: rgba(accent),
    typeText: rgba(muted, 0.85),
    theme: {
      accentColor: rgba(accent),
      accentFg: "rgba(255, 255, 255, 1)",
      accentLight: rgba(accent, 0.14),
      textDark: rgba(foreground),
      textMedium: rgba(muted),
      textLight: rgba(muted, 0.75),
      textBubble: rgba(foreground),
      bgIconHeader: rgba(muted),
      fgIconHeader: rgba(background),
      textHeader: rgba(foreground),
      textHeaderSelected: "rgba(255, 255, 255, 1)",
      bgCell: rgba(background),
      bgCellMedium: mix(background, foreground, 0.03),
      bgHeader: mix(background, chrome, 1),
      bgHeaderHasFocus: mix(background, foreground, 0.08),
      bgHeaderHovered: mix(background, foreground, 0.07),
      bgBubble: rgba(foreground, 0.06),
      bgBubbleSelected: rgba(background),
      bgSearchResult: rgba(accent, 0.28),
      borderColor: rgba(border),
      horizontalBorderColor: rgba(border, 0.7),
      headerBottomBorderColor: rgba(border),
      drilldownBorder: rgba(border),
      linkColor: rgba(foreground),
      resizeIndicatorColor: rgba(ring),
      cellHorizontalPadding: 10,
      cellVerticalPadding: 4,
      // Glide builds the header font as `${headerFontStyle} ${fontFamily}`, so ending
      // the style with a family list puts headers in Geist and cells in Geist Mono.
      headerFontStyle: `500 12px ${SANS},`,
      baseFontStyle: "12.5px",
      markerFontStyle: "11px",
      fontFamily: MONO,
      editorFontSize: "12.5px",
      lineHeight: 1.4,
      headerIconSize: 16,
    },
  };
}
