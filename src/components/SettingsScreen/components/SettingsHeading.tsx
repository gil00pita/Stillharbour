import { Box, Heading, chakra } from '@chakra-ui/react';
import { PageHeader } from '@/components/PageHeader';
import type { ReactNode, Ref } from 'react';

export function SettingsHeading({
  heading,
  jobsControl,
  navigationControl,
}: {
  heading: Ref<HTMLHeadingElement>;
  jobsControl: ReactNode;
  navigationControl: ReactNode;
}) {
  return (
    <PageHeader>
      <chakra.header
        className="toolbar settings-heading"
        css={{
          flexShrink: '0',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '16px',
          minHeight: '64px',
          padding: '12px 24px',
          borderBottom: '1px solid {colors.border}',
          background: '{colors.bg.toolbar}',
          '@media (max-width: 600px)': { minHeight: '64px', padding: '18px 24px' },
          marginBottom: '24px',
          '& h1': { fontSize: '24px', margin: '0 0 8px' },
          '& > p': { color: '{colors.fg.muted}', maxWidth: '72ch' },
          margin: '0',
        }}
      >
        <Box
          className="settings-title-row"
          css={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            justifyContent: 'space-between',
            width: '100%',
          }}
        >
          {navigationControl}
          <Heading as="h1" unstyled ref={heading} tabIndex={-1}>
            Settings
          </Heading>
          {jobsControl}
        </Box>
      </chakra.header>
    </PageHeader>
  );
}
