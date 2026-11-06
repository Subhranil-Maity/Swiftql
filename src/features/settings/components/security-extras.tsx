import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { api, type KeyringStatus } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { cn } from "@/lib/utils";

import { SettingRow } from "./setting-row";

/** Keyring availability and forgetting trusted SSH hosts. */
export function SecurityExtras() {
  const [keyring, setKeyring] = useState<KeyringStatus | null>(null);
  const [clearing, setClearing] = useState(false);

  useEffect(() => {
    api
      .keyringStatus()
      .then(setKeyring)
      .catch((error: unknown) => setKeyring({ available: false, message: errorMessage(error) }));
  }, []);

  const clearHosts = async () => {
    setClearing(true);
    try {
      await api.clearTrustedSshHosts();
      toast.success("Trusted SSH hosts forgotten", {
        description: "You'll be asked to check each host's key again.",
      });
    } catch (error) {
      toast.error("Couldn't forget trusted SSH hosts", { description: errorMessage(error) });
    } finally {
      setClearing(false);
    }
  };

  return (
    <>
      <SettingRow
        id="keyring-status"
        label="System keychain"
        description={
          keyring?.available === false
            ? (keyring.message ?? "Passwords can't be saved; you'll be asked for them when connecting.")
            : "Saved passwords and SSH passphrases are stored here, never in SwiftQL's files."
        }
      >
        <span
          id="keyring-status"
          className={cn(
            "inline-flex items-center gap-1.5 text-sm",
            keyring?.available === false ? "text-destructive" : "text-muted-foreground",
          )}
        >
          <span
            aria-hidden
            className={cn(
              "size-2 rounded-full",
              keyring === null ? "bg-muted-foreground/40" : keyring.available ? "bg-success" : "bg-destructive",
            )}
          />
          {keyring === null ? "Checking…" : keyring.available ? "Available" : "Unavailable"}
        </span>
      </SettingRow>
      <SettingRow
        id="clear-ssh-hosts"
        label="Trusted SSH hosts"
        description="Host keys you trusted in SwiftQL. Your ~/.ssh/known_hosts isn't touched."
      >
        <Button id="clear-ssh-hosts" variant="outline" size="sm" disabled={clearing} onClick={() => void clearHosts()}>
          Forget all
        </Button>
      </SettingRow>
    </>
  );
}
