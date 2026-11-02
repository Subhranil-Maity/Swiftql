import { useCallback, useMemo, useReducer } from "react";

import type { DriverInfo } from "@/lib/ipc";

import {
  type FormErrors,
  type FormState,
  hasErrors,
  validate,
} from "../model/form-state";

type Action =
  | { type: "set"; patch: Partial<FormState> }
  | { type: "setOption"; key: string; value: string }
  | { type: "reset"; form: FormState }
  | { type: "showErrors"; purpose: "test" | "save" };

interface State {
  form: FormState;
  /** Errors stay hidden until the first Test or Save. */
  shownFor: "test" | "save" | null;
}

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "set":
      return { ...state, form: { ...state.form, ...action.patch } };
    case "setOption":
      return {
        ...state,
        form: { ...state.form, options: { ...state.form.options, [action.key]: action.value } },
      };
    case "reset":
      return { form: action.form, shownFor: null };
    case "showErrors":
      // Once save errors are shown, keep checking the name too.
      return { ...state, shownFor: state.shownFor === "save" ? "save" : action.purpose };
  }
}

export function useConnectionForm(initial: FormState, driver: DriverInfo) {
  const [state, dispatch] = useReducer(reducer, { form: initial, shownFor: null });

  const errors: FormErrors = useMemo(
    () => (state.shownFor ? validate(state.form, driver, state.shownFor) : {}),
    [state.form, state.shownFor, driver],
  );

  const set = useCallback(
    <K extends keyof FormState>(field: K, value: FormState[K]) =>
      dispatch({ type: "set", patch: { [field]: value } as Partial<FormState> }),
    [],
  );
  const setOption = useCallback(
    (key: string, value: string) => dispatch({ type: "setOption", key, value }),
    [],
  );
  const reset = useCallback((form: FormState) => dispatch({ type: "reset", form }), []);

  /** Shows errors for `purpose` and returns whether the form is valid for it. */
  const check = useCallback(
    (purpose: "test" | "save") => {
      dispatch({ type: "showErrors", purpose });
      return !hasErrors(validate(state.form, driver, purpose));
    },
    [state.form, driver],
  );

  return { form: state.form, errors, set, setOption, reset, check };
}

export type ConnectionFormApi = ReturnType<typeof useConnectionForm>;
