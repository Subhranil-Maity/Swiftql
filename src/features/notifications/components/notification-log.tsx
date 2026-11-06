import { useEffect, useRef } from "react";

import { cn } from "@/lib/utils";

import type { NotificationEntry } from "../stores/notifications";

function time(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const ms = String(d.getMilliseconds()).padStart(3, "0");
  return `${d.toLocaleTimeString(undefined, { hour12: false })}.${ms}`;
}

/** The live log, newest at the bottom; follows new entries while scrolled to the end. */
export function NotificationLog({
  entries,
  selectedId,
  onSelect,
}: {
  entries: NotificationEntry[];
  selectedId: number | null;
  onSelect: (id: number) => void;
}) {
  const scroller = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);

  useEffect(() => {
    const el = scroller.current;
    if (el && atEnd.current) el.scrollTop = el.scrollHeight;
  }, [entries]);

  if (entries.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center p-6 text-center text-sm text-muted-foreground">
        <p>
          Notifications appear here as they arrive. Try{" "}
          <code className="font-mono">NOTIFY my_channel, &apos;hello&apos;</code> from another session.
        </p>
      </div>
    );
  }

  return (
    <div
      ref={scroller}
      className="min-h-0 flex-1 overflow-auto font-mono text-xs"
      onScroll={(e) => {
        const el = e.currentTarget;
        atEnd.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
      }}
      role="log"
      aria-live="polite"
    >
      <table className="w-full border-collapse">
        <thead className="sticky top-0 bg-background text-left text-muted-foreground">
          <tr className="border-b">
            <th className="px-3 py-1.5 font-normal">Time</th>
            <th className="px-3 py-1.5 font-normal">Channel</th>
            <th className="w-full px-3 py-1.5 font-normal">Payload</th>
            <th className="px-3 py-1.5 font-normal">PID</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((e) => (
            <tr
              key={e.id}
              onClick={() => onSelect(e.id)}
              aria-selected={e.id === selectedId}
              className={cn(
                "cursor-default border-b border-border/50 hover:bg-muted/60",
                e.id === selectedId && "bg-muted",
              )}
            >
              <td className="px-3 py-1 whitespace-nowrap text-muted-foreground tabular-nums">
                {time(e.receivedAt)}
              </td>
              <td className="px-3 py-1 whitespace-nowrap text-[var(--tab-accent)]">{e.channel}</td>
              <td className="max-w-0 truncate px-3 py-1">
                {e.payload || <span className="text-muted-foreground italic">empty</span>}
              </td>
              <td className="px-3 py-1 text-muted-foreground tabular-nums">{e.processId}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
