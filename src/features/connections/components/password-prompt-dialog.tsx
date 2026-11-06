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
import { SECRET_LABELS } from "@/features/connections/model/connect-flow";

import type { SecretPrompt } from "../hooks/use-secret-flow";

/** Asks once for a password, SSH password or key passphrase that isn't saved. */
export function PasswordPromptDialog({
  prompt,
  title,
  description,
  action = "Connect",
  onSubmit,
  onCancel,
}: {
  prompt: SecretPrompt | null;
  /** e.g. "Connect to Production". */
  title: string;
  /** e.g. "alice@db.example.com:5432". */
  description?: string;
  action?: string;
  onSubmit: (value: string) => void;
  onCancel: () => void;
}) {
  const id = useId();
  const [value, setValue] = useState("");
  const labels = prompt ? SECRET_LABELS[prompt.secret] : SECRET_LABELS.password;

  return (
    <Dialog
      open={prompt !== null}
      onOpenChange={(open) => {
        if (!open) {
          setValue("");
          onCancel();
        }
      }}
    >
      <DialogContent>
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            onSubmit(value);
            setValue("");
          }}
        >
          <DialogHeader>
            <DialogTitle className="text-[15px]">{title}</DialogTitle>
            <DialogDescription className="text-xs">
              {labels.title}
              {description && <span className="ml-1 font-mono">· {description}</span>}
            </DialogDescription>
          </DialogHeader>
          <Field data-invalid={!!prompt?.error}>
            <FieldLabel htmlFor={`${id}-secret`}>{labels.label}</FieldLabel>
            <Input
              id={`${id}-secret`}
              type="password"
              value={value}
              onChange={(e) => setValue(e.currentTarget.value)}
              aria-invalid={!!prompt?.error}
              autoComplete={labels.autoComplete}
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
              {action}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
