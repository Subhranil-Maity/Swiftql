import { ShieldAlertIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";

import type { HostKeyPrompt } from "../hooks/use-secret-flow";

/** An SSH server SwiftQL hasn't seen: show its fingerprint and let the user decide. */
export function HostKeyDialog({
  prompt,
  onTrust,
  onCancel,
}: {
  prompt: HostKeyPrompt | null;
  onTrust: () => void;
  onCancel: () => void;
}) {
  const info = prompt?.info;
  const where = info ? (info.port === 22 ? info.host : `${info.host}:${info.port}`) : "";
  return (
    <Dialog open={prompt !== null} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-[15px]">
            <ShieldAlertIcon className="size-4 text-warning" />
            Unknown SSH host
          </DialogTitle>
          <DialogDescription>
            This is the first connection to <span className="font-mono text-foreground">{where}</span>. Check
            that the fingerprint matches the server's before trusting it.
          </DialogDescription>
        </DialogHeader>
        {info && (
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 rounded-md border bg-muted/40 p-3 text-xs">
            <dt className="text-muted-foreground">Key type</dt>
            <dd className="font-mono">{info.keyType}</dd>
            <dt className="text-muted-foreground">Fingerprint</dt>
            <dd className="font-mono break-all select-all">{info.fingerprint}</dd>
          </dl>
        )}
        <p className="text-xs text-muted-foreground">
          On the server, <code className="font-mono">ssh-keygen -lf /etc/ssh/ssh_host_*_key.pub</code> prints it.
          Trusted hosts are saved in SwiftQL only; ~/.ssh/known_hosts isn't changed.
        </p>
        <DialogFooter>
          <Button variant="ghost" onClick={onCancel} disabled={prompt?.submitting}>
            Cancel
          </Button>
          <Button onClick={onTrust} disabled={prompt?.submitting}>
            {prompt?.submitting && <Spinner data-icon="inline-start" />}
            Trust and connect
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
