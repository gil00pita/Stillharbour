import { Field, Box, Heading, Text, chakra } from '@chakra-ui/react';
import type { AppPreferences, LibraryStatus } from '../../../bindings';
import { NativeSelect } from '@/components/NativeSelect';
import { BrowserControls } from '@/components/BrowserControls';
import { Switch } from '@/components/Switch';

export function BrowsingDefaults({
  draft,
  status,
  onChange: update,
}: {
  draft: AppPreferences;
  status: LibraryStatus | null;
  onChange: (patch: Partial<AppPreferences>) => void;
}) {
  return (
    <chakra.section className="settings-section" aria-labelledby="browsing-defaults-title">
      <Heading as="h2" unstyled id="browsing-defaults-title">
        Browsing defaults
      </Heading>
      <Text className="help">
        Applied when you open Photos. Changes here do not alter your catalog notes or scan existing
        folders.
      </Text>
      <Box className="form-grid">
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>Default library</span>
            </Field.Label>
            <NativeSelect
              aria-label="Default library"
              value={draft.defaultRootId ?? ''}
              onChange={(event) => update({ defaultRootId: event.target.value || null })}
            >
              <option value="">All libraries</option>
              {draft.defaultRootId &&
                !status?.roots.some((root) => root.id === draft.defaultRootId) && (
                  <option value={draft.defaultRootId}>Unavailable saved library</option>
                )}
              {status?.roots.map((root) => (
                <option key={root.id} value={root.id}>
                  {root.label}
                </option>
              ))}
            </NativeSelect>
          </label>
        </Field.Root>
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>Default sort</span>
            </Field.Label>
            <NativeSelect
              aria-label="Default sort"
              value={draft.sort}
              onChange={(event) => update({ sort: event.target.value as AppPreferences['sort'] })}
            >
              <option value="capture">Date · newest first</option>
              <option value="added">Recently added</option>
              <option value="modified">File modified · newest first</option>
              <option value="name">Filename</option>
              <option value="size">Size · largest first</option>
            </NativeSelect>
          </label>
        </Field.Root>
      </Box>
      <BrowserControls
        mode={draft.galleryView}
        size={draft.thumbnailSize}
        fit={draft.gridFit}
        onMode={(galleryView) => update({ galleryView })}
        onSize={(thumbnailSize) => update({ thumbnailSize })}
        onFit={(gridFit) => update({ gridFit })}
      />
      <Switch
        label="Start with navigation collapsed"
        checked={draft.navigationCollapsed}
        onChange={(navigationCollapsed) => update({ navigationCollapsed })}
      />
    </chakra.section>
  );
}
