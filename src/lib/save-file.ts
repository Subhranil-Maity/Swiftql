// Saving an export to a file the user picks. The backend opens the native
// "Save as" dialog and writes the bytes, so the webview never picks a path.
import { saveExport, type SaveFilter } from "@/lib/ipc";

export type { SaveFilter };

/**
 * Asks where to save, then writes `contents` there. Resolves with the path,
 * or null if the dialog was cancelled. Throws `IpcError` if writing fails.
 */
export async function saveFile(
  defaultName: string,
  contents: string | Uint8Array,
  filters: SaveFilter[],
): Promise<string | null> {
  const bytes = typeof contents === "string" ? new TextEncoder().encode(contents) : contents;
  return saveExport(defaultName, bytes, filters);
}

/** The bytes behind a `data:` URL, e.g. from html-to-image. */
export function dataUrlBytes(dataUrl: string): Uint8Array {
  const comma = dataUrl.indexOf(",");
  const meta = dataUrl.slice(0, comma);
  const body = dataUrl.slice(comma + 1);
  if (meta.endsWith(";base64")) {
    const binary = atob(body);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
  }
  return new TextEncoder().encode(decodeURIComponent(body));
}

/** Safe as a file name on every OS: "public.orders" stays, "a/b:c" → "a_b_c". */
export function fileSafe(name: string): string {
  return name.replace(/[<>:"/\|?*\u0000-\u001f]+/g, "_").slice(0, 120) || "export";
}
