import { Box, Heading, Text } from '@chakra-ui/react';

export function StructureLibraryScope({ editing }: { editing: boolean }) {
  if (!editing) return null;
  return (
    <Box as="section" aria-label="Library scope" className="structure-source">
      <Box>
        <Heading as="h2" fontSize="md">
          Entire library
        </Heading>
        <Text color="fg.muted">
          All indexed photos and videos across every library location. Gallery filters, albums and
          photo selections do not limit this scope.
        </Text>
      </Box>
    </Box>
  );
}
export function StructureFeedback({ error, stale }: { error: string | null; stale: boolean }) {
  return (
    <>
      {error && (
        <Text role="alert" className="error" mb="4">
          {error}
        </Text>
      )}
      {stale && (
        <Text role="status" mb="4">
          The catalog has changed. Return to editing and build a fresh preview before continuing.
        </Text>
      )}
    </>
  );
}
