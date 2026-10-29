import { useCallback, useEffect, useState } from 'react';
import type {
  DirectorySelection,
  FoundationStatus,
  LibraryStatus,
  RootMode,
  ScanFilter,
} from './bindings';
import { errorMessage, getStatus, library, libraryData, pickDirectory } from './ipc';

export function useFoundation() {
  const [foundation, setFoundation] = useState<FoundationStatus | null>(null);
  const [status, setStatus] = useState<LibraryStatus | null>(null);
  const [selected, setSelected] = useState<DirectorySelection | null>(null);
  const [settingsId, setSettingsId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [notice, setNotice] = useState('');
  const showError = useCallback((error: unknown) => setError(errorMessage(error)), []);
  const refresh = useCallback(
    async () => setStatus(await libraryData({ kind: 'status' }, 'status')),
    [],
  );
  useEffect(() => {
    let active = true;
    Promise.all([getStatus(), libraryData({ kind: 'status' }, 'status')])
      .then(([foundation, status]) => {
        if (active) {
          setFoundation(foundation);
          setStatus(status);
          setLoading(false);
        }
      })
      .catch((error) => {
        if (active) {
          showError(error);
          setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [attempt, showError]);
  useEffect(() => {
    if (!foundation) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const value = await libraryData({ kind: 'status' }, 'status');
        if (active) setStatus(value);
      } catch (error) {
        if (active) showError(error);
      }
      if (active) timer = setTimeout(() => void poll(), 1500);
    }
    timer = setTimeout(() => void poll(), 1500);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [foundation, showError]);
  async function chooseFolder(relinkId?: string) {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      const value = await pickDirectory();
      if (value && relinkId) {
        await library({ kind: 'relink', root_id: relinkId, selection_id: value.id });
        await refresh();
        setNotice('Folder relinked. Reconciliation is running.');
      } else if (value) {
        setSelected(value);
        setSettingsId(null);
        setNotice('Folder selected. Its contents have not been scanned.');
      } else setNotice('Selection canceled. Your previous selection is unchanged.');
    } catch (error) {
      showError(error);
    } finally {
      setBusy(false);
    }
  }
  async function save(label: string, mode: RootMode, filter: ScanFilter) {
    if (!selected) return;
    setBusy(true);
    setError(null);
    try {
      await library({ kind: 'save_and_scan', selection_id: selected.id, label, mode, filter });
      await refresh();
      setSelected(null);
      setSettingsId(null);
      setNotice('Scan started. Originals stay unchanged.');
    } catch (error) {
      showError(error);
      await refresh().catch(showError);
    } finally {
      setBusy(false);
    }
  }
  async function rescan(rootId: string, mode: RootMode, filter: ScanFilter) {
    setBusy(true);
    setError(null);
    try {
      await library({ kind: 'update_mode', root_id: rootId, mode });
      await library({ kind: 'scan', root_id: rootId, filter });
      await refresh();
      setNotice('Scan started. Originals stay unchanged.');
    } catch (error) {
      await refresh().catch(showError);
      throw error;
    } finally {
      setBusy(false);
    }
  }
  return {
    foundation,
    status,
    selected,
    setSelected,
    settingsId,
    setSettingsId,
    loading,
    busy,
    error,
    setError,
    notice,
    showError,
    refresh,
    chooseFolder,
    save,
    rescan,
    reconnect: () => {
      setError(null);
      setLoading(true);
      setAttempt((value) => value + 1);
    },
  };
}
