import type { LibraryOrganizationPreview, OrganizationTemplate } from '../../bindings';
import type { LayoutTab } from '@/components/OrganizationPresets';
import { pathParts, withFolders } from '@/components/StructurePathBuilder';

export const filenamePatterns = {
  original: '{original_stem}.{ext}',
  date_original: '{capture:%Y%m%d_%H%M%S}_{original_stem}.{ext}',
  sequence: '{sequence:04}.{ext}',
} as const;
export type FilenameChoice = keyof typeof filenamePatterns;
export function filenameChoice(pattern: string): FilenameChoice | 'custom' {
  const filename = pathParts(pattern).filename;
  return (
    (Object.keys(filenamePatterns) as FilenameChoice[]).find(
      (key) => filenamePatterns[key] === filename,
    ) ?? 'custom'
  );
}
export function withFilename(pattern: string, choice: FilenameChoice) {
  return withFolders(pathParts(pattern).folders, filenamePatterns[choice]);
}
export type SessionPreset = {
  id: string;
  name: string;
  category: LayoutTab;
  template: OrganizationTemplate;
};
const sessions = new Map<string, SessionPreset[]>();
export function sessionPresets(catalog: string): SessionPreset[] {
  return sessions.get(catalog) ?? [];
}
export function saveSessionPreset(catalog: string, preset: SessionPreset) {
  const presets = sessionPresets(catalog);
  if (presets.length >= 20)
    throw new Error('This session already has 20 presets. Remove one before saving another.');
  const name = preset.name.trim();
  if (!name || name.length > 80) throw new Error('Use a preset name of 1–80 characters.');
  if (presets.some((item) => item.name.toLocaleLowerCase() === name.toLocaleLowerCase()))
    throw new Error('A session preset already has this name. Choose another name.');
  if (!sessions.has(catalog) && sessions.size >= 32)
    throw new Error(
      'Session preset storage is full. Remove presets in another catalog before saving here.',
    );
  const next = [...presets, { ...preset, name, template: { ...preset.template } }];
  sessions.set(catalog, next);
  return next;
}
export function removeSessionPreset(catalog: string, id: string) {
  const next = sessionPresets(catalog).filter((preset) => preset.id !== id);
  if (next.length) sessions.set(catalog, next);
  else sessions.delete(catalog);
  return next;
}

export function simulationAvailable(
  preview: LibraryOrganizationPreview | null,
  stale: boolean,
  paging: boolean,
) {
  if (!preview?.complete || !preview.summary || stale || paging) return false;
  return preview.summary.moves + preview.summary.renames > 0;
}
