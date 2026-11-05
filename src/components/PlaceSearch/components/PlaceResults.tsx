import { Box, Button, Stack, Text } from '@chakra-ui/react';
import type { PlaceSearchReply, PlaceSummary } from '../../../bindings';

export function PlaceStatus({
  error,
  current,
  text,
  retry,
}: {
  error: string | null;
  current: PlaceSearchReply | null;
  text: string;
  retry: () => void;
}) {
  if (error)
    return (
      <Box role="alert">
        <Text m="0">{error}</Text>
        <Button type="button" onClick={retry}>
          Retry place search
        </Button>
      </Box>
    );
  let message = 'Searching places…';
  if (current) {
    if (text.trim().length < 2) message = 'Enter at least two characters.';
    else
      message = current.matches.length
        ? 'Choose a city or country below.'
        : 'No places found. Try another name or spelling.';
  }
  return (
    <Text m="0" role="status" fontSize="sm">
      {message}
    </Text>
  );
}

export function PlaceResults({
  reply,
  disabled,
  choose,
}: {
  reply: PlaceSearchReply;
  disabled: boolean;
  choose: (place: PlaceSummary) => void;
}) {
  return (
    <>
      <Box as="ul" m="0" p="2" listStyleType="none" maxH="64" overflowY="auto">
        {reply.matches.map((place) => (
          <Box as="li" key={place.id}>
            <Button
              type="button"
              variant="outline"
              width="full"
              height="auto"
              py="2"
              justifyContent="start"
              whiteSpace="normal"
              textAlign="start"
              disabled={disabled}
              onClick={() => choose(place)}
            >
              <Stack gap="1" align="start">
                <Text m="0">{place.label}</Text>
                <Text m="0" fontSize="xs" color="fg.muted">
                  {place.area}
                </Text>
              </Stack>
            </Button>
          </Box>
        ))}
      </Box>
      {reply.more && (
        <Text m="0" fontSize="sm">
          More places match. Add a country or region to narrow the search.
        </Text>
      )}
      <Box as="details" fontSize="xs" color="fg.muted">
        <Box as="summary" cursor="pointer">
          Coverage and data sources
        </Box>
        <Text m="0" whiteSpace="pre-wrap">
          {reply.credits}
        </Text>
      </Box>
    </>
  );
}
