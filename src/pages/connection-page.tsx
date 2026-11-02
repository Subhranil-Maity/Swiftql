import { useEffect, useId, useState } from "react";
import { CircleAlertIcon } from "lucide-react";

import { PageShell } from "@/components/page-shell";
import { Alert, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { ConnectionForm } from "@/features/connections/components/connection-form";
import { DriverPicker } from "@/features/connections/components/driver-picker";
import { emptyForm, fromSaved } from "@/features/connections/model/form-state";
import { useConnectionsStore } from "@/stores/connections";
import { useDriversStore } from "@/stores/drivers";
import { useHealthStore } from "@/stores/health";
import { useNavigationStore } from "@/stores/navigation";

/**
 * New / edit connection. A new connection starts at the driver picker, an
 * edit goes straight to the form.
 */
export function ConnectionPage({ editingId }: { editingId: string | null }) {
  const go = useNavigationStore((s) => s.go);
  const drivers = useDriversStore((s) => s.drivers);
  const driversStatus = useDriversStore((s) => s.status);
  const driversError = useDriversStore((s) => s.error);
  const editing = useConnectionsStore((s) =>
    editingId ? s.connections.find((c) => c.id === editingId) : undefined,
  );
  const connectionsLoaded = useConnectionsStore((s) => s.loaded);
  const recheck = useHealthStore((s) => s.check);

  const [driverId, setDriverId] = useState<string | null>(editing?.driver ?? null);
  const [step, setStep] = useState<"pick" | "form">(editingId ? "form" : "pick");
  const pickerLabel = useId();

  const toWelcome = () => go({ page: "welcome" });

  // The connection was deleted (or never existed): nothing to edit.
  const missing = editingId !== null && connectionsLoaded && !editing;
  useEffect(() => {
    if (missing) go({ page: "welcome" });
  }, [missing, go]);

  // Picker default once drivers arrive.
  const selected = editing?.driver ?? driverId ?? drivers[0]?.id ?? null;
  const driver = drivers.find((d) => d.id === selected);
  const title = editing ? `Edit ${editing.name}` : "New connection";

  if (step === "pick" && !editingId) {
    return (
      <PageShell
        title={title}
        onBack={toWelcome}
        backLabel="Back to connections"
        footer={
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={toWelcome}>
              Cancel
            </Button>
            <Button disabled={!driver} onClick={() => setStep("form")}>
              Continue
            </Button>
          </div>
        }
      >
        <div className="flex flex-col gap-3 duration-150 animate-in fade-in-0 slide-in-from-left-2 motion-reduce:animate-none">
          <p id={pickerLabel} className="text-muted-foreground">
            Choose a database
          </p>
          <DriverPicker
            value={selected}
            onChange={setDriverId}
            onContinue={() => driver && setStep("form")}
            labelledBy={pickerLabel}
          />
        </div>
      </PageShell>
    );
  }

  if (!driver || (editingId && !editing)) {
    return (
      <PageShell title={title} onBack={toWelcome} backLabel="Back to connections">
        {driversStatus === "error" ? (
          <Alert variant="destructive">
            <CircleAlertIcon />
            <AlertTitle>{driversError}</AlertTitle>
          </Alert>
        ) : driversStatus === "ready" && editing ? (
          <Alert variant="destructive">
            <CircleAlertIcon />
            <AlertTitle>No driver named "{editing.driver}" is available.</AlertTitle>
          </Alert>
        ) : (
          <div className="flex h-16 items-center justify-center text-muted-foreground">
            <Spinner />
          </div>
        )}
      </PageShell>
    );
  }

  return (
    <ConnectionForm
      // A different driver means a fresh form.
      key={driver.id}
      driver={driver}
      initial={editing ? fromSaved(editing, driver) : emptyForm(driver)}
      onCancel={toWelcome}
      onSaved={(saved, action) => {
        void recheck(saved.id);
        go(
          action === "saveAndConnect"
            ? { page: "welcome", connectId: saved.id }
            : { page: "welcome" },
        );
      }}
    >
      {({ body, footer }) => (
        <PageShell
          title={title}
          subtitle={editing ? undefined : driver.name}
          onBack={editing ? toWelcome : () => setStep("pick")}
          backLabel={editing ? "Back to connections" : "Back to database list"}
          footer={footer}
        >
          <div className="duration-150 animate-in fade-in-0 slide-in-from-right-2 motion-reduce:animate-none">
            {body}
          </div>
        </PageShell>
      )}
    </ConnectionForm>
  );
}
