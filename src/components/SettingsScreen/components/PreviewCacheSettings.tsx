import type { LibraryStatus } from '../../../bindings';
import { Button } from '@/components/Button';

export function PreviewCacheSettings({
  status,
  busy,
  onReview,
}: {
  status: LibraryStatus | null;
  busy: boolean;
  onReview: (opener: HTMLButtonElement) => void;
}) {
  return (
    <section className="settings-section" aria-labelledby="preview-cache-title">
      <h2 id="preview-cache-title">Preview cache</h2>
      <p className="help">
        Clears indexed previews for all libraries. Originals and catalog notes stay unchanged.
      </p>
      <Button
        type="button"
        disabled={
          busy ||
          !status ||
          status.jobs.some((job) =>
            ['queued', 'running', 'cancel_requested', 'pause_requested', 'paused'].includes(
              job.state,
            ),
          )
        }
        onClick={(event) => {
          onReview(event.currentTarget);
        }}
      >
        Clear preview cache
      </Button>
    </section>
  );
}
