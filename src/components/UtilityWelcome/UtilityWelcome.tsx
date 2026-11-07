import { Box, Heading, Text } from '@chakra-ui/react';

export function UtilityWelcome({ title, description }: { title: string; description: string }) {
  return (
    <Box as="section" aria-label="Scan introduction" py="4" maxW="full">
      <Heading as="h2" fontSize="lg" fontWeight={500} lineHeight="title" mt="0" mb="3">
        {title}
      </Heading>
      <Text color="fg.muted" maxW="70ch" m="0" lineHeight="1.6">
        {description}
      </Text>
    </Box>
  );
}
