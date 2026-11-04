// Theme for the results grid. Glide draws on a canvas, where `var(--x)`
// doesn't resolve, and it mixes colours itself, which needs plain rgba. So the
// app's CSS variables (src/index.css) are read and converted here; call again
// when the theme changes.
import type { Theme } from "@glideapps/glide-data-grid";

let probe: CanvasRenderingContext2D | null = null;

/** Any CSS colour (oklch included) as [r, g, b, a] with a in 0..1. */
function toRgba(color: string): [number, number, number, number] {
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

function rgba([r, g, b, a]: [number, number, number, number], alpha = 1): string {
  return `rgba(${r}, ${g}, ${b}, ${+(a * alpha).toFixed(3)})`;
}

const MONO = "'Geist Mono Variable', ui-monospace, monospace";
const SANS = "'Geist Variable', sans-serif";

export function gridTheme(): Partial<Theme> {
  const css = getComputedStyle(document.documentElement);
  const read = (name: string) => toRgba(css.getPropertyValue(name).trim() || "#888");

  const background = read("--background");
  const foreground = read("--foreground");
  const muted = read("--muted-foreground");
  const border = read("--border");
  const ring = read("--ring");

  return {
    accentColor: rgba(foreground, 0.55),
    accentFg: rgba(background),
    accentLight: rgba(foreground, 0.08),
    textDark: rgba(foreground),
    textMedium: rgba(muted),
    textLight: rgba(muted, 0.75),
    textBubble: rgba(foreground),
    bgIconHeader: rgba(muted),
    fgIconHeader: rgba(background),
    textHeader: rgba(muted),
    textHeaderSelected: rgba(background),
    bgCell: rgba(background),
    bgCellMedium: rgba(foreground, 0.03),
    bgHeader: rgba(background),
    bgHeaderHasFocus: rgba(foreground, 0.06),
    bgHeaderHovered: rgba(foreground, 0.05),
    bgBubble: rgba(foreground, 0.06),
    bgBubbleSelected: rgba(background),
    bgSearchResult: rgba(foreground, 0.12),
    borderColor: rgba(border),
    horizontalBorderColor: rgba(border, 0.6),
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
  };
}
