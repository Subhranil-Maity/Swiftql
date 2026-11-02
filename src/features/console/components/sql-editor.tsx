/** A plain SQL text area that fills its panel. */
export function SqlEditor({
  value,
  onChange,
  label,
}: {
  value: string;
  onChange: (sql: string) => void;
  /** Accessible name, e.g. the tab title. */
  label: string;
}) {
  return (
    // TODO: syntax highlighting and IntelliSense (needs a real editor component)
    <textarea
      aria-label={label}
      value={value}
      onChange={(event) => onChange(event.target.value)}
      placeholder="Write SQL here"
      spellCheck={false}
      autoCapitalize="off"
      autoCorrect="off"
      autoComplete="off"
      className="size-full resize-none bg-transparent px-4 py-3 font-mono text-sm leading-6 outline-none placeholder:text-muted-foreground"
    />
  );
}
