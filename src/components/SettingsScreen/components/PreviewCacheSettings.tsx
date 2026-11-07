import { Heading, Text, chakra } from '@chakra-ui/react';
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
    <chakra.section className="settings-section" aria-labelledby="preview-cache-title">
      <Heading as="h2" unstyled id="preview-cache-title">
        Preview cache
      </Heading>
      <Text className="help">
        Clears indexed previews for all libraries. Originals and catalog notes stay unchanged.
      </Text>
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
    </chakra.section>
  );
}
