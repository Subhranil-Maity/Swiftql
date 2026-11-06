// Controls for one setting each. They save on change (switches, selects) or
// on blur / Enter (text and numbers); the backend validates and an invalid
// value shows under the control.
import { useEffect, useId, useState } from "react";

import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";

import { useSaveSetting } from "../hooks/use-save-setting";
import type { SettingsPatch } from "@/stores/settings";

export function SwitchControl({
  id,
  checked,
  patch,
}: {
  id: string;
  checked: boolean;
  patch: (value: boolean) => SettingsPatch;
}) {
  const { save } = useSaveSetting();
  return <Switch id={id} checked={checked} onCheckedChange={(v) => void save(patch(v))} />;
}

export function SelectControl<T extends string>({
  id,
  value,
  options,
  patch,
  className,
}: {
  id: string;
  value: T;
  options: { value: T; label: string }[];
  patch: (value: T) => SettingsPatch;
  className?: string;
}) {
  const { save } = useSaveSetting();
  return (
    <Select items={options} value={value} onValueChange={(v) => v && void save(patch(v as T))}>
      <SelectTrigger id={id} className={cn("w-48", className)}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {options.map((o) => (
            <SelectItem key={o.value} value={o.value}>
              {o.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}

/** A draft that follows the saved value and commits on blur or Enter. */
function useDraft(saved: string, commit: (text: string) => Promise<string | null>) {
  const [draft, setDraft] = useState(saved);
  const [error, setError] = useState<string | null>(null);
  // Follow changes saved elsewhere (another window, a reset).
  useEffect(() => setDraft(saved), [saved]);
  const submit = async () => {
    if (draft === saved) return setError(null);
    setError(await commit(draft));
  };
  return { draft, setDraft, error, submit };
}

export function NumberControl({
  id,
  value,
  min,
  max,
  unit,
  patch,
}: {
  id: string;
  value: number;
  min: number;
  max: number;
  /** Shown after the input, e.g. "seconds". */
  unit?: string;
  patch: (value: number) => SettingsPatch;
}) {
  const { saveOrError } = useSaveSetting();
  const errorId = useId();
  const { draft, setDraft, error, submit } = useDraft(String(value), async (text) => {
    const t = text.trim();
    if (!/^\d+$/.test(t) || Number(t) < min || Number(t) > max) {
      return `Enter a whole number from ${min.toLocaleString()} to ${max.toLocaleString()}.`;
    }
    return saveOrError(patch(Number(t)));
  });
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-2">
        <Input
          id={id}
          inputMode="numeric"
          value={draft}
          onChange={(e) => setDraft(e.currentTarget.value)}
          onBlur={() => void submit()}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void submit();
            }
          }}
          aria-invalid={!!error}
          aria-describedby={error ? errorId : undefined}
          className="w-28 text-right font-mono tabular-nums"
        />
        {unit && <span className="w-14 text-xs text-muted-foreground">{unit}</span>}
      </div>
      {error && (
        <p id={errorId} className="max-w-56 text-right text-xs text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}

export function TextControl({
  id,
  value,
  placeholder,
  mono,
  patch,
  className,
}: {
  id: string;
  value: string;
  placeholder?: string;
  mono?: boolean;
  patch: (value: string) => SettingsPatch;
  className?: string;
}) {
  const { saveOrError } = useSaveSetting();
  const errorId = useId();
  const { draft, setDraft, error, submit } = useDraft(value, (text) => saveOrError(patch(text)));
  return (
    <div className="flex flex-col items-end gap-1">
      <Input
        id={id}
        value={draft}
        placeholder={placeholder}
        onChange={(e) => setDraft(e.currentTarget.value)}
        onBlur={() => void submit()}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void submit();
          }
        }}
        aria-invalid={!!error}
        aria-describedby={error ? errorId : undefined}
        spellCheck={false}
        className={cn("w-56", mono && "font-mono", className)}
      />
      {error && (
        <p id={errorId} className="max-w-56 text-right text-xs text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
