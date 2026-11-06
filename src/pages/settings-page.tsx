import { ChevronLeftIcon, CircleAlertIcon, RotateCcwIcon, SearchIcon } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import { Alert, AlertTitle } from "@/components/ui/alert";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { DisplayPreview } from "@/features/settings/components/display-preview";
import { SecurityExtras } from "@/features/settings/components/security-extras";
import { SettingRow } from "@/features/settings/components/setting-row";
import { matches, SECTIONS, type SectionId } from "@/features/settings/model/sections";
import { api, type Settings } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { cn } from "@/lib/utils";
import { useNavigationStore } from "@/stores/navigation";
import { type SettingsPatch, useSettingsStore } from "@/stores/settings";

/** Settings: a section list on the left, every setting on the right, searchable. */
export function SettingsPage() {
  const go = useNavigationStore((s) => s.go);
  const { settings, error } = useSettingsStore();
  const [query, setQuery] = useState("");
  const [current, setCurrent] = useState<SectionId>("general");
  const [confirmReset, setConfirmReset] = useState(false);
  const sectionRefs = useRef<Partial<Record<SectionId, HTMLElement | null>>>({});

  const visible = useMemo(
    () =>
      SECTIONS.map((section) => ({ ...section, settings: section.settings.filter((d) => matches(d, query)) })).filter(
        (s) => s.settings.length > 0 || (!query.trim() && s.id === "connection"),
      ),
    [query],
  );

  const jump = (id: SectionId) => {
    setCurrent(id);
    sectionRefs.current[id]?.scrollIntoView({ block: "start", behavior: "smooth" });
  };

  const resetSection = async (id: SectionId, title: string) => {
    try {
      // A null deletes the section from the file, so its defaults apply.
      await useSettingsStore.getState().update({ [id]: null } as unknown as SettingsPatch);
      toast.success(`${title} reset to defaults`);
    } catch (e) {
      toast.error("Couldn't reset", { description: errorMessage(e) });
    }
  };

  const resetAll = async () => {
    try {
      await api.resetSettings();
      toast.success("All settings reset to defaults");
    } catch (e) {
      toast.error("Couldn't reset", { description: errorMessage(e) });
    }
  };

  return (
    <main className="flex h-svh min-h-0 flex-col bg-background text-foreground">
      <header className="flex h-14 shrink-0 items-center gap-2 border-b px-4">
        <Button variant="ghost" size="icon" aria-label="Back" onClick={() => go({ page: "welcome" })}>
          <ChevronLeftIcon />
        </Button>
        <h1 className="text-lg font-semibold tracking-tight">Settings</h1>
        <Button variant="ghost" size="sm" className="ml-auto text-muted-foreground" onClick={() => setConfirmReset(true)}>
          <RotateCcwIcon data-icon="inline-start" />
          Reset all
        </Button>
      </header>

      {error ? (
        <div className="p-6">
          <Alert variant="destructive">
            <CircleAlertIcon />
            <AlertTitle>{error}</AlertTitle>
          </Alert>
        </div>
      ) : !settings ? (
        <div className="flex flex-1 items-center justify-center text-muted-foreground">
          <Spinner />
        </div>
      ) : (
        <div className="flex min-h-0 flex-1">
          <nav aria-label="Settings sections" className="flex w-60 shrink-0 flex-col gap-3 border-r p-3">
            <InputGroup className="h-8">
              <InputGroupAddon>
                <SearchIcon />
              </InputGroupAddon>
              <InputGroupInput
                aria-label="Search settings"
                placeholder="Search settings"
                value={query}
                onChange={(e) => setQuery(e.currentTarget.value)}
                autoFocus
              />
            </InputGroup>
            <ul className="flex flex-col gap-0.5">
              {SECTIONS.map((section) => {
                const count = visible.find((v) => v.id === section.id)?.settings.length ?? 0;
                const disabled = query.trim() !== "" && count === 0;
                return (
                  <li key={section.id}>
                    <button
                      type="button"
                      disabled={disabled}
                      aria-current={current === section.id ? "true" : undefined}
                      onClick={() => jump(section.id)}
                      className={cn(
                        "flex w-full items-center justify-between rounded-md px-2.5 py-1.5 text-left text-sm outline-none",
                        "hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-40",
                        current === section.id && !query.trim() && "bg-muted font-medium",
                      )}
                    >
                      {section.title}
                      {query.trim() && count > 0 && (
                        <span className="text-xs text-muted-foreground tabular-nums">{count}</span>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
            <p className="mt-auto px-2.5 text-xs text-muted-foreground">Changes save as you make them.</p>
          </nav>

          <ScrollArea className="min-h-0 flex-1">
            <div className="mx-auto flex w-full max-w-[720px] flex-col gap-10 px-8 py-6">
              {visible.length === 0 && (
                <p className="py-12 text-center text-sm text-muted-foreground">No setting matches “{query.trim()}”.</p>
              )}
              {visible.map((section) => (
                <section
                  key={section.id}
                  ref={(el) => {
                    sectionRefs.current[section.id] = el;
                  }}
                  aria-labelledby={`section-${section.id}`}
                  className="scroll-mt-6"
                  onFocusCapture={() => setCurrent(section.id)}
                >
                  <div className="mb-1 flex items-baseline justify-between gap-4">
                    <div>
                      <h2 id={`section-${section.id}`} className="text-base font-semibold tracking-tight">
                        {section.title}
                      </h2>
                      <p className="text-[13px] text-muted-foreground">{section.description}</p>
                    </div>
                    {!query.trim() && (
                      <Button
                        variant="ghost"
                        size="xs"
                        className="text-muted-foreground"
                        onClick={() => void resetSection(section.id, section.title)}
                      >
                        Reset section
                      </Button>
                    )}
                  </div>
                  {section.id === "general" && !query.trim() && <DisplayPreview general={settings.general} />}
                  <div>
                    {section.settings.map((def) => (
                      <SettingRow key={def.id} id={`setting-${def.id}`} label={def.label} description={def.description}>
                        {def.render(settings as Settings, `setting-${def.id}`)}
                      </SettingRow>
                    ))}
                    {section.id === "connection" && !query.trim() && <SecurityExtras />}
                  </div>
                </section>
              ))}
            </div>
          </ScrollArea>
        </div>
      )}

      <AlertDialog open={confirmReset} onOpenChange={setConfirmReset}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Reset all settings?</AlertDialogTitle>
            <AlertDialogDescription>
              Every setting goes back to its default. Saved connections and passwords aren't affected.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                setConfirmReset(false);
                void resetAll();
              }}
            >
              Reset all
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </main>
  );
}
