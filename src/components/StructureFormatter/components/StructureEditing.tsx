import { Box, Text, chakra } from '@chakra-ui/react';
import type { FormEvent } from 'react';
import { OrganizationTemplates } from '@/components/OrganizationTemplates';
import { Button } from '@/components/Button';
import type { LayoutTab } from '@/components/OrganizationPresets';
import type { OrganizationTemplate } from '../../../bindings';
import type { useStructurePreview } from '../use-structure-preview';
import { StructureRuleEditor } from './StructureRuleEditor';
import { StructureExamples } from './StructureExamples';
import { StructureProtection } from './StructureProtection';

export function StructureEditing({
  template,
  category,
  workflow,
  onSubmit,
  onChange,
  onCategoryChange,
  onLoad,
}: {
  template: OrganizationTemplate;
  category: LayoutTab;
  workflow: ReturnType<typeof useStructurePreview>;
  onSubmit: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  onChange: (patch: Partial<OrganizationTemplate>) => void;
  onCategoryChange: (category: LayoutTab) => void;
  onLoad: (template: OrganizationTemplate) => void;
}) {
  const { preview } = workflow;
  return (
    <>
      <Box className="structure-editor">
        <chakra.form
          id="structure-rules-form"
          onSubmit={(event) => void onSubmit(event)}
          aria-busy={workflow.busy}
        >
          <StructureRuleEditor
            template={template}
            category={category}
            busy={workflow.busy}
            onChange={onChange}
            onCategoryChange={onCategoryChange}
          />
        </chakra.form>
        <StructureExamples template={template} />
      </Box>
      <chakra.details className="structure-advanced" mt="5">
        <chakra.summary>Catalog templates</chakra.summary>
        <OrganizationTemplates template={template} onLoad={onLoad} />
      </chakra.details>
      <StructureProtection />
      {workflow.busy && (
        <Box mt="4">
          <Text role="status">
            Checking the entire library…{' '}
            {preview
              ? `${preview.processed.toLocaleString()} of ${preview.total.toLocaleString()}`
              : 'Preparing library scope'}
          </Text>
          <Button type="button" onClick={workflow.reset}>
            Cancel preview
          </Button>
        </Box>
      )}
    </>
  );
}
