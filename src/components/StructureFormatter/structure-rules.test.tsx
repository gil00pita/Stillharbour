import { expect, it } from 'vitest';
import {
  filenameChoice,
  withFilename,
  sessionPresets,
  saveSessionPreset,
  removeSessionPreset,
  type SessionPreset,
} from './structure-rules';

function preset(name: string): SessionPreset {
  return {
    id: crypto.randomUUID(),
    name,
    category: 'custom',
    template: {
      pattern: 'Family/{country}/{original_stem}.{ext}',
      event: '',
      missingDate: 'skip',
      filenameStemCase: 'lowercase',
    },
  };
}
it('changes filename choices without rewriting custom folder levels', () => {
  const original = 'Family/{country}/{original_stem}.{ext}';
  expect(filenameChoice(original)).toBe('original');
  expect(withFilename(original, 'sequence')).toBe('Family/{country}/{sequence:04}.{ext}');
  expect(filenameChoice('Family/custom_{original_stem}.{ext}')).toBe('custom');
});
it('retains session-only snapshots by catalog and rejects duplicate or oversized names', () => {
  const catalog = crypto.randomUUID();
  const first = preset('  My archive  ');
  saveSessionPreset(catalog, first);
  first.template.pattern = 'Changed/{original_stem}.{ext}';
  expect(sessionPresets(catalog)[0]?.template.pattern).toBe(
    'Family/{country}/{original_stem}.{ext}',
  );
  expect(sessionPresets(`${catalog}-other`)).toEqual([]);
  expect(() => saveSessionPreset(catalog, preset('my ARCHIVE'))).toThrow('already has this name');
  expect(() => saveSessionPreset(catalog, preset('x'.repeat(81)))).toThrow('1–80');
  expect(sessionPresets(catalog)).toHaveLength(1);
  removeSessionPreset(catalog, first.id);
  expect(sessionPresets(catalog)).toEqual([]);
});
it('enforces the session preset limit without silently discarding earlier rules', () => {
  const catalog = crypto.randomUUID();
  const first = preset('First');
  saveSessionPreset(catalog, first);
  for (let index = 1; index < 20; index++) saveSessionPreset(catalog, preset(`Preset ${index}`));
  expect(() => saveSessionPreset(catalog, preset('Overflow'))).toThrow('20 presets');
  expect(sessionPresets(catalog)[0]?.id).toBe(first.id);
  removeSessionPreset(catalog, first.id);
  expect(saveSessionPreset(catalog, preset('Replacement'))).toHaveLength(20);
});
