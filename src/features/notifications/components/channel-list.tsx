import { PlusIcon, RadioTowerIcon, XIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { errorMessage } from "@/lib/ipc-error";

/** Channels the listener is subscribed to, with add and remove. */
export function ChannelList({
  channels,
  onListen,
  onUnlisten,
}: {
  channels: string[];
  onListen: (channel: string) => Promise<void>;
  onUnlisten: (channel: string) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);

  const add = async () => {
    const channel = name.trim();
    if (!channel) return;
    setBusy(true);
    try {
      await onListen(channel);
      setName("");
    } catch (error) {
      toast.error(`Couldn't listen on ${channel}`, { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <form
        className="flex gap-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          void add();
        }}
      >
        <Input
          aria-label="Channel to listen on"
          placeholder="Channel name"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          spellCheck={false}
          className="h-8 font-mono"
        />
        <Button type="submit" size="sm" variant="outline" disabled={busy || !name.trim()}>
          <PlusIcon data-icon="inline-start" />
          Listen
        </Button>
      </form>
      {channels.length === 0 ? (
        <p className="text-xs text-muted-foreground">Not listening yet. LISTEN run in the console shows here too.</p>
      ) : (
        <ul className="flex flex-col gap-0.5" aria-label="Channels">
          {channels.map((channel) => (
            <li key={channel} className="group flex items-center gap-2 rounded-md px-2 py-1 hover:bg-muted">
              <RadioTowerIcon className="size-3.5 text-[var(--tab-accent)]" />
              <span className="min-w-0 flex-1 truncate font-mono text-sm">{channel}</span>
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label={`Stop listening on ${channel}`}
                className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
                onClick={() =>
                  void onUnlisten(channel).catch((error: unknown) =>
                    toast.error(`Couldn't stop listening on ${channel}`, { description: errorMessage(error) }),
                  )
                }
              >
                <XIcon />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
