import { useId, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { formatTarget } from "@/lib/connection-format";

import type { PasswordPrompt } from "../hooks/use-connect";

/** Asks for a password once, for a connection that doesn't save it. */
export function PasswordPromptDialog({
  prompt,
  onSubmit,
  onCancel,
}: {
  prompt: PasswordPrompt | null;
  onSubmit: (password: string) => void;
  onCancel: () => void;
}) {
  const id = useId();
  const [password, setPassword] = useState("");
  const connection = prompt?.connection;
  const user = connection && "user" in connection.auth ? connection.auth.user : null;

  return (
    <Dialog
      open={prompt !== null}
      onOpenChange={(open) => {
        if (!open) {
          setPassword("");
          onCancel();
        }
      }}
    >
      <DialogContent>
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            onSubmit(password);
          }}
        >
          <DialogHeader>
            <DialogTitle className="text-[15px]">Connect to {connection?.name}</DialogTitle>
            {connection && (
              <DialogDescription className="font-mono text-xs">
                {user ? `${user}@` : ""}
                {formatTarget(connection.target, connection.driver)}
              </DialogDescription>
            )}
          </DialogHeader>
          <Field data-invalid={!!prompt?.error}>
            <FieldLabel htmlFor={`${id}-pw`}>Password</FieldLabel>
            <Input
              id={`${id}-pw`}
              type="password"
              value={password}
              onChange={(e) => setPassword(e.currentTarget.value)}
              aria-invalid={!!prompt?.error}
              autoComplete="current-password"
              autoFocus
            />
            <FieldError>{prompt?.error}</FieldError>
          </Field>
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={onCancel} disabled={prompt?.submitting}>
              Cancel
            </Button>
            <Button type="submit" disabled={prompt?.submitting}>
              {prompt?.submitting && <Spinner data-icon="inline-start" />}
              Connect
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
