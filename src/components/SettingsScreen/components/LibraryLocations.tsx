import type { LibraryStatus } from '../../../bindings';
import { Button } from '@/components/Button';

export function LibraryLocations({
  status,
  busy,
  onAddFolder,
  onRelink,
  onRemove,
}: {
  status: LibraryStatus | null;
  busy: boolean;
  onAddFolder: () => void;
  onRelink: (id: string) => Promise<void>;
  onRemove: (id: string, opener: HTMLButtonElement) => void;
}) {
  return (
    <section className="settings-section" aria-labelledby="library-locations-title">
      <div className="settings-section-heading">
        <div>
          <h2 id="library-locations-title">Library locations</h2>
          <p className="help">Photo folders indexed in place. Your photos stay where they are.</p>
        </div>
        <Button disabled={busy || !status} onClick={onAddFolder}>
          Add photo folder
        </Button>
      </div>
      {!status ? (
        <p role="status">Connecting to your libraries…</p>
      ) : (
        <>
          {!status.roots.length && <p className="help">No photo folders added yet.</p>}
          <ul className="settings-libraries">
            {status.roots.map((root) => (
              <li key={root.id}>
                <div>
                  <h3>{root.label}</h3>
                  <p className="path">
                    <bdi>{root.displayPath}</bdi>
                  </p>
                  <p className="help">
                    {root.availability.replaceAll('_', ' ')} · {root.fileCount.toLocaleString()}{' '}
                    indexed · {root.mode === 'managed' ? 'Managed' : 'Read only'}
                  </p>
                </div>
                <div className="settings-library-actions">
                  <Button
                    disabled={busy || root.availability === 'scanning'}
                    onClick={() => void onRelink(root.id)}
                    aria-label={`Change location for ${root.label}`}
                  >
                    Change location
                  </Button>
                  <Button
                    type="button"
                    className="destructive"
                    disabled={busy || root.availability === 'scanning'}
                    onClick={(event) => {
                      onRemove(root.id, event.currentTarget);
                    }}
                    aria-label={`Remove library ${root.label}`}
                  >
                    Remove library
                  </Button>
                </div>
              </li>
            ))}
          </ul>
          <p className="help">
            Use Change location to reconnect a moved folder, or Add photo folder for a different
            library. Stillharbour checks that a reconnected folder matches your library before
            updating its index.
          </p>
        </>
      )}
    </section>
  );
}
