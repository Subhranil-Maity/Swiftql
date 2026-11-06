import { useEffect, useMemo, useRef } from "react";
import { indentUnit } from "@codemirror/language";
import CodeMirror, { EditorState, EditorView, keymap, Prec } from "@uiw/react-codemirror";

import { darkEditorTheme, lightEditorTheme } from "@/features/console/lib/editor-theme";
import { sqlLanguage } from "@/features/console/lib/sql-language";
import { sqlLinter } from "@/features/console/lib/sql-lint";
import { statementMarkers } from "@/features/console/lib/statement-markers";
import { registerEditor, unregisterEditor } from "@/features/console/model/editor-registry";
import { useSchemaCatalogStore } from "@/stores/schema-catalog";
import { useSettingsStore } from "@/stores/settings";
import { useIsDark } from "@/stores/theme";

/** CodeMirror SQL editor that fills its panel. */
export function SqlEditor({
  tabId,
  sessionId,
  value,
  onChange,
  label,
  driverId,
  onRunStatement,
  onRunScript,
}: {
  /** Registers the view so the Run button can read its cursor. */
  tabId: string;
  /** Its database's tables and columns are offered as completions. */
  sessionId: string;
  value: string;
  onChange: (sql: string) => void;
  /** Accessible name, e.g. the tab title. */
  label: string;
  /** Session's driver; picks the SQL dialect, syntax checker and statement splitter. */
  driverId: string;
  /** Ctrl/Cmd+Enter. */
  onRunStatement: () => void;
  /** Ctrl/Cmd+Shift+Enter. */
  onRunScript: () => void;
}) {
  const isDark = useIsDark();
  const catalog = useSchemaCatalogStore((s) => s.catalogs[sessionId]);
  const editor = useSettingsStore((s) => s.settings?.editor);
  const fontFamily = editor?.fontFamily ?? "ui-monospace, monospace";
  const fontSize = editor?.fontSize ?? 14;
  const tabSize = editor?.tabSize ?? 2;
  const wordWrap = editor?.wordWrap ?? false;
  const upperCase = (editor?.keywordCase ?? "upper") === "upper";
  const lint = editor?.lint ?? true;

  // Latest handlers without rebuilding the extensions on every render.
  const handlers = useRef({ onRunStatement, onRunScript });
  useEffect(() => {
    handlers.current = { onRunStatement, onRunScript };
  });

  const extensions = useMemo(
    () => [
      sqlLanguage(driverId, catalog, upperCase),
      lint ? sqlLinter(driverId) : [],
      statementMarkers(driverId),
      EditorState.tabSize.of(tabSize),
      indentUnit.of(" ".repeat(tabSize)),
      wordWrap ? EditorView.lineWrapping : [],
      EditorView.theme({
        "&": { fontSize: `${fontSize}px` },
        ".cm-content, .cm-gutters, .cm-tooltip-autocomplete": { fontFamily },
      }),
      // Above the default keymap, where Mod-Enter would insert a blank line.
      Prec.highest(
        keymap.of([
          {
            key: "Mod-Enter",
            run: () => {
              handlers.current.onRunStatement();
              return true;
            },
          },
          {
            key: "Mod-Shift-Enter",
            run: () => {
              handlers.current.onRunScript();
              return true;
            },
          },
        ]),
      ),
      EditorView.contentAttributes.of({ "aria-label": label }),
    ],
    [driverId, label, catalog, upperCase, lint, tabSize, wordWrap, fontSize, fontFamily],
  );

  const viewRef = useRef<EditorView | null>(null);
  useEffect(
    () => () => {
      if (viewRef.current) unregisterEditor(tabId, viewRef.current);
    },
    [tabId],
  );

  return (
    <CodeMirror
      value={value}
      onChange={onChange}
      onCreateEditor={(view) => {
        viewRef.current = view;
        registerEditor(tabId, view);
      }}
      extensions={extensions}
      theme={isDark ? darkEditorTheme : lightEditorTheme}
      placeholder="Write SQL here"
      basicSetup={{
        lineNumbers: editor?.lineNumbers ?? true,
        foldGutter: editor?.lineNumbers ?? true,
        highlightActiveLine: editor?.highlightActiveLine ?? true,
        highlightActiveLineGutter: editor?.highlightActiveLine ?? true,
        autocompletion: editor?.autocomplete ?? true,
        tabSize,
      }}
      height="100%"
      className="h-full min-h-0 overflow-hidden"
    />
  );
}
