import { Box, Heading, Text } from '@chakra-ui/react';

export function UtilityWelcome({ title, description }: { title: string; description: string }) {
  return (
    <Box as="section" aria-label="Scan introduction" py="4" maxW="full">
      <Heading as="h2" fontWeight="medium">
        {title}
      </Heading>
      <Text color="fg.muted" maxW="sm">
        {description}
      </Text>
    </Box>
  );
}
