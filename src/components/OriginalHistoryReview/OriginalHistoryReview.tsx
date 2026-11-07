import { Box, Heading, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type { OriginalOperationHistory, OriginalOperationJob } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';

async function readCatalogHistory(jobId: string) {
  return libraryData(
    { kind: 'organization_original_history', job_id: jobId },
    'organization_original_history',
  );
}
type Props = {
  job: OriginalOperationJob;
  readHistory?: (jobId: string) => Promise<OriginalOperationHistory>;
};
export function OriginalHistoryReview(props: Props) {
  return <HistoryForJob key={props.job.id} {...props} />;
}
function HistoryForJob({ job, readHistory = readCatalogHistory }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const epoch = useRef(0);
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState(false);
  const [history, setHistory] = useState<OriginalOperationHistory | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  useEffect(() => {
    if (open && !dialog.current?.open) dialog.current?.showModal();
  }, [open]);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [],
  );
  function close() {
    epoch.current += 1;
    dialog.current?.close();
    setOpen(false);
    setPending(false);
    setHistory(null);
    setError(null);
  }
  async function load() {
    const token = ++epoch.current;
    setOpen(true);
    setPending(true);
    setHistory(null);
    setError(null);
    setPage(0);
    try {
      const value = await readHistory(job.id);
      if (epoch.current !== token) return;
      if (
        value.job.id !== job.id ||
        value.job.planId !== job.planId ||
        value.job.sourceRootId !== job.sourceRootId ||
        !value.historyOnly ||
        value.currentLocationsChecked ||
        value.executionAvailable ||
        value.members.length === 0 ||
        value.members.length > 512 ||
        new Set(value.members.map((member) => member.id)).size !== value.members.length
      ) {
        throw new Error(
          'Organization history does not match this job. Preserve the catalog for recovery review.',
        );
      }
      setHistory(value);
    } catch (cause) {
      if (epoch.current === token) setError(errorMessage(cause));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  return (
    <>
      <Button type="button" onClick={() => void load()}>
        Review retained organization plan
      </Button>
      {open && (
        <chakra.dialog
          ref={dialog}
          className="organization-preview"
          css={{
            width: 'min(1100px, calc(100vw - 48px))',
            maxHeight: 'calc(100dvh - 48px)',
            padding: '0',
            border: '1px solid {colors.border}',
            borderRadius: '12px',
            color: 'inherit',
            background: '{colors.bg.panel}',
            '&[open]': { display: 'flex', flexDirection: 'column' },
            '&::backdrop': { background: 'rgb(0 0 0 / 65%)' },
            '@media (max-width: 760px)': {
              width: 'calc(100vw - 24px)',
              maxHeight: 'calc(100dvh - 24px)',
            },
            '&.organization-inline': {
              display: 'flex',
              flexDirection: 'column',
              position: 'static',
              width: '100%',
              maxHeight: 'none',
              margin: '0',
              border: '0',
              borderRadius: '0',
              background: 'transparent',
            },
          }}
          aria-labelledby="original-history-title"
          onCancel={(event) => {
            event.preventDefault();
            event.stopPropagation();
            close();
          }}
        >
          <chakra.header
            className="organization-heading"
            css={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: '24px',
              padding: '20px 24px',
              flexShrink: '0',
              borderBottom: '1px solid {colors.border}',
              '& h2': { margin: '0', marginBottom: '8px', fontSize: '20px' },
              '& p': { margin: '0' },
            }}
          >
            <Heading as="h2" unstyled id="original-history-title">
              Retained organization plan
            </Heading>
            <Button type="button" onClick={close}>
              Close plan history
            </Button>
          </chakra.header>
          <Box
            className="organization-body"
            css={{
              padding: '0 24px 24px',
              overflowY: 'auto',
              minHeight: '0',
              '& fieldset': {
                display: 'grid',
                gap: '16px',
                padding: '0',
                border: '0',
                margin: '24px 0',
                minWidth: '0',
              },
              '& label': { display: 'grid', gap: '8px', minWidth: '0' },
              '& input': { width: '100%', minWidth: '0' },
              '& select': { width: '100%', minWidth: '0' },
              '& fieldset > button': { justifySelf: 'start' },
              '& bdi': { whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' },
            }}
          >
            <Text>
              Saved plan history. Current file locations and contents require a fresh native
              recovery inspection before any action.
            </Text>
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              Opening this review grants no execution approval. Original-file changes remain
              unavailable under the preservation recovery gate.
            </Text>
            {pending && <Text role="status">Loading retained plan…</Text>}
            {error && (
              <>
                <Text
                  role="alert"
                  className="error"
                  css={{
                    border: '1px solid {colors.border.control}',
                    borderRadius: 'var(--control-radius)',
                    background: '{colors.danger.bg}',
                    color: '{colors.danger.fg}',
                    padding: '16px 20px',
                    margin: '24px 0 0',
                    fontSize: '13px',
                    lineHeight: '1.6',
                    '& p': { margin: '0' },
                    '& button': {
                      marginTop: '12px',
                      border: '1px solid currentColor',
                      background: 'transparent',
                      color: 'inherit',
                      borderRadius: '4px',
                      padding: '7px 12px',
                    },
                  }}
                >
                  {error}
                </Text>
                <Button type="button" onClick={() => void load()}>
                  Retry history
                </Button>
              </>
            )}
            {history && (
              <>
                <Text>
                  {history.job.state === 'needs_recovery'
                    ? 'Needs recovery review'
                    : history.job.state === 'completed_verified'
                      ? 'Recorded completed and verified'
                      : 'Recorded pending attempt'}
                </Text>
                <Text>
                  {history.members.length} retained members · {history.job.verifiedMoves} /{' '}
                  {history.job.totalMoves} recorded verified moves
                </Text>
                <Text>
                  Plan digest: <bdi>{history.planDigest}</bdi>
                </Text>
                {history.reverseOf && (
                  <Text>
                    Reverse of plan <bdi>{history.reverseOf}</bdi>
                  </Text>
                )}
                <Box
                  className="organization-table"
                  css={{
                    overflowX: 'auto',
                    marginTop: '16px',
                    '& table': { width: '100%', borderCollapse: 'collapse', tableLayout: 'fixed' },
                    '& th': {
                      padding: '12px 16px 12px 0',
                      textAlign: 'left',
                      verticalAlign: 'top',
                      overflowWrap: 'anywhere',
                      borderBottom: '1px solid {colors.border}',
                    },
                    '& td': {
                      padding: '12px 16px 12px 0',
                      textAlign: 'left',
                      verticalAlign: 'top',
                      overflowWrap: 'anywhere',
                      borderBottom: '1px solid {colors.border}',
                    },
                    '& caption': {
                      textAlign: 'left',
                      paddingBottom: '12px',
                      color: '{colors.fg.muted}',
                    },
                    '& small': {
                      display: 'block',
                      color: '{colors.fg.muted}',
                      marginTop: '8px',
                      fontSize: '12px',
                      lineHeight: '16px',
                    },
                    '& bdi': { whiteSpace: 'pre-wrap' },
                    '& p': { margin: '0 0 8px' },
                  }}
                >
                  <Table.Root unstyled>
                    <Table.Caption unstyled captionSide="top">
                      Saved paths for this {history.job.kind === 'undo' ? 'undo' : 'rename'} plan
                    </Table.Caption>
                    <Table.Header unstyled>
                      <Table.Row unstyled>
                        <Table.ColumnHeader unstyled scope="col">
                          Before
                        </Table.ColumnHeader>
                        <Table.ColumnHeader unstyled scope="col">
                          After
                        </Table.ColumnHeader>
                      </Table.Row>
                    </Table.Header>
                    <Table.Body unstyled>
                      {history.members.slice(page * 64, (page + 1) * 64).map((member) => (
                        <Table.Row unstyled key={member.id}>
                          <Table.Cell unstyled>
                            <bdi>{member.before}</bdi>
                          </Table.Cell>
                          <Table.Cell unstyled>
                            <bdi>{member.after}</bdi>
                          </Table.Cell>
                        </Table.Row>
                      ))}
                    </Table.Body>
                  </Table.Root>
                </Box>
                {history.members.length > 64 && (
                  <chakra.nav aria-label="Retained plan members">
                    <Button
                      type="button"
                      disabled={page === 0}
                      onClick={() => setPage((value) => value - 1)}
                    >
                      Previous members
                    </Button>
                    <Text as="span">
                      Page {page + 1} of {Math.ceil(history.members.length / 64)}
                    </Text>
                    <Button
                      type="button"
                      disabled={(page + 1) * 64 >= history.members.length}
                      onClick={() => setPage((value) => value + 1)}
                    >
                      Next members
                    </Button>
                  </chakra.nav>
                )}
              </>
            )}
          </Box>
        </chakra.dialog>
      )}
    </>
  );
}
