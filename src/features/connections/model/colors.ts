// Connection colours: the one identity cue for a connection, stored in
// `SavedConnection.color`. Red = production is a convention users know.

export const CONNECTION_COLORS = [
  { id: "slate", label: "Slate" },
  { id: "blue", label: "Blue" },
  { id: "green", label: "Green" },
  { id: "amber", label: "Amber" },
  { id: "red", label: "Red" },
  { id: "violet", label: "Violet" },
] as const;

export type ConnectionColor = (typeof CONNECTION_COLORS)[number]["id"];

export const DEFAULT_COLOR: ConnectionColor = "slate";

export function isConnectionColor(value: unknown): value is ConnectionColor {
  return CONNECTION_COLORS.some((c) => c.id === value);
}

/** CSS colour for a stored value; unknown or missing falls back to slate. */
export function colorVar(value: string | null | undefined): string {
  return `var(--conn-${isConnectionColor(value) ? value : DEFAULT_COLOR})`;
}
