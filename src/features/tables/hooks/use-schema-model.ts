import { useCallback, useEffect } from "react";

import { useSchemaCatalogStore } from "@/stores/schema-catalog";
import { type ModelStatus, useSchemaModelStore } from "@/stores/schema-model";

const IDLE: ModelStatus = { loading: false, error: null };

/** The session's schema model, loaded on first use. `model` is undefined until then. */
export function useSchemaModel(sessionId: string) {
  const model = useSchemaModelStore((s) => s.models[sessionId]);
  const status = useSchemaModelStore((s) => s.status[sessionId] ?? IDLE);
  const load = useSchemaModelStore((s) => s.load);
  const refreshModel = useSchemaModelStore((s) => s.refresh);

  useEffect(() => {
    void load(sessionId);
  }, [load, sessionId]);

  // Refresh also reloads autocomplete, so the console sees what another client changed.
  const refresh = useCallback(() => {
    void refreshModel(sessionId);
    void useSchemaCatalogStore.getState().refresh(sessionId);
  }, [refreshModel, sessionId]);
  return { model, status, refresh };
}
