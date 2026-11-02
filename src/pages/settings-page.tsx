import { useEffect, useId, useState } from "react";
import { CircleAlertIcon } from "lucide-react";

import { PageShell } from "@/components/page-shell";
import { Alert, AlertTitle } from "@/components/ui/alert";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { IpcError } from "@/lib/ipc-error";
import { useNavigationStore } from "@/stores/navigation";
import { useSettingsStore } from "@/stores/settings";

export function SettingsPage() {
  const go = useNavigationStore((s) => s.go);
  const { settings, error } = useSettingsStore();

  return (
    <PageShell
      title="Settings"
      onBack={() => go({ page: "welcome" })}
      backLabel="Back to connections"
    >
      {error ? (
        <Alert variant="destructive">
          <CircleAlertIcon />
          <AlertTitle>{error}</AlertTitle>
        </Alert>
      ) : !settings ? (
        <div className="flex h-16 items-center justify-center text-muted-foreground">
          <Spinner />
        </div>
      ) : (
        <FieldSet>
          <FieldLegend>Connections</FieldLegend>
          <FieldGroup>
            <CheckOnStartupField value={settings.connection.checkOnStartup} />
          </FieldGroup>
        </FieldSet>
      )}
    </PageShell>
  );
}

// Same bounds as `ConnectionSettings` validation in config-store (settings.rs).
const MAX_CHECKS = 50;

/** Number of recent connections to check on launch. Saves on blur or Enter. */
function CheckOnStartupField({ value }: { value: number }) {
  const id = useId();
  const update = useSettingsStore((s) => s.update);
  const [draft, setDraft] = useState(String(value));
  const [error, setError] = useState<string | null>(null);

  // Follow changes saved elsewhere (another window, a reset).
  useEffect(() => setDraft(String(value)), [value]);

  const commit = async () => {
    const text = draft.trim();
    if (!/^\d+$/.test(text) || Number(text) > MAX_CHECKS) {
      setError(`Enter a whole number from 0 to ${MAX_CHECKS}.`);
      return;
    }
    const next = Number(text);
    setError(null);
    setDraft(String(next));
    if (next === value) return;
    try {
      await update({ connection: { checkOnStartup: next } });
    } catch (e) {
      setError(IpcError.from(e).message);
    }
  };

  return (
    <Field data-invalid={!!error}>
      <FieldLabel htmlFor={id}>Check recent connections on startup</FieldLabel>
      <Input
        id={id}
        type="number"
        inputMode="numeric"
        min={0}
        max={MAX_CHECKS}
        step={1}
        value={draft}
        onChange={(e) => setDraft(e.currentTarget.value)}
        onBlur={() => void commit()}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void commit();
          }
        }}
        aria-invalid={!!error}
        aria-describedby={`${id}-desc`}
        className="w-24 font-mono"
      />
      <FieldDescription id={`${id}-desc`}>
        How many recently used connections to test when SwiftQL opens. 0 turns it off.
      </FieldDescription>
      <FieldError>{error}</FieldError>
    </Field>
  );
}
