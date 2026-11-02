import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";

import type { TestState } from "../hooks/use-test-connection";
import { TestConnectionResult } from "./test-connection-result";

export type SubmitAction = "save" | "saveAndConnect";

/**
 * Test on the left, Cancel / Save / Save and connect on the right. On a
 * narrow window the buttons wrap onto their own row. Lives outside the
 * `<form>` (in the page footer), so the submit button points at it by id.
 */
export function ConnectionFormFooter({
  formId,
  test,
  onTest,
  onCancel,
  onSubmit,
  submitting,
}: {
  formId: string;
  test: TestState;
  onTest: () => void;
  onCancel: () => void;
  onSubmit: (action: SubmitAction) => void;
  /** Which submit button is running, if any. */
  submitting: SubmitAction | null;
}) {
  const busy = submitting !== null;
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
      <div className="flex min-w-0 flex-[1_1_14rem] items-center gap-3">
        <Button
          type="button"
          variant="outline"
          onClick={onTest}
          disabled={busy || test.status === "testing"}
        >
          Test connection
        </Button>
        <TestConnectionResult state={test} />
      </div>
      <div className="ml-auto flex shrink-0 flex-wrap justify-end gap-2">
        <Button type="button" variant="ghost" onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
        <Button type="button" variant="outline" onClick={() => onSubmit("save")} disabled={busy}>
          {submitting === "save" && <Spinner data-icon="inline-start" />}
          Save
        </Button>
        <Button type="submit" form={formId} disabled={busy}>
          {submitting === "saveAndConnect" && <Spinner data-icon="inline-start" />}
          Save and connect
        </Button>
      </div>
    </div>
  );
}
