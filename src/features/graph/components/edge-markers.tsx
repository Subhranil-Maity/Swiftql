// Crow's-foot ends for foreign-key edges. Drawn as paths inside each edge
// rather than SVG <marker> defs, so they survive PNG/SVG export (the export
// only captures the viewport, where shared defs don't live).

/** +1: the edge leaves the card to the right; -1: to the left. */
export type Outward = 1 | -1;

const BAR_AT = 8;
const FOOT_LENGTH = 10;
const SPREAD = 5;

/** "Exactly one": a bar across the line, near the card. */
export function oneMarkerPath(x: number, y: number, out: Outward): string {
  const bx = x + out * BAR_AT;
  return `M ${bx} ${y - SPREAD} L ${bx} ${y + SPREAD}`;
}

/** "Many": three prongs fanning out onto the card's edge. */
export function manyMarkerPath(x: number, y: number, out: Outward): string {
  const hx = x + out * FOOT_LENGTH;
  return `M ${hx} ${y} L ${x} ${y - SPREAD} M ${hx} ${y} L ${x} ${y} M ${hx} ${y} L ${x} ${y + SPREAD}`;
}

/** Both ends of an edge as one path: "one" at the referenced table, "many" at the referencing one. */
export function EdgeEnds({
  one,
  many,
  className,
  style,
}: {
  one: { x: number; y: number; out: Outward };
  many: { x: number; y: number; out: Outward };
  className?: string;
  style?: React.CSSProperties;
}) {
  return (
    <path
      d={`${oneMarkerPath(one.x, one.y, one.out)} ${manyMarkerPath(many.x, many.y, many.out)}`}
      fill="none"
      strokeLinecap="round"
      className={className}
      style={style}
    />
  );
}
