import { Handle, Position } from "@xyflow/react";

const HIDDEN = "!size-1.5 !min-h-0 !min-w-0 !border-0 !bg-transparent !opacity-0 pointer-events-none";

/**
 * A left (incoming) and right (outgoing) anchor on one row, so edges leave
 * and enter at that exact row. Invisible: edges can't be drawn by hand.
 */
export function RowHandles({ left, right }: { left: string; right: string }) {
  return (
    <>
      <Handle type="target" position={Position.Left} id={left} isConnectable={false} className={HIDDEN} />
      <Handle type="source" position={Position.Right} id={right} isConnectable={false} className={HIDDEN} />
    </>
  );
}
