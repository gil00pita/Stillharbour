import { Box, Flex, Text } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import classes from '../StructureFormatter.module.css';

const messages = {
  edit: 'Previewing leaves originals untouched. Actual restructuring remains disabled pending reviewed execution and proven recovery.',
  review:
    'Simulation only. No files will be moved or renamed; actual restructuring remains disabled.',
  complete:
    'Simulation finished. Your library is unchanged; actual restructuring remains disabled.',
};
export function StructureFlowFooter({
  step,
  busy,
  canSimulate,
  onSave,
  onSimulate,
}: {
  step: keyof typeof messages;
  busy: boolean;
  canSimulate: boolean;
  onSave: () => void;
  onSimulate: () => void;
}) {
  if (step === 'complete') return null;
  return (
    <Box className={`structure-flow-footer ${classes.footer}`}>
      <Text className="help" id="structure-execution-help">
        {messages[step]}
      </Text>
      <Flex gap="3" wrap="wrap">
        <Button type="button" disabled={busy} onClick={onSave}>
          Save preset
        </Button>
        {step === 'edit' && (
          <Button type="submit" form="structure-rules-form" className="primary" disabled={busy}>
            {busy ? 'Building preview…' : 'Preview restructure'}
          </Button>
        )}
        {step === 'review' && (
          <Button type="button" className="primary" disabled={!canSimulate} onClick={onSimulate}>
            Simulate plan
          </Button>
        )}
        <Button type="button" disabled aria-describedby="structure-execution-help">
          Run restructure
        </Button>
      </Flex>
    </Box>
  );
}
