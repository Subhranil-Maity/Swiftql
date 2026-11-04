import { type CSSProperties, lazy, type ReactNode, Suspense, useState } from "react";

import { Spinner } from "@/components/ui/spinner";
import { colorVar } from "@/features/connections/model/colors";
import { ConsoleView } from "@/features/console/components/console-view";
import { TablesView } from "@/features/tables/components/tables-view";
import { useWorkspace } from "@/features/workspace/model/workspace-context";
import { cn } from "@/lib/utils";
import { type WorkspaceView, useWorkspaceView } from "@/stores/workspace-view";

// React Flow and elk are large; load them the first time the graph opens.
const GraphView = lazy(() =>
  import("@/features/graph/components/graph-view").then((m) => ({ default: m.GraphView })),
);

function Loading() {
  return (
    <div className="flex flex-1 items-center justify-center">
      <Spinner className="text-muted-foreground" />
    </div>
  );
}

/**
 * The console, Tables and Graph views. Each mounts the first time it's shown
 * and then stays mounted but hidden, so editor undo history and the graph's
 * viewport survive switching.
 */
export function WorkspaceViews() {
  const workspace = useWorkspace();
  const current = useWorkspaceView(workspace.connectionId);
  // Views opened at least once; updated during render so the new view mounts in the same pass.
  const [opened, setOpened] = useState<ReadonlySet<WorkspaceView>>(() => new Set([current]));
  if (!opened.has(current)) setOpened(new Set(opened).add(current));

  const pane = (view: WorkspaceView, label: string, children: ReactNode) =>
    opened.has(view) && (
      <section
        aria-label={label}
        hidden={current !== view}
        className={cn("min-h-0 min-w-0 flex-1 flex-col", current === view && "flex")}
      >
        {children}
      </section>
    );

  return (
    // --tab-accent: the connection colour, the one accent every view uses.
    <div
      className="flex min-h-0 flex-1 flex-col"
      style={{ "--tab-accent": colorVar(workspace.color) } as CSSProperties}
    >
      {pane(
        "console",
        "Console",
        <ConsoleView
          connectionId={workspace.connectionId}
          sessionId={workspace.sessionId}
          driverId={workspace.driverId}
          color={workspace.color}
          active={current === "console"}
        />,
      )}
      {pane("tables", "Tables", <TablesView />)}
      {pane(
        "graph",
        "Graph",
        <Suspense fallback={<Loading />}>
          <GraphView active={current === "graph"} />
        </Suspense>,
      )}
    </div>
  );
}
