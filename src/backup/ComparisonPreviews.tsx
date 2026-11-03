import { Button } from '../components/ui/controls';
import { useEffect, useRef, useState } from 'react';
import { libraryData, errorMessage } from '../ipc';

export function ComparisonPreviews({ runId, entryId }: { runId: string; entryId: string }) {
  const alive = useRef(true);
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<
    Partial<Record<'source' | 'destination', { image?: string; message: string }>>
  >({});
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  return (
    <div>
      <Button
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          for (const side of ['source', 'destination'] as const) {
            try {
              const value = await libraryData(
                { kind: 'backup_comparison_preview', run_id: runId, entry_id: entryId, side },
                'backup_comparison_preview',
              );
              if (!alive.current) return;
              if (
                value.runId !== runId ||
                value.entryId !== entryId ||
                value.side !== side ||
                value.png.length > 2 * 1024 * 1024
              )
                throw new Error('Preview identity or budget changed. Recompare.');
              let binary = '';
              for (const byte of value.png) binary += String.fromCharCode(byte);
              setResults((previous) => ({
                ...previous,
                [side]: { image: `data:image/png;base64,${btoa(binary)}`, message: value.message },
              }));
            } catch (cause) {
              if (!alive.current) return;
              setResults((previous) => ({ ...previous, [side]: { message: errorMessage(cause) } }));
            }
          }
          if (alive.current) setBusy(false);
        }}
      >
        Load side-by-side previews
      </Button>
      <p>
        Explicit read-only previews use the shared byte budget and bounded decoder. Unsupported,
        missing or changed files retain their text observations.
      </p>
      <div className="backup-scope" aria-label="Side-by-side comparison previews">
        {(['source', 'destination'] as const).map((side) => (
          <section key={side}>
            <h5>{side === 'source' ? 'Source preview' : 'Destination preview'}</h5>
            {results[side]?.image && (
              <img src={results[side].image} alt={`${side} comparison preview`} />
            )}
            <p role="status">
              {results[side]?.message ?? (busy ? 'Preparing preview…' : 'Preview not requested.')}
            </p>
          </section>
        ))}
      </div>
    </div>
  );
}
