import { type ReactNode, useEffect, useId, useRef, useState } from "react";
import { toast } from "sonner";

import { Separator } from "@/components/ui/separator";
import type { DriverInfo, SavedConnection } from "@/lib/ipc";
import { IpcError } from "@/lib/ipc-error";
import { useConnectionsStore } from "@/stores/connections";

import { useConnectionForm } from "../hooks/use-connection-form";
import { useTestConnection } from "../hooks/use-test-connection";
import { type FormState, toInput } from "../model/form-state";
import { AuthFields } from "./auth-fields";
import { ConnectionFormFooter, type SubmitAction } from "./connection-form-footer";
import { ConnectionPreview } from "./connection-preview";
import { ExtraOptionField } from "./extra-option-field";
import { GeneralFields } from "./general-fields";
import { TargetFields } from "./target-fields";
import { ConnectionErrorAlert } from "./test-connection-result";
import { TlsFields } from "./tls-fields";

/**
 * The connection form. It renders two parts, the fields and the footer
 * buttons, and hands them to `children` so the page can put the footer in
 * its pinned footer slot.
 */
export function ConnectionForm({
  driver,
  initial,
  onCancel,
  onSaved,
  children,
}: {
  driver: DriverInfo;
  initial: FormState;
  onCancel: () => void;
  onSaved: (saved: SavedConnection, action: SubmitAction) => void;
  children: (parts: { body: ReactNode; footer: ReactNode }) => ReactNode;
}) {
  const formApi = useConnectionForm(initial, driver);
  const { form, check, setOption } = formApi;
  const { state: test, test: runTest } = useTestConnection(form);
  const save = useConnectionsStore((s) => s.save);
  const [submitting, setSubmitting] = useState<SubmitAction | null>(null);
  const [saveError, setSaveError] = useState<IpcError | null>(null);
  const formRef = useRef<HTMLFormElement>(null);
  const errorRef = useRef<HTMLDivElement>(null);
  const formId = useId();

  /** Moves focus to the first invalid field once errors are rendered. */
  const focusFirstError = () =>
    requestAnimationFrame(() =>
      formRef.current?.querySelector<HTMLElement>("[aria-invalid=true]")?.focus(),
    );

  const onTest = () => {
    if (!check("test")) return focusFirstError();
    void runTest(toInput(form));
  };

  const onSubmit = async (action: SubmitAction) => {
    if (!check("save")) return focusFirstError();
    setSubmitting(action);
    setSaveError(null);
    try {
      const saved = await save(toInput(form));
      if (action === "save") toast.success("Connection saved");
      onSaved(saved, action);
    } catch (error) {
      setSaveError(IpcError.from(error));
    } finally {
      setSubmitting(null);
    }
  };

  const error = saveError ?? (test.status === "error" ? test.error : null);

  // The error sits below the fields; bring it into view when it appears.
  useEffect(() => {
    if (error) errorRef.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [error]);

  const body = (
    <form
      id={formId}
      ref={formRef}
      noValidate
      onSubmit={(e) => {
        e.preventDefault();
        void onSubmit("saveAndConnect");
      }}
      className="flex flex-col gap-5"
    >
      <GeneralFields {...formApi} />
      <Separator />
      <TargetFields api={formApi} driver={driver} />
      <AuthFields api={formApi} driver={driver} />
      <TlsFields />
      {driver.extraOptions.length > 0 && (
        <div className="grid gap-4 sm:grid-cols-2">
          {driver.extraOptions.map((spec) => (
            <ExtraOptionField
              key={spec.key}
              spec={spec}
              value={form.options[spec.key] ?? ""}
              onChange={(v) => setOption(spec.key, v)}
            />
          ))}
        </div>
      )}
      <ConnectionPreview form={form} />
      {error && (
        <div ref={errorRef} className="scroll-mb-4">
          <ConnectionErrorAlert error={error} />
        </div>
      )}
    </form>
  );

  const footer = (
    <ConnectionFormFooter
      formId={formId}
      test={test}
      onTest={onTest}
      onCancel={onCancel}
      onSubmit={(action) => void onSubmit(action)}
      submitting={submitting}
    />
  );

  return children({ body, footer });
}
