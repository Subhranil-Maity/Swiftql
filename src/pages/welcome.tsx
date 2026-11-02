import { useEffect, useId, useMemo, useRef, useState } from "react";
import { PlusIcon, SearchIcon, SettingsIcon } from "lucide-react";

import { PageShell } from "@/components/page-shell";
import { ThemeToggle } from "@/components/theme-toggle";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
} from "@/components/ui/empty";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { ConnectionList } from "@/features/connections/components/connection-list";
import { DeleteConnectionDialog } from "@/features/connections/components/delete-connection-dialog";
import { PasswordPromptDialog } from "@/features/connections/components/password-prompt-dialog";
import { useConnect } from "@/features/connections/hooks/use-connect";
import { formatTarget } from "@/lib/connection-format";
import type { SavedConnection } from "@/lib/ipc";
import { useConnectionsStore } from "@/stores/connections";
import { useNavigationStore } from "@/stores/navigation";

/** Connects once to the connection a "Save and connect" sent us here with. */
function useConnectOnArrival(connect: (c: SavedConnection) => Promise<void>) {
  const view = useNavigationStore((s) => s.view);
  const go = useNavigationStore((s) => s.go);
  const connections = useConnectionsStore((s) => s.connections);
  const pending = view.page === "welcome" ? view.connectId : undefined;
  // StrictMode runs effects twice; never connect twice for one arrival.
  const handled = useRef<string | null>(null);

  useEffect(() => {
    if (!pending || handled.current === pending) return;
    handled.current = pending;
    go({ page: "welcome" });
    const connection = connections.find((c) => c.id === pending);
    if (connection) void connect(connection);
  }, [pending, connections, connect, go]);
}

export function WelcomePage() {
  const { connections, loaded, error } = useConnectionsStore();
  const go = useNavigationStore((s) => s.go);
  const { connect, connectingId, prompt, submitPassword, cancelPrompt } = useConnect();
  const [query, setQuery] = useState("");
  const [deleting, setDeleting] = useState<SavedConnection | null>(null);
  const listLabel = useId();

  useConnectOnArrival(connect);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return connections;
    return connections.filter((c) =>
      [c.name, formatTarget(c.target, c.driver)].some((v) => v.toLowerCase().includes(q)),
    );
  }, [connections, query]);

  const newConnection = () => go({ page: "connection", editingId: null });

  return (
    <PageShell
      title={<span className="text-[28px] leading-none">SwiftQL</span>}
      actions={
        <>
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="Settings"
                  onClick={() => go({ page: "settings" })}
                />
              }
            >
              <SettingsIcon />
            </TooltipTrigger>
            <TooltipContent>Settings</TooltipContent>
          </Tooltip>
          <ThemeToggle />
        </>
      }
    >
      <div className="flex gap-2">
        <InputGroup>
          <InputGroupInput
            aria-label="Search connections"
            placeholder="Search connections"
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
          />
          <InputGroupAddon>
            <SearchIcon />
          </InputGroupAddon>
        </InputGroup>
        <Button onClick={newConnection}>
          <PlusIcon data-icon="inline-start" />
          New connection
        </Button>
      </div>

      <section className="flex flex-col gap-2">
        <h2 id={listLabel} className="font-medium text-muted-foreground">
          Connections
        </h2>

        {error ? (
          <p role="alert" className="py-6 text-destructive">
            {error}
          </p>
        ) : !loaded ? null : connections.length === 0 ? (
          <Empty className="border border-dashed">
            <EmptyHeader>
              <EmptyDescription>No saved connections. Add one to start querying.</EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button variant="outline" onClick={newConnection}>
                <PlusIcon data-icon="inline-start" />
                New connection
              </Button>
            </EmptyContent>
          </Empty>
        ) : filtered.length === 0 ? (
          <p className="py-6 text-muted-foreground">No connections match "{query}".</p>
        ) : (
          <ConnectionList
            connections={filtered}
            connectingId={connectingId}
            onConnect={(c) => void connect(c)}
            onEdit={(c) => go({ page: "connection", editingId: c.id })}
            onDelete={setDeleting}
            labelledBy={listLabel}
          />
        )}
      </section>

      <DeleteConnectionDialog connection={deleting} onClose={() => setDeleting(null)} />
      <PasswordPromptDialog
        key={prompt?.connection.id ?? "none"}
        prompt={prompt}
        onSubmit={(pw) => void submitPassword(pw)}
        onCancel={cancelPrompt}
      />
    </PageShell>
  );
}
