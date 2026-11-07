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
    <chakra.section className="settings-section" aria-labelledby="library-locations-title">
      <Box className="settings-section-heading">
        <Box>
          <Heading as="h2" unstyled id="library-locations-title">
            Library locations
          </Heading>
          <Text className="help">
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
          {!status.roots.length && <Text className="help">No photo folders added yet.</Text>}
          <List.Root unstyled className="settings-libraries">
            {status.roots.map((root) => (
              <List.Item unstyled key={root.id}>
                <Box>
                  <Heading as="h3" unstyled>
                    {root.label}
                  </Heading>
                  <Text className="path">
                    <bdi>{root.displayPath}</bdi>
                  </Text>
                  <Text className="help">
                    {root.availability.replaceAll('_', ' ')} · {root.fileCount.toLocaleString()}{' '}
                    indexed · {root.mode === 'managed' ? 'Managed' : 'Read only'}
                  </Text>
                </Box>
                <Box className="settings-library-actions">
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
                </Box>
              </List.Item>
            ))}
          </List.Root>
          <Text className="help">
            Use Change location to reconnect a moved folder, or Add photo folder for a different
            library. Stillharbour checks that a reconnected folder matches your library before
            updating its index.
          </Text>
        </>
      )}
    </chakra.section>
  );
}
