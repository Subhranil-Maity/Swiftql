import { SendIcon } from "lucide-react";
import { useId, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { errorMessage } from "@/lib/ipc-error";

/** NOTIFY a channel, e.g. to test a listener. Ctrl+Enter sends. */
export function SendNotifyForm({
  defaultChannel,
  onSend,
}: {
  defaultChannel: string;
  onSend: (channel: string, payload: string) => Promise<void>;
}) {
  const id = useId();
  const [channel, setChannel] = useState("");
  const [payload, setPayload] = useState("");
  const [busy, setBusy] = useState(false);
  const target = channel.trim() || defaultChannel;

  const send = async () => {
    if (!target) return;
    setBusy(true);
    try {
      await onSend(target, payload);
      toast.success(`Sent to ${target}`);
    } catch (error) {
      toast.error("Couldn't send the notification", { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      className="flex flex-col gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        void send();
      }}
    >
      <Field>
        <FieldLabel htmlFor={`${id}-channel`} className="text-xs">
          Send NOTIFY
        </FieldLabel>
        <Input
          id={`${id}-channel`}
          placeholder={defaultChannel || "Channel"}
          value={channel}
          onChange={(e) => setChannel(e.currentTarget.value)}
          spellCheck={false}
          className="h-8 font-mono"
        />
      </Field>
      <Textarea
        aria-label="Payload"
        placeholder='Payload, e.g. {"id": 1}'
        value={payload}
        onChange={(e) => setPayload(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            void send();
          }
        }}
        spellCheck={false}
        className="min-h-16 font-mono text-xs"
      />
      <Button type="submit" size="sm" variant="outline" disabled={busy || !target}>
        <SendIcon data-icon="inline-start" />
        Send
      </Button>
    </form>
  );
}
