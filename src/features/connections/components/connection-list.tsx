import type { SavedConnection } from "@/lib/ipc";

import { ConnectionListItem } from "./connection-list-item";

export function ConnectionList({
  connections,
  connectingId,
  onConnect,
  onEdit,
  onDelete,
  labelledBy,
}: {
  connections: SavedConnection[];
  connectingId: string | null;
  onConnect: (c: SavedConnection) => void;
  onEdit: (c: SavedConnection) => void;
  onDelete: (c: SavedConnection) => void;
  labelledBy: string;
}) {
  return (
    <ul aria-labelledby={labelledBy} className="-mx-2 flex flex-col">
      {connections.map((c) => (
        <ConnectionListItem
          key={c.id}
          connection={c}
          connecting={connectingId === c.id}
          // One connect at a time.
          disabled={connectingId !== null}
          onConnect={() => onConnect(c)}
          onEdit={() => onEdit(c)}
          onDelete={() => onDelete(c)}
        />
      ))}
    </ul>
  );
}
