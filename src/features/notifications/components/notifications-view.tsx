import { PauseIcon, PlayIcon, SearchIcon, Trash2Icon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group";
import { Separator } from "@/components/ui/separator";
import { useWorkspace } from "@/features/workspace/model/workspace-context";
import { plural } from "@/lib/format-count";

import { type SessionNotifications, useNotificationsStore } from "../stores/notifications";
import { ChannelList } from "./channel-list";
import { NotificationDetail } from "./notification-detail";
import { NotificationLog } from "./notification-log";
import { SendNotifyForm } from "./send-notify-form";

const EMPTY: SessionNotifications = { entries: [], channels: [], unread: 0, dropped: 0, paused: false };

/** LISTEN / NOTIFY: channels on the left, the live log on the right. */
export function NotificationsView({ active }: { active: boolean }) {
  const { sessionId } = useWorkspace();
  const state = useNotificationsStore((s) => s.bySession[sessionId] ?? EMPTY);
  const { listen, unlisten, send, setPaused, clear, setWatching, refreshChannels } = useNotificationsStore.getState();
  const [filter, setFilter] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);

  useEffect(() => {
    setWatching(sessionId, active);
    return () => setWatching(sessionId, false);
  }, [sessionId, active, setWatching]);

  useEffect(() => {
    void refreshChannels(sessionId).catch(() => {});
  }, [sessionId, refreshChannels]);

  const shown = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return state.entries;
    return state.entries.filter((e) => e.channel.toLowerCase().includes(q) || e.payload.toLowerCase().includes(q));
  }, [state.entries, filter]);
  const selected = state.entries.find((e) => e.id === selectedId) ?? null;

  return (
    <div className="flex min-h-0 flex-1">
      <aside className="flex w-64 shrink-0 flex-col gap-4 overflow-auto border-r p-4">
        <div>
          <h2 className="text-sm font-medium">Channels</h2>
          <p className="text-xs text-muted-foreground">On a separate connection, so console queries never block it.</p>
        </div>
        <ChannelList
          channels={state.channels}
          onListen={(c) => listen(sessionId, c)}
          onUnlisten={(c) => unlisten(sessionId, c)}
        />
        <Separator />
        <SendNotifyForm defaultChannel={state.channels[0] ?? ""} onSend={(c, p) => send(sessionId, c, p)} />
      </aside>
      <section className="flex min-w-0 flex-1 flex-col">
        <div className="flex h-10 shrink-0 items-center gap-2 border-b px-3">
          <InputGroup className="h-7 max-w-64">
            <InputGroupAddon>
              <SearchIcon />
            </InputGroupAddon>
            <InputGroupInput
              aria-label="Filter notifications"
              placeholder="Filter channel or payload"
              value={filter}
              onChange={(e) => setFilter(e.currentTarget.value)}
            />
          </InputGroup>
          <span className="text-xs text-muted-foreground tabular-nums">
            {plural(state.entries.length, "notification")}
            {state.dropped > 0 && ` · ${state.dropped.toLocaleString()} dropped`}
          </span>
          <div className="ml-auto flex items-center gap-1">
            <Button size="xs" variant="ghost" onClick={() => setPaused(sessionId, !state.paused)}>
              {state.paused ? <PlayIcon data-icon="inline-start" /> : <PauseIcon data-icon="inline-start" />}
              {state.paused ? "Resume" : "Pause"}
            </Button>
            <Button
              size="xs"
              variant="ghost"
              disabled={state.entries.length === 0}
              onClick={() => {
                clear(sessionId);
                setSelectedId(null);
              }}
            >
              <Trash2Icon data-icon="inline-start" />
              Clear
            </Button>
          </div>
        </div>
        <NotificationLog entries={shown} selectedId={selectedId} onSelect={setSelectedId} />
        {selected && <NotificationDetail entry={selected} />}
      </section>
    </div>
  );
}
