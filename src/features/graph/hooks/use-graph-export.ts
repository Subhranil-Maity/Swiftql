import { type RefObject, useCallback } from "react";
import { getNodesBounds, useReactFlow } from "@xyflow/react";
import { toPng, toSvg } from "html-to-image";
import { toast } from "sonner";

import { errorMessage } from "@/lib/ipc-error";
import { dataUrlBytes, fileSafe, saveFile } from "@/lib/save-file";

export type GraphExportFormat = "png" | "svg";

/** Margin around the tables in the image. */
const PADDING = 48;
/** Browsers refuse canvases much larger than this. */
const MAX_CANVAS_SIDE = 16_000;

/**
 * Draws every visible table and edge at 100% zoom, whatever the current
 * viewport, then asks where to save it.
 */
export function useGraphExport(containerRef: RefObject<HTMLElement | null>, name: string) {
  const { getNodes } = useReactFlow();

  return useCallback(
    async (format: GraphExportFormat) => {
      const container = containerRef.current;
      const viewport = container?.querySelector<HTMLElement>(".react-flow__viewport");
      const nodes = getNodes().filter((n) => !n.hidden);
      if (!container || !viewport || nodes.length === 0) return;

      const bounds = getNodesBounds(nodes);
      const width = Math.ceil(bounds.width + PADDING * 2);
      const height = Math.ceil(bounds.height + PADDING * 2);
      const options = {
        backgroundColor: getComputedStyle(container).backgroundColor,
        width,
        height,
        pixelRatio: Math.min(2, MAX_CANVAS_SIDE / Math.max(width, height)),
        style: {
          width: `${width}px`,
          height: `${height}px`,
          transform: `translate(${PADDING - bounds.x}px, ${PADDING - bounds.y}px) scale(1)`,
        },
      };

      try {
        const dataUrl = format === "png" ? await toPng(viewport, options) : await toSvg(viewport, options);
        const path = await saveFile(`${fileSafe(name)}.${format}`, dataUrlBytes(dataUrl), [
          { name: format.toUpperCase(), extensions: [format] },
        ]);
        if (path) toast.success("Saved the diagram", { description: path });
      } catch (error) {
        toast.error("Couldn't export the diagram", { description: errorMessage(error) });
      }
    },
    [containerRef, getNodes, name],
  );
}
