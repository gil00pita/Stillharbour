import { Box, Flex, Heading, useRecipe } from '@chakra-ui/react';
import { useEffect, useId, useRef, useState, type FormEvent } from 'react';
import { defaultPreset, type LayoutTab } from '@/components/OrganizationPresets';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/Button';
import { SavedOrganizationPlans } from '@/components/SavedOrganizationPlans';
import type {
  LibraryOrganizationPreview,
  LibraryStatus,
  OrganizationTemplate,
} from '../../bindings';
import { structureFormatterRecipe } from './StructureFormatter.recipe';
import { useStructurePreview } from './use-structure-preview';
import { sessionPresets, simulationAvailable, type SessionPreset } from './structure-rules';
import { StructureProtection } from './components/StructureProtection';
import { StructureEditing } from './components/StructureEditing';
import { StructureFlowFooter } from './components/StructureFlowFooter';
import { StructureLibraryScope, StructureFeedback } from './components/StructureFeedback';
import { StructureReview } from './components/StructureReview';
import { SessionPresetsDialog } from './components/SessionPresetsDialog';
import {
  StructureSimulationComplete,
  StructureSimulationDialog,
} from './components/StructureSimulation';

export function StructureFormatter({
  status,
  catalogId,
}: {
  status: LibraryStatus;
  catalogId?: string | null;
}) {
  const styles = useRecipe({ recipe: structureFormatterRecipe })();
  const title = useRef<HTMLHeadingElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const fallbackScope = useId();
  const catalog = catalogId ?? fallbackScope;
  const [step, setStep] = useState<'edit' | 'review' | 'complete'>('edit');
  const [category, setCategory] = useState<LayoutTab>('date');
  const [template, setTemplate] = useState<OrganizationTemplate>({
    pattern: defaultPreset.pattern,
    event: '',
    missingDate: 'undated',
  });
  const [plansOpen, setPlansOpen] = useState(false);
  const [presetMode, setPresetMode] = useState<'save' | 'load' | null>(null);
  const [saved, setSaved] = useState(() => ({ catalog, presets: sessionPresets(catalog) }));
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [completion, setCompletion] = useState<LibraryOrganizationPreview | null>(null);
  const workflow = useStructurePreview(status.revision);
  const { preview } = workflow;
  const presets = saved.catalog === catalog ? saved.presets : sessionPresets(catalog);
  useEffect(() => {
    title.current?.focus({ preventScroll: true });
  }, []);
  useEffect(() => {
    if (content.current) content.current.scrollTop = 0;
  }, [step]);
  function update(patch: Partial<OrganizationTemplate>) {
    workflow.reset();
    setCompletion(null);
    setStep('edit');
    setTemplate((value) => ({ ...value, ...patch }));
  }
  function loadPreset(preset: SessionPreset) {
    workflow.reset();
    setCompletion(null);
    setTemplate({ ...preset.template });
    setCategory(preset.category);
    setStep('edit');
    setPresetMode(null);
  }
  async function build(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!event.currentTarget.reportValidity()) return;
    if (await workflow.build(template)) setStep('review');
  }
  function edit() {
    setStep('edit');
  }
  return (
    <Box className="structure-page" css={styles}>
      <PageHeader>
        <Flex as="header" className="toolbar">
          <Heading as="h1" ref={title} tabIndex={-1}>
            Structure Formater
          </Heading>
          <Flex className="header-actions">
            <Button
              type="button"
              aria-label="Saved presets"
              disabled={workflow.busy}
              onClick={() => setPresetMode('load')}
            >
              Presets
            </Button>
            <Button type="button" onClick={() => setPlansOpen(true)}>
              Saved plans
            </Button>
          </Flex>
        </Flex>
      </PageHeader>
      <Box className="structure-content" ref={content}>
        <StructureLibraryScope editing={step === 'edit'} />
        <StructureFeedback error={workflow.error} stale={workflow.stale} />
        {step === 'edit' && (
          <StructureEditing
            template={template}
            category={category}
            workflow={workflow}
            onSubmit={build}
            onChange={update}
            onCategoryChange={setCategory}
            onLoad={(value) => {
              update(value);
              setCategory('custom');
            }}
          />
        )}
        {step === 'review' && preview?.complete && (
          <>
            <StructureReview
              key={preview.previewId}
              preview={preview}
              template={template}
              roots={status.roots}
              paging={workflow.paging}
              stale={workflow.stale}
              onPage={workflow.page}
              onEdit={edit}
              error={workflow.error}
            />
            <StructureProtection />
          </>
        )}
        {step === 'complete' && completion && (
          <StructureSimulationComplete
            preview={completion}
            onEdit={edit}
            onReview={() => setStep('review')}
          />
        )}
      </Box>
      <StructureFlowFooter
        step={step}
        busy={workflow.busy}
        canSimulate={simulationAvailable(
          preview,
          workflow.stale,
          workflow.paging || Boolean(workflow.error),
        )}
        onSave={() => setPresetMode('save')}
        onSimulate={() => setConfirmOpen(true)}
      />
      {presetMode && (
        <SessionPresetsDialog
          key={presetMode}
          mode={presetMode}
          catalog={catalog}
          template={template}
          category={category}
          presets={presets}
          onPresetsChange={(values) => setSaved({ catalog, presets: values })}
          onLoad={loadPreset}
          onClose={() => setPresetMode(null)}
        />
      )}
      {confirmOpen && preview && (
        <StructureSimulationDialog
          preview={preview}
          stale={workflow.stale}
          onValidate={workflow.validate}
          onClose={() => setConfirmOpen(false)}
          onComplete={(value) => {
            setCompletion(value);
            setConfirmOpen(false);
            setStep('complete');
          }}
        />
      )}
      {plansOpen && <SavedOrganizationPlans onClose={() => setPlansOpen(false)} />}
    </Box>
  );
}
