import { Box, Heading, Text } from '@chakra-ui/react';

export function StructureLibraryScope({ editing }: { editing: boolean }) {
  if (!editing) return null;
  return (
    <Box
      as="section"
      aria-label="Library scope"
      className="structure-source"
      css={{
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '24px',
        paddingBlock: '0 24px',
        marginBlock: '0 24px',
        borderBottom: '1px solid {colors.border}',
        '& h2': { fontSize: '16px', fontWeight: '600', margin: '0 0 8px' },
        '& p': { margin: '0', maxWidth: '65ch' },
        '@media (max-width: 760px)': {
          alignItems: 'flex-start',
          flexDirection: 'column',
          gap: '16px',
        },
      }}
    >
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
          mb="4"
        >
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
