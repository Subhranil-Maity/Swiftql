import type { KeyboardEvent, Ref } from "react";
import { SearchIcon, XIcon } from "lucide-react";

import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import { cn } from "@/lib/utils";

/** A search box with a clear button; Esc clears it too (and blurs it when already empty). */
export function SearchInput({
  ref,
  value,
  onChange,
  placeholder,
  label,
  className,
  onKeyDown,
}: {
  ref?: Ref<HTMLInputElement>;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  /** Accessible name; defaults to the placeholder. */
  label?: string;
  className?: string;
  onKeyDown?: (event: KeyboardEvent<HTMLInputElement>) => void;
}) {
  return (
    <InputGroup className={cn("h-7 w-64", className)}>
      <InputGroupAddon>
        <SearchIcon />
      </InputGroupAddon>
      <InputGroupInput
        ref={ref}
        type="search"
        value={value}
        placeholder={placeholder}
        aria-label={label ?? placeholder}
        spellCheck={false}
        autoComplete="off"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            if (value) {
              event.preventDefault();
              event.stopPropagation();
              onChange("");
            } else {
              event.currentTarget.blur();
            }
          }
          onKeyDown?.(event);
        }}
        // The native clear button duplicates ours.
        className="[&::-webkit-search-cancel-button]:hidden"
      />
      {value && (
        <InputGroupAddon align="inline-end">
          <InputGroupButton size="icon-xs" aria-label="Clear search" onClick={() => onChange("")}>
            <XIcon />
          </InputGroupButton>
        </InputGroupAddon>
      )}
    </InputGroup>
  );
}
