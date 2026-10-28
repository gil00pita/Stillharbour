import type { ScanFilter } from '../bindings';
import { normalizeFileExtensions } from './model';
export const scanTagFields = [
  { key: 'excludeGlobs', label: 'Exclude patterns', placeholder: '**/edited/**' },
  { key: 'excludedFolders', label: 'Excluded folder names', placeholder: 'Private' },
  { key: 'extensions', label: 'File Extensions', placeholder: '.jpg' },
] as const;

// Include unfinished input even when the form is submitted without a blur.
// The native shared schema retains date fields; this scan UI deliberately clears them.
export function scanFilterFromForm(filter: ScanFilter, form: HTMLFormElement): ScanFilter {
  const submitted = { ...filter, includeGlobs: [], fromDate: null, throughDate: null };
  const fields = new FormData(form);
  for (const { key } of scanTagFields) {
    const draft = fields.get(key);
    if (typeof draft === 'string' && draft.trim())
      submitted[key] = Array.from(new Set([...submitted[key], draft.trim()]));
  }
  submitted.extensions = normalizeFileExtensions(submitted.extensions);
  return submitted;
}
