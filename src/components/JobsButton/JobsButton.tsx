import { Text } from '@chakra-ui/react';
import { useId, type Ref } from 'react';
import { Button } from '@/components/Button';
import type { LibraryStatus } from '../../bindings';
import { activeJobCount } from '../../utils/job-activity';

export function JobsButton({
  status,
  current,
  onOpen,
  buttonRef,
}: {
  status: LibraryStatus | null;
  current: boolean;
  onOpen: () => void;
  buttonRef: Ref<HTMLButtonElement>;
}) {
  const id = useId();
  const count = activeJobCount(status);
  return (
    <>
      <Button
        type="button"
        ref={buttonRef}
        className="jobs-trigger"
        aria-label="Jobs"
        title="Jobs"
        aria-current={current ? 'page' : undefined}
        aria-describedby={id}
        onClick={onOpen}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M8 6V3h8v3M3 6h18v14H3ZM3 11h18M10 10v4h4v-4"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinejoin="round"
          />
        </svg>
        <Text as="span" className="nav-label">
          Jobs
        </Text>
        {count > 0 && (
          <Text as="span" className="jobs-count" aria-hidden="true">
            {count}
          </Text>
        )}
      </Button>
      <Text as="span" id={id} className="screen-reader-status" role="status">
        {status ? `${count} running ${count === 1 ? 'job' : 'jobs'}` : 'Connecting to jobs'}
      </Text>
    </>
  );
}
