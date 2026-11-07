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
    <chakra.section
      className="settings-section"
      css={{
        padding: '32px 0',
        borderBottom: '1px solid {colors.border}',
        '& h2': {
          fontSize: '16px',
          fontWeight: '600',
          letterSpacing: '-0.01em',
          margin: '0 0 8px',
        },
        '& h3': { fontSize: '14px', margin: '0 0 8px' },
        '& > .help': { maxWidth: '72ch' },
        '& .view-switch': { margin: '16px 0', flexWrap: 'wrap', gap: '8px' },
        '& .view-switch > button': { fontSize: '13px !important' },
        '& .thumbnail-control': { fontSize: '13px' },
        '& .scan-options': { marginTop: '20px' },
      }}
      aria-labelledby="preview-cache-title"
    >
      <Heading as="h2" unstyled id="preview-cache-title">
        Preview cache
      </Heading>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
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
