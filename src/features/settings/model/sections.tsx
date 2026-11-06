// Every setting the page shows, grouped by section. Rows are data, so the
// search can filter them. Bounds match `Settings::validate` in
// crates/config-store/src/settings.rs.
import type { ReactNode } from "react";

import { NumberControl, SelectControl, SwitchControl, TextControl } from "../components/setting-controls";
import type {
  BooleanDisplay,
  Density,
  HostKeyPolicy,
  KeywordCase,
  Settings,
  SslMode,
  TimestampDisplay,
  TransactionMode,
} from "@/lib/ipc";

export type SectionId = "general" | "editor" | "query" | "connection";

export type SettingDef = {
  id: string;
  label: string;
  description: string;
  /** Extra words the search matches. */
  keywords?: string;
  render: (settings: Settings, id: string) => ReactNode;
};

export type SectionDef = {
  id: SectionId;
  title: string;
  description: string;
  settings: SettingDef[];
};

const opts = <T extends string>(entries: [T, string][]) => entries.map(([value, label]) => ({ value, label }));

export const SECTIONS: SectionDef[] = [
  {
    id: "general",
    title: "Appearance",
    description: "How SwiftQL and result grids look.",
    settings: [
      {
        id: "theme",
        label: "Theme",
        description: "System follows your OS and switches with it.",
        keywords: "dark light mode color",
        render: (s, id) => (
          <SelectControl
            id={id}
            value={s.general.theme}
            options={opts<Settings["general"]["theme"]>([
              ["system", "System"],
              ["light", "Light"],
              ["dark", "Dark"],
            ])}
            patch={(theme) => ({ general: { theme } })}
          />
        ),
      },
      {
        id: "uiFontSize",
        label: "Interface text size",
        description: "Scales the whole interface; the editor and grid have their own sizes.",
        keywords: "font zoom scale",
        render: (s, id) => (
          <NumberControl id={id} value={s.general.uiFontSize} min={10} max={24} unit="px" patch={(uiFontSize) => ({ general: { uiFontSize } })} />
        ),
      },
      {
        id: "density",
        label: "Density",
        description: "Compact fits more on screen with tighter spacing.",
        keywords: "spacing compact padding",
        render: (s, id) => (
          <SelectControl
            id={id}
            value={s.general.density}
            options={opts<Density>([
              ["comfortable", "Comfortable"],
              ["compact", "Compact"],
            ])}
            patch={(density) => ({ general: { density } })}
          />
        ),
      },
      {
        id: "nullDisplay",
        label: "NULL shows as",
        description: "Text for NULL cells in result grids. Copies and exports are unaffected.",
        keywords: "null empty",
        render: (s, id) => (
          <TextControl id={id} value={s.general.nullDisplay} mono placeholder="(empty)" className="w-32" patch={(nullDisplay) => ({ general: { nullDisplay } })} />
        ),
      },
      {
        id: "booleanDisplay",
        label: "Booleans show as",
        description: "How true and false read in result grids.",
        keywords: "bool true false",
        render: (s, id) => (
          <SelectControl
            id={id}
            value={s.general.booleanDisplay}
            options={opts<BooleanDisplay>([
              ["text", "true / false"],
              ["short", "t / f"],
              ["symbol", "✓ / ✗"],
            ])}
            patch={(booleanDisplay) => ({ general: { booleanDisplay } })}
          />
        ),
      },
      {
        id: "timestampDisplay",
        label: "Timestamps with time zone",
        description: "As the server sends them (UTC), or in this computer's time zone.",
        keywords: "date time timezone timestamptz",
        render: (s, id) => (
          <SelectControl
            id={id}
            value={s.general.timestampDisplay}
            options={opts<TimestampDisplay>([
              ["iso", "UTC, as sent"],
              ["local", "Local time"],
            ])}
            patch={(timestampDisplay) => ({ general: { timestampDisplay } })}
          />
        ),
      },
      {
        id: "numberGrouping",
        label: "Thousands separators",
        description: "Group digits in numeric cells: 1,234,567.",
        keywords: "number format comma",
        render: (s, id) => <SwitchControl id={id} checked={s.general.numberGrouping} patch={(numberGrouping) => ({ general: { numberGrouping } })} />,
      },
    ],
  },
  {
    id: "editor",
    title: "Editor",
    description: "The SQL console's editor.",
    settings: [
      {
        id: "fontFamily",
        label: "Font",
        description: "Any installed monospace font; list fallbacks after commas.",
        keywords: "typeface family monospace",
        render: (s, id) => <TextControl id={id} value={s.editor.fontFamily} mono patch={(fontFamily) => ({ editor: { fontFamily } })} />,
      },
      {
        id: "fontSize",
        label: "Font size",
        description: "Text size in the editor.",
        render: (s, id) => <NumberControl id={id} value={s.editor.fontSize} min={6} max={72} unit="px" patch={(fontSize) => ({ editor: { fontSize } })} />,
      },
      {
        id: "tabSize",
        label: "Tab width",
        description: "Spaces per indent level.",
        keywords: "indent spaces",
        render: (s, id) => <NumberControl id={id} value={s.editor.tabSize} min={1} max={16} unit="spaces" patch={(tabSize) => ({ editor: { tabSize } })} />,
      },
      {
        id: "wordWrap",
        label: "Wrap long lines",
        description: "Instead of scrolling sideways.",
        keywords: "wrap",
        render: (s, id) => <SwitchControl id={id} checked={s.editor.wordWrap} patch={(wordWrap) => ({ editor: { wordWrap } })} />,
      },
      {
        id: "lineNumbers",
        label: "Line numbers",
        description: "In the gutter beside the text.",
        keywords: "gutter",
        render: (s, id) => <SwitchControl id={id} checked={s.editor.lineNumbers} patch={(lineNumbers) => ({ editor: { lineNumbers } })} />,
      },
      {
        id: "highlightActiveLine",
        label: "Highlight the current line",
        description: "Shades the line the cursor is on.",
        keywords: "cursor",
        render: (s, id) => (
          <SwitchControl id={id} checked={s.editor.highlightActiveLine} patch={(highlightActiveLine) => ({ editor: { highlightActiveLine } })} />
        ),
      },
      {
        id: "autocomplete",
        label: "Autocomplete",
        description: "Suggest keywords, tables and columns while typing.",
        keywords: "completion suggestions intellisense",
        render: (s, id) => <SwitchControl id={id} checked={s.editor.autocomplete} patch={(autocomplete) => ({ editor: { autocomplete } })} />,
      },
      {
        id: "keywordCase",
        label: "Keyword case",
        description: "How autocomplete inserts SQL keywords.",
        keywords: "uppercase lowercase",
        render: (s, id) => (
          <SelectControl
            id={id}
            value={s.editor.keywordCase}
            options={opts<KeywordCase>([
              ["upper", "SELECT"],
              ["lower", "select"],
            ])}
            patch={(keywordCase) => ({ editor: { keywordCase } })}
          />
        ),
      },
      {
        id: "lint",
        label: "Check syntax while typing",
        description: "Underlines statements the server would reject as invalid SQL.",
        keywords: "lint errors parser",
        render: (s, id) => <SwitchControl id={id} checked={s.editor.lint} patch={(lint) => ({ editor: { lint } })} />,
      },
    ],
  },
  {
    id: "query",
    title: "Query & results",
    description: "Running statements and holding their rows.",
    settings: [
      {
        id: "maxRowsHeld",
        label: "Rows kept per result",
        description: "Reading stops here and the rest of the result isn't fetched. 0 means no limit.",
        keywords: "limit memory rows",
        render: (s, id) => (
          <NumberControl id={id} value={s.query.maxRowsHeld} min={0} max={100_000_000} unit="rows" patch={(maxRowsHeld) => ({ query: { maxRowsHeld } })} />
        ),
      },
      {
        id: "maxResultMb",
        label: "Memory per result",
        description: "Also stops reading once a result is about this large.",
        keywords: "limit memory size",
        render: (s, id) => (
          <NumberControl id={id} value={s.query.maxResultMb} min={16} max={16_384} unit="MB" patch={(maxResultMb) => ({ query: { maxResultMb } })} />
        ),
      },
      {
        id: "pageSize",
        label: "Rows fetched at a time",
        description: "How many rows the grid loads as you scroll.",
        keywords: "page scroll batch",
        render: (s, id) => <NumberControl id={id} value={s.query.pageSize} min={50} max={10_000} unit="rows" patch={(pageSize) => ({ query: { pageSize } })} />,
      },
      {
        id: "queryTimeoutSecs",
        label: "Statement timeout",
        description: "The server cancels a statement that runs longer. 0 means no timeout.",
        keywords: "statement_timeout cancel slow",
        render: (s, id) => (
          <NumberControl id={id} value={s.query.queryTimeoutSecs} min={0} max={86_400} unit="seconds" patch={(queryTimeoutSecs) => ({ query: { queryTimeoutSecs } })} />
        ),
      },
      {
        id: "transactionMode",
        label: "Transactions",
        description: "Manual opens a transaction on the first run; commit or roll back from the header.",
        keywords: "autocommit commit rollback begin",
        render: (s, id) => (
          <SelectControl
            id={id}
            value={s.query.transactionMode}
            options={opts<TransactionMode>([
              ["auto", "Auto-commit"],
              ["manual", "Manual commit"],
            ])}
            patch={(transactionMode) => ({ query: { transactionMode } })}
          />
        ),
      },
      {
        id: "confirmDestructive",
        label: "Confirm destructive statements",
        description: "Ask before DROP, TRUNCATE, and DELETE or UPDATE without WHERE.",
        keywords: "drop delete truncate safety warn",
        render: (s, id) => (
          <SwitchControl id={id} checked={s.query.confirmDestructive} patch={(confirmDestructive) => ({ query: { confirmDestructive } })} />
        ),
      },
    ],
  },
  {
    id: "connection",
    title: "Connections & security",
    description: "Reaching servers, encryption and saved secrets.",
    settings: [
      {
        id: "connectTimeoutSecs",
        label: "Connect timeout",
        description: "Give up reaching a server (or SSH host) after this long.",
        keywords: "timeout network",
        render: (s, id) => (
          <NumberControl id={id} value={s.connection.connectTimeoutSecs} min={1} max={600} unit="seconds" patch={(connectTimeoutSecs) => ({ connection: { connectTimeoutSecs } })} />
        ),
      },
      {
        id: "keepaliveSecs",
        label: "Keepalive",
        description: "Keeps idle connections and SSH tunnels from being dropped. 0 turns it off.",
        keywords: "idle tcp",
        render: (s, id) => (
          <NumberControl id={id} value={s.connection.keepaliveSecs} min={0} max={3600} unit="seconds" patch={(keepaliveSecs) => ({ connection: { keepaliveSecs } })} />
        ),
      },
      {
        id: "checkOnStartup",
        label: "Check recent connections on startup",
        description: "How many recently used connections to test when SwiftQL opens. 0 turns it off.",
        keywords: "health status",
        render: (s, id) => (
          <NumberControl id={id} value={s.connection.checkOnStartup} min={0} max={50} patch={(checkOnStartup) => ({ connection: { checkOnStartup } })} />
        ),
      },
      {
        id: "defaultSslMode",
        label: "SSL for new connections",
        description: "The starting SSL mode in the connection form.",
        keywords: "tls encryption certificate",
        render: (s, id) => (
          <SelectControl
            id={id}
            value={s.connection.defaultSslMode}
            options={opts<SslMode>([
              ["disable", "Disable"],
              ["prefer", "Prefer"],
              ["require", "Require"],
              ["verifyCa", "Verify CA"],
              ["verifyFull", "Verify full"],
            ])}
            patch={(defaultSslMode) => ({ connection: { defaultSslMode } })}
          />
        ),
      },
      {
        id: "sshHostKeyPolicy",
        label: "Unknown SSH hosts",
        description: "What to do the first time a server's host key is seen. A changed key is always refused.",
        keywords: "known_hosts fingerprint tunnel",
        render: (s, id) => (
          <SelectControl
            id={id}
            value={s.connection.sshHostKeyPolicy}
            options={opts<HostKeyPolicy>([
              ["ask", "Ask me"],
              ["acceptNew", "Trust automatically"],
              ["strict", "Refuse"],
            ])}
            patch={(sshHostKeyPolicy) => ({ connection: { sshHostKeyPolicy } })}
          />
        ),
      },
      {
        id: "savePasswordsByDefault",
        label: "Save passwords by default",
        description: "Whether “Save password” starts checked in the connection form.",
        keywords: "keyring keychain secret",
        render: (s, id) => (
          <SwitchControl id={id} checked={s.connection.savePasswordsByDefault} patch={(savePasswordsByDefault) => ({ connection: { savePasswordsByDefault } })} />
        ),
      },
      {
        id: "notificationBuffer",
        label: "Notifications kept",
        description: "LISTEN notifications kept per session; the oldest are dropped first.",
        keywords: "listen notify",
        render: (s, id) => (
          <NumberControl id={id} value={s.connection.notificationBuffer} min={10} max={100_000} patch={(notificationBuffer) => ({ connection: { notificationBuffer } })} />
        ),
      },
    ],
  },
];

/** Rows whose label, description or keywords contain every word of `query`. */
export function matches(def: SettingDef, query: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const text = `${def.label} ${def.description} ${def.keywords ?? ""}`.toLowerCase();
  return words.every((w) => text.includes(w));
}
