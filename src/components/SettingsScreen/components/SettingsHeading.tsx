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
      <chakra.header className="toolbar settings-heading">
        <Box className="settings-title-row">
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
