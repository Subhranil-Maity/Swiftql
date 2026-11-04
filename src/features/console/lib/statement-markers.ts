// Shows the editor's statements: a bar in the gutter beside the one Ctrl+Enter
// would run, and a faint rule above each statement that starts a new line.
import {
  Decoration,
  type DecorationSet,
  type EditorState,
  EditorView,
  type Extension,
  GutterMarker,
  gutter,
  RangeSetBuilder,
  StateEffect,
  StateField,
  ViewPlugin,
  type ViewUpdate,
} from "@uiw/react-codemirror";

import { type StatementRange, splitStatements } from "@/features/console/lib/sql-statements";

/** Wait after the last edit before splitting again. */
const SPLIT_DELAY_MS = 150;

const setStatements = StateEffect.define<StatementRange[]>();

/** The document's statements: replaced after each split, moved along with edits in between. */
const statementsField = StateField.define<StatementRange[]>({
  create: () => [],
  update(ranges, tr) {
    for (const effect of tr.effects) if (effect.is(setStatements)) return effect.value;
    if (!tr.docChanged) return ranges;
    return ranges
      .map((r) => ({ from: tr.changes.mapPos(r.from, 1), to: tr.changes.mapPos(r.to, -1) }))
      .filter((r) => r.from < r.to);
  },
});

/**
 * The statement Ctrl+Enter runs: the one holding the cursor, else the
 * nearest one before it, else the first one after it.
 */
export function currentStatement(state: EditorState): StatementRange | null {
  const ranges = state.field(statementsField, false) ?? [];
  const head = state.selection.main.head;
  let before: StatementRange | null = null;
  for (const range of ranges) {
    if (range.from <= head && head <= range.to) return range;
    if (range.to < head) before = range;
    else return before ?? range;
  }
  return before;
}

/** Re-splits the document shortly after it changes. */
const splitter = (driverId: string) =>
  ViewPlugin.fromClass(
    class {
      timer: ReturnType<typeof setTimeout> | undefined;
      destroyed = false;

      constructor(readonly view: EditorView) {
        this.schedule(0);
      }

      update(update: ViewUpdate) {
        if (update.docChanged) this.schedule(SPLIT_DELAY_MS);
      }

      schedule(delay: number) {
        clearTimeout(this.timer);
        this.timer = setTimeout(() => void this.split(), delay);
      }

      async split() {
        const doc = this.view.state.doc;
        const ranges = await splitStatements(driverId, doc.toString());
        // Edits made while splitting get a newer split of their own.
        if (this.destroyed || this.view.state.doc !== doc) return;
        this.view.dispatch({ effects: setStatements.of(ranges) });
      }

      destroy() {
        this.destroyed = true;
        clearTimeout(this.timer);
      }
    },
  );

class StatementBar extends GutterMarker {
  elementClass = "cm-statement-bar";
}
const bar = new StatementBar();

const currentStatementGutter = gutter({
  class: "cm-statement-gutter",
  lineMarker(view, line) {
    const current = currentStatement(view.state);
    return current && line.from <= current.to && line.to >= current.from ? bar : null;
  },
  lineMarkerChange: (update) =>
    update.docChanged ||
    update.selectionSet ||
    update.startState.field(statementsField) !== update.state.field(statementsField),
});

const ruleAbove = Decoration.line({ class: "cm-statement-start" });

/** A rule above each statement that begins on a later line than the previous one ends. */
const statementRules = EditorView.decorations.compute([statementsField], (state) => {
  const builder = new RangeSetBuilder<Decoration>();
  let previousEndLine = -1;
  for (const range of state.field(statementsField)) {
    const line = state.doc.lineAt(range.from);
    if (previousEndLine !== -1 && line.number > previousEndLine) {
      builder.add(line.from, line.from, ruleAbove);
    }
    previousEndLine = state.doc.lineAt(range.to).number;
  }
  return builder.finish() as DecorationSet;
});

/** Statement tracking and markers for a driver's SQL. */
export function statementMarkers(driverId: string): Extension {
  return [statementsField, splitter(driverId), currentStatementGutter, statementRules];
}

/** Puts freshly split statements into the editor, e.g. right before a run. */
export function updateStatements(view: EditorView, ranges: StatementRange[]) {
  view.dispatch({ effects: setStatements.of(ranges) });
}
