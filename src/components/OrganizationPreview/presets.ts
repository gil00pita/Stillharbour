export const defaultPreset = {
  name: 'Year / date / original name',
  pattern: '{capture:%Y}/{capture:%Y-%m-%d}/{original_stem}.{ext}',
};
export const presets = [
  defaultPreset,
  {
    name: 'Year / month / date and sequence',
    pattern: '{capture:%Y}/{capture:%m}/{capture:%Y%m%d_%H%M%S}_{sequence:04}.{ext}',
  },
  {
    name: 'Event / date / original name',
    pattern: '{event}/{capture:%Y-%m-%d}/{original_stem}.{ext}',
  },
];
