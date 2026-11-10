import { Box, Heading, List, Text, chakra } from '@chakra-ui/react';
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
      aria-labelledby="library-locations-title"
    >
      <Box
        className="settings-section-heading"
        css={{
          display: 'flex',
          justifyContent: 'space-between',
          gap: '16px',
          alignItems: 'start',
          '@media (max-width: 760px)': { flexWrap: 'wrap' },
        }}
      >
        <Box>
          <Heading as="h2" unstyled id="library-locations-title">
            Library locations
          </Heading>
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
          >
            Photo folders indexed in place. Your photos stay where they are.
          </Text>
        </Box>
        <Button disabled={busy || !status} onClick={onAddFolder}>
          Add photo folder
        </Button>
      </Box>
      {!status ? (
        <Text role="status">Connecting to your libraries…</Text>
      ) : (
        <>
          {!status.roots.length && (
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              No photo folders added yet.
            </Text>
          )}
          <List.Root
            unstyled
            className="settings-libraries"
            css={{
              padding: '0',
              margin: '24px 0 16px',
              listStyle: 'none',
              '& li': {
                display: 'grid',
                gridTemplateColumns: 'minmax(0, 1fr) auto',
                gap: '16px',
                padding: '16px 0',
                borderTop: '1px solid {colors.border}',
              },
              '@media (max-width: 760px)': {
                '& li': { gridTemplateColumns: 'minmax(0, 1fr)', gap: '12px' },
              },
            }}
          >
            {status.roots.map((root) => (
              <List.Item unstyled key={root.id}>
                <Box>
                  <Heading as="h3" unstyled>
                    {root.label}
                  </Heading>
                  <Text
                    className="path"
                    css={{
                      fontSize: '13px',
                      lineHeight: '1.6',
                      overflowWrap: 'anywhere',
                      whiteSpace: 'pre-wrap',
                      margin: '8px 0 0',
                      color: '{colors.fg.muted}',
                    }}
                  >
                    <bdi>{root.displayPath}</bdi>
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
                    {root.availability.replaceAll('_', ' ')} · {root.fileCount.toLocaleString()}{' '}
                    indexed · {root.mode === 'managed' ? 'Managed' : 'Read only'}
                  </Text>
                </Box>
                <Box
                  className="settings-library-actions"
                  css={{ display: 'flex', flexWrap: 'wrap', alignItems: 'start', gap: '8px' }}
                >
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
                    css={{ 'button&:not(.primary)': { color: '{colors.fg.secondary}' } }}
                    disabled={busy || root.availability === 'scanning'}
                    onClick={(event) => {
                      onRemove(root.id, event.currentTarget);
                    }}
                    aria-label={`Remove library ${root.label}`}
                  >
                    Remove library
                  </Button>
                </Box>
              </List.Item>
            ))}
          </List.Root>
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
          >
            Use Change location to reconnect a moved folder, or Add photo folder for a different
            library. Stillharbour checks that a reconnected folder matches your library before
            updating its index.
          </Text>
        </>
      )}
    </chakra.section>
  );
}
