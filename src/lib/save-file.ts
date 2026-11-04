// Saving an export to a file the user picks: the native "Save as" dialog,
// then the backend writes the bytes.
import { api, showSaveDialog } from "@/lib/ipc";

export type SaveFilter = { name: string; extensions: string[] };

/**
 * Asks where to save, then writes `contents` there. Resolves with the path,
 * or null if the dialog was cancelled. Throws `IpcError` if writing fails.
 */
export async function saveFile(
  defaultName: string,
  contents: string | Uint8Array,
  filters: SaveFilter[],
): Promise<string | null> {
  const path = await showSaveDialog({ defaultPath: defaultName, filters });
  if (!path) return null;
  const bytes = typeof contents === "string" ? new TextEncoder().encode(contents) : contents;
  await api.writeExportFile(path, Array.from(bytes));
  return path;
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
