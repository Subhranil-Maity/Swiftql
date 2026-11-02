// CodeMirror theme for the SQL editor. Every colour is an app CSS variable
// (src/index.css), so the editor follows light/dark and the neutral palette
// like the rest of the UI. The `dark` flag only picks CodeMirror's base
// styles for anything not overridden here.
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import { EditorView, type Extension } from "@uiw/react-codemirror";

/** `color` at `percent` opacity, for tints over the editor background. */
const tint = (color: string, percent: number) =>
  `color-mix(in oklab, ${color} ${percent}%, transparent)`;

const chrome = {
  "&": {
    height: "100%",
    backgroundColor: "var(--background)",
    color: "var(--foreground)",
    fontSize: "var(--text-sm)",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    lineHeight: "1.65",
  },
  ".cm-content": {
    padding: "12px 0",
    caretColor: "var(--foreground)",
  },
  ".cm-line": { padding: "0 16px 0 4px" },
  ".cm-cursor, .cm-dropCursor": { borderLeft: "2px solid var(--foreground)" },
  ".cm-placeholder": { color: "var(--muted-foreground)" },

  // Selection and matches.
  ".cm-selectionBackground": { backgroundColor: tint("var(--foreground)", 12) },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground": {
    backgroundColor: tint("var(--foreground)", 18),
  },
  ".cm-content ::selection": { backgroundColor: tint("var(--foreground)", 18) },
  ".cm-activeLine": { backgroundColor: tint("var(--foreground)", 4) },
  ".cm-selectionMatch": { backgroundColor: tint("var(--foreground)", 9) },
  "&.cm-focused .cm-matchingBracket": {
    backgroundColor: tint("var(--foreground)", 14),
    outline: "none",
  },
  "&.cm-focused .cm-nonmatchingBracket": {
    backgroundColor: tint("var(--destructive)", 25),
  },
  ".cm-searchMatch": {
    backgroundColor: tint("var(--warning)", 25),
    outline: `1px solid ${tint("var(--warning)", 60)}`,
  },
  ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: tint("var(--warning)", 45) },

  // Gutter: line numbers sit quietly on the same surface as the code.
  ".cm-gutters": {
    backgroundColor: "var(--background)",
    color: tint("var(--muted-foreground)", 70),
    border: "none",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    minWidth: "3ch",
    padding: "0 8px 0 16px",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "transparent",
    color: "var(--foreground)",
  },
  ".cm-foldGutter .cm-gutterElement": { color: "var(--muted-foreground)" },
  ".cm-foldPlaceholder": {
    backgroundColor: "var(--muted)",
    border: "none",
    color: "var(--muted-foreground)",
    padding: "0 6px",
    borderRadius: "var(--radius-sm)",
  },

  // Popups (autocomplete, hover info) look like the app's popovers.
  ".cm-tooltip": {
    backgroundColor: "var(--popover)",
    color: "var(--popover-foreground)",
    border: "1px solid var(--border)",
    borderRadius: "var(--radius-md)",
    boxShadow: "0 4px 16px -4px rgb(0 0 0 / 0.2)",
    overflow: "hidden",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul": {
    fontFamily: "var(--font-mono)",
    fontSize: "var(--text-sm)",
    maxHeight: "16rem",
    padding: "4px",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li": {
    padding: "2px 8px",
    borderRadius: "var(--radius-sm)",
    lineHeight: "1.6",
  },
  ".cm-tooltip-autocomplete ul li[aria-selected]": {
    backgroundColor: "var(--accent)",
    color: "var(--accent-foreground)",
  },
  ".cm-completionMatchedText": {
    textDecoration: "none",
    fontWeight: "600",
  },
  ".cm-completionDetail": {
    color: "var(--muted-foreground)",
    fontStyle: "normal",
    marginLeft: "1em",
  },
  ".cm-completionIcon": { color: "var(--muted-foreground)", opacity: "1" },

  // Syntax errors: a wavy underline in the app's red, the message on hover.
  ".cm-lintRange-error": {
    backgroundImage: "none",
    textDecoration: "underline wavy var(--destructive)",
    textDecorationSkipInk: "none",
    textUnderlineOffset: "3px",
  },
  ".cm-tooltip-lint": { padding: "0" },
  ".cm-diagnostic": {
    fontFamily: "var(--font-sans)",
    fontSize: "var(--text-sm)",
    padding: "6px 10px 6px 12px",
    marginLeft: "0",
    maxWidth: "36rem",
    whiteSpace: "pre-wrap",
  },
  ".cm-diagnostic-error": { borderLeft: "3px solid var(--destructive)" },
  ".cm-diagnosticSource": {
    color: "var(--muted-foreground)",
    fontSize: "0.75rem",
    opacity: "1",
    marginTop: "2px",
  },
  ".cm-panel.cm-panel-lint ul [aria-selected]": {
    backgroundColor: "var(--accent)",
    color: "var(--accent-foreground)",
  },

  // Search panel (Ctrl+F).
  ".cm-panels": {
    backgroundColor: "var(--background)",
    color: "var(--foreground)",
  },
  ".cm-panels.cm-panels-bottom": { borderTop: "1px solid var(--border)" },
  ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--border)" },
  ".cm-panel.cm-search": {
    fontFamily: "var(--font-sans)",
    fontSize: "var(--text-sm)",
    padding: "6px 12px",
  },
  ".cm-panel.cm-search input, .cm-panel.cm-search button": {
    font: "inherit",
    borderRadius: "var(--radius-sm)",
  },
  ".cm-textfield": {
    backgroundColor: "transparent",
    border: "1px solid var(--input)",
    color: "var(--foreground)",
    padding: "2px 6px",
  },
  ".cm-button": {
    backgroundImage: "none",
    backgroundColor: "var(--secondary)",
    color: "var(--secondary-foreground)",
    border: "1px solid var(--border)",
    padding: "2px 8px",
  },
};

const highlight = HighlightStyle.define([
  { tag: [t.keyword, t.operatorKeyword, t.controlKeyword, t.modifier], color: "var(--syntax-keyword)" },
  { tag: [t.typeName, t.standard(t.name)], color: "var(--syntax-type)" },
  { tag: [t.string, t.special(t.string)], color: "var(--syntax-string)" },
  { tag: [t.number, t.bool, t.null, t.special(t.name)], color: "var(--syntax-number)" },
  { tag: [t.comment, t.lineComment, t.blockComment], color: "var(--syntax-comment)", fontStyle: "italic" },
  {
    tag: [t.operator, t.punctuation, t.separator, t.paren, t.squareBracket, t.brace],
    color: "var(--syntax-punctuation)",
  },
  { tag: t.invalid, color: "var(--destructive)" },
]);

function editorTheme(dark: boolean): Extension {
  return [EditorView.theme(chrome, { dark }), syntaxHighlighting(highlight)];
}

export const lightEditorTheme = editorTheme(false);
export const darkEditorTheme = editorTheme(true);
