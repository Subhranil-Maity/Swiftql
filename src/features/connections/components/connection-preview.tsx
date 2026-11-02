import { toConnectionString } from "@/lib/connection-format";

import type { FormState } from "../model/form-state";
import { needsUser, toTarget } from "../model/form-state";

/** Live, read-only connection string for the form. Never shows the password. */
export function ConnectionPreview({ form }: { form: FormState }) {
  const preview = toConnectionString({
    driverId: form.driver,
    target: toTarget(form),
    user: needsUser(form.authKind) ? form.user.trim() : undefined,
    sslMode: form.method === "url" ? undefined : form.sslMode,
  });
  return (
    <output
      aria-label="Connection string preview"
      className="block truncate rounded-md bg-muted px-2.5 py-1.5 font-mono text-xs text-muted-foreground select-all"
      title={preview}
    >
      {preview}
    </output>
  );
}
