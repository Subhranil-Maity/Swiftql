import { useMemo } from "react";
import CodeMirror, { EditorView } from "@uiw/react-codemirror";

import { darkEditorTheme, lightEditorTheme } from "@/features/console/lib/editor-theme";
import { sqlLanguage } from "@/features/console/lib/sql-language";
import { sqlLinter } from "@/features/console/lib/sql-lint";
import { useIsDark } from "@/stores/theme";

/** CodeMirror SQL editor that fills its panel. */
export function SqlEditor({
  value,
  onChange,
  label,
  driverId,
}: {
  value: string;
  onChange: (sql: string) => void;
  /** Accessible name, e.g. the tab title. */
  label: string;
  /** Session's driver; picks the SQL dialect and syntax checker. */
  driverId: string;
}) {
  const isDark = useIsDark();

  const extensions = useMemo(
    () => [
      sqlLanguage(driverId),
      sqlLinter(driverId),
      EditorView.contentAttributes.of({ "aria-label": label }),
    ],
    [driverId, label],
  );

  return (
    // TODO: Ctrl/Cmd+Enter runs the query once execution exists
    <CodeMirror
      value={value}
      onChange={onChange}
      extensions={extensions}
      theme={isDark ? darkEditorTheme : lightEditorTheme}
      placeholder="Write SQL here"
      height="100%"
      className="h-full min-h-0 overflow-hidden"
    />
  );
}
