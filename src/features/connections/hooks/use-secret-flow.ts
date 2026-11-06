import { useCallback, useRef, useState } from "react";

import {
  type HostKeyInfo,
  nextStep,
  type SecretKind,
  type Secrets,
  withSecret,
} from "@/features/connections/model/connect-flow";
import { api } from "@/lib/ipc";
import { errorMessage, isAuthError } from "@/lib/ipc-error";

export type SecretPrompt = {
  secret: SecretKind;
  /** Set after a wrong answer, shown under the field. */
  error: string | null;
  submitting: boolean;
};

export type HostKeyPrompt = { info: HostKeyInfo; submitting: boolean };

/**
 * Runs `attempt` and answers what it asks for: a prompt for a missing
 * password / SSH secret, or a host key to confirm, then tries again with
 * everything typed so far. `onDone` gets the result; `onError` any failure
 * that isn't a question.
 */
export function useSecretFlow<T>(
  attempt: (secrets: Secrets) => Promise<T>,
  onDone: (result: T) => void,
  onError: (error: unknown) => void,
) {
  const [prompt, setPrompt] = useState<SecretPrompt | null>(null);
  const [hostKey, setHostKey] = useState<HostKeyPrompt | null>(null);
  const secrets = useRef<Secrets>({});
  /** The secret answered last, to ask again when it was wrong. */
  const answered = useRef<SecretKind | null>(null);

  const tryOnce = useCallback(async () => {
    try {
      const result = await attempt(secrets.current);
      setPrompt(null);
      setHostKey(null);
      onDone(result);
    } catch (error) {
      const step = nextStep(error);
      if (step?.kind === "secret") {
        setHostKey(null);
        setPrompt({ secret: step.secret, error: null, submitting: false });
      } else if (step?.kind === "hostKey") {
        setPrompt(null);
        setHostKey({ info: step.info, submitting: false });
      } else if (answered.current && isAuthError(error)) {
        // A wrong answer: ask again with the reason.
        setPrompt({ secret: answered.current, submitting: false, error: errorMessage(error) });
      } else {
        setPrompt(null);
        setHostKey(null);
        onError(error);
      }
    }
  }, [attempt, onDone, onError]);

  const start = useCallback(() => {
    secrets.current = {};
    answered.current = null;
    void tryOnce();
  }, [tryOnce]);

  const submitSecret = useCallback(
    (value: string) => {
      if (!prompt) return;
      secrets.current = withSecret(secrets.current, prompt.secret, value);
      answered.current = prompt.secret;
      setPrompt({ ...prompt, submitting: true, error: null });
      void tryOnce();
    },
    [prompt, tryOnce],
  );

  const trustHost = useCallback(async () => {
    if (!hostKey) return;
    setHostKey({ ...hostKey, submitting: true });
    answered.current = null;
    try {
      await api.trustSshHost(hostKey.info.host, hostKey.info.port, hostKey.info.key);
    } catch (error) {
      setHostKey(null);
      onError(error);
      return;
    }
    void tryOnce();
  }, [hostKey, tryOnce, onError]);

  const cancel = useCallback(() => {
    setPrompt(null);
    setHostKey(null);
  }, []);

  return { start, prompt, hostKey, submitSecret, trustHost, cancel };
}
