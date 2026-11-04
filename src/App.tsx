import { useEffect } from "react";

import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { ConnectionPage } from "@/pages/connection-page";
import { SettingsPage } from "@/pages/settings-page";
import { WelcomePage } from "@/pages/welcome";
import { WorkspacePage } from "@/pages/workspace";
import { useConnectionsStore } from "@/stores/connections";
import { useDriversStore } from "@/stores/drivers";
import { useHealthStore } from "@/stores/health";
import { useNavigationStore } from "@/stores/navigation";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";

/** Loads app data once, then runs the startup health checks. */
function useStartup() {
  const loadConnections = useConnectionsStore((s) => s.load);
  const loadDrivers = useDriversStore((s) => s.load);
  const loadSettings = useSettingsStore((s) => s.load);
  const connectionsLoaded = useConnectionsStore((s) => s.loaded);
  const settingsLoaded = useSettingsStore((s) => s.settings !== null);
  const checkRecent = useHealthStore((s) => s.checkRecent);

  useEffect(() => {
    void loadConnections();
    void loadDrivers();
    void loadSettings();
  }, [loadConnections, loadDrivers, loadSettings]);

  // checkRecent runs once per launch however often this fires.
  useEffect(() => {
    if (connectionsLoaded && settingsLoaded) checkRecent();
  }, [connectionsLoaded, settingsLoaded, checkRecent]);
}

function Page() {
  const view = useNavigationStore((s) => s.view);
  switch (view.page) {
    case "connection":
      // A fresh key per connection resets the page's step and form.
      return <ConnectionPage key={view.editingId ?? "new"} editingId={view.editingId} />;
    case "settings":
      return <SettingsPage />;
    case "welcome":
      return <WelcomePage />;
  }
}

function App() {
  const session = useSessionsStore((s) =>
    s.activeSessionId ? s.sessions[s.activeSessionId] : undefined,
  );
  // Settings opens over the workspace too; its Back returns there.
  const showSettings = useNavigationStore((s) => s.view.page === "settings");
  useStartup();

  return (
    <TooltipProvider>
      {session && !showSettings ? <WorkspacePage session={session} /> : <Page />}
      <Toaster position="bottom-right" />
    </TooltipProvider>
  );
}

export default App;
