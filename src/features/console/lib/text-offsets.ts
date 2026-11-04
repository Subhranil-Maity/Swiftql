// Conversions between the offsets parsers and servers report and the UTF-16
// offsets JavaScript strings and CodeMirror use.

/** Postgres counts characters (code points); CodeMirror offsets are UTF-16 units. */
export function codePointToOffset(text: string, codePoints: number): number {
  let offset = 0;
  for (let i = 0; i < codePoints && offset < text.length; i++) {
    offset += (text.codePointAt(offset) ?? 0) > 0xffff ? 2 : 1;
  }
  return offset;
}

/** Bytes `cp` takes in UTF-8. */
function utf8Length(cp: number): number {
  if (cp < 0x80) return 1;
  if (cp < 0x800) return 2;
  if (cp < 0x10000) return 3;
  return 4;
}

/**
 * A function from UTF-8 byte offsets in `text` to UTF-16 offsets. libpg-query
 * reports byte offsets. A byte inside a character maps to that character's
 * start; offsets past the end map to `text.length`.
 */
export function utf8ToUtf16(text: string): (byte: number) => number {
  // ASCII is the common case and needs no table.
  if (!/[^\x00-\x7f]/.test(text)) return (byte) => Math.min(Math.max(byte, 0), text.length);

  const starts: number[] = []; // starts[byte] = UTF-16 offset of the char holding that byte
  let offset = 0;
  while (offset < text.length) {
    const cp = text.codePointAt(offset) ?? 0;
    for (let i = utf8Length(cp); i > 0; i--) starts.push(offset);
    offset += cp > 0xffff ? 2 : 1;
  }
  return (byte) => (byte >= starts.length ? text.length : starts[Math.max(byte, 0)]);
}
