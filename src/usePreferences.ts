import { useCallback, useEffect, useState } from 'react';
import type { AppPreferences } from './bindings';
import { errorMessage, getPreferences, savePreferences } from './ipc';
export function usePreferences() {
  const [preferences, setPreferences] = useState<AppPreferences | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let active = true;
    getPreferences()
      .then((value) => {
        if (active) {
          setPreferences(value);
          setError(null);
        }
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      });
    return () => {
      active = false;
    };
  }, [attempt]);
  const save = useCallback(async (draft: AppPreferences) => {
    const saved = await savePreferences(draft);
    setPreferences(saved);
    return saved;
  }, []);
  return {
    preferences,
    error,
    retry: () => {
      setError(null);
      setAttempt((value) => value + 1);
    },
    save,
  };
}
