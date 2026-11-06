import { FolderOpenIcon } from "lucide-react";
import { toast } from "sonner";

import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from "@/components/ui/input-group";
import { pickFile } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

export type FileFilter = { name: string; extensions: string[] };

export const PEM_FILTERS: FileFilter[] = [
  { name: "PEM certificate or key", extensions: ["pem", "crt", "cer", "key"] },
  { name: "All files", extensions: ["*"] },
];

/** A path typed or picked with the native "Open" dialog. */
export function FilePathInput({
  id,
  value,
  onChange,
  placeholder,
  filters,
  invalid,
}: {
  id: string;
  value: string;
  onChange: (path: string) => void;
  placeholder?: string;
  filters?: FileFilter[];
  invalid?: boolean;
}) {
  const browse = async () => {
    try {
      const path = await pickFile(filters);
      if (path) onChange(path);
    } catch (error) {
      toast.error("Couldn't open the file picker", { description: errorMessage(error) });
    }
  };
  return (
    <InputGroup>
      <InputGroupInput
        id={id}
        value={value}
        onChange={(e) => onChange(e.currentTarget.value)}
        placeholder={placeholder}
        aria-invalid={invalid}
        spellCheck={false}
        className="font-mono"
      />
      <InputGroupAddon align="inline-end">
        <InputGroupButton size="icon-xs" aria-label="Browse" onClick={() => void browse()}>
          <FolderOpenIcon />
        </InputGroupButton>
      </InputGroupAddon>
    </InputGroup>
  );
}
