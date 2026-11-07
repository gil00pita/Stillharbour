export type LayoutGroup = 'date' | 'location' | 'tags' | 'camera';
export type OrganizationPreset = {
  name: string;
  description: string;
  example: string[];
  pattern: string;
  group: LayoutGroup;
};
const filename = '{original_stem}.{ext}';
function layout(
  group: LayoutGroup,
  name: string,
  folders: string[],
  example: string[],
  description: string,
): OrganizationPreset {
  return {
    group,
    name,
    pattern: [...folders, filename].join('/'),
    example: [...example, 'IMG_0421.jpg'],
    description,
  };
}
export const defaultPreset: OrganizationPreset = {
  group: 'date',
  name: 'By day',
  description: 'Year / date / original filename',
  example: ['2026', '2026-11-07', 'IMG_0421.jpg'],
  pattern: '{capture:%Y}/{capture:%Y-%m-%d}/{original_stem}.{ext}',
};
export const presets: OrganizationPreset[] = [
  layout('date', 'Year', ['{capture:%Y}'], ['2026'], 'One folder for each capture year'),
  layout(
    'date',
    'Year → Month',
    ['{capture:%Y}', '{capture:%m}'],
    ['2026', '11'],
    'Group each year into months',
  ),
  layout(
    'date',
    'Year → Month → Day',
    ['{capture:%Y}', '{capture:%m}', '{capture:%d}'],
    ['2026', '11', '07'],
    'A separate folder for each day',
  ),
  layout(
    'date',
    'Year → Quarter',
    ['{capture:%Y}', '{quarter}'],
    ['2026', 'Q4'],
    'Capture-year quarters, Q1 through Q4',
  ),
  layout(
    'date',
    'Year → Month → Location',
    ['{capture:%Y}', '{capture:%m}', '{country}'],
    ['2026', '11', 'Portugal'],
    'Use the embedded country beneath each month',
  ),
  defaultPreset,
  {
    group: 'date',
    name: 'By month',
    description: 'Year / month / timestamp and sequence',
    example: ['2026', '11', '20261107_143052_0001.jpg'],
    pattern: '{capture:%Y}/{capture:%m}/{capture:%Y%m%d_%H%M%S}_{sequence:04}.{ext}',
  },
  {
    group: 'date',
    name: 'Date & time',
    description: 'Date / time and original filename',
    example: ['2026-11-07', '143052_IMG_0421.jpg'],
    pattern: '{capture:%Y-%m-%d}/{capture:%H%M%S}_{original_stem}.{ext}',
  },
  layout('location', 'Country', ['{country}'], ['Portugal'], 'Use the embedded country name'),
  layout(
    'location',
    'Country → City',
    ['{country}', '{city}'],
    ['Portugal', 'Lisbon'],
    'Group cities within each country',
  ),
  layout(
    'location',
    'Country → Region → City',
    ['{country}', '{region}', '{city}'],
    ['Portugal', 'Algarve', 'Lagos'],
    'Keep the full embedded region hierarchy',
  ),
  layout(
    'location',
    'Country → City → Place',
    ['{country}', '{city}', '{place}'],
    ['Portugal', 'Lisbon', 'Belem'],
    'Add the embedded place or sublocation',
  ),
  layout('tags', 'Tag', ['{tag}'], ['Nature'], 'Use an unambiguous top-level catalog tag'),
  layout(
    'tags',
    'Tag → Subcategory',
    ['{tag}', '{subcategory}'],
    ['Nature', 'Birds'],
    'Follow two levels of your tag categories',
  ),
  layout(
    'tags',
    'Tag → Subcategory → Subject',
    ['{tag}', '{subcategory}', '{subject}'],
    ['Nature', 'Birds', 'Kingfishers'],
    'Follow three levels of your tag categories',
  ),
  layout(
    'tags',
    'Tag → Location',
    ['{tag}', '{city}'],
    ['Architecture', 'Lisbon'],
    'Use the embedded city beneath the tag',
  ),
  layout(
    'tags',
    'Tag → Year',
    ['{tag}', '{capture:%Y}'],
    ['Street-Photography', '2026'],
    'Group tagged photos by capture year',
  ),
  layout(
    'tags',
    'Tag → Year → Month',
    ['{tag}', '{capture:%Y}', '{capture:%m}'],
    ['Street-Photography', '2026', '02'],
    'Group tagged photos by capture month',
  ),
  layout(
    'camera',
    'Camera type',
    ['{camera_type}'],
    ['Mirrorless'],
    'Requires an explicit embedded camera type',
  ),
  layout(
    'camera',
    'Camera type → Brand',
    ['{camera_type}', '{brand}'],
    ['Mirrorless', 'Sony'],
    'Camera type, then the observed make',
  ),
  layout(
    'camera',
    'Camera type → Brand → Model',
    ['{camera_type}', '{brand}', '{model}'],
    ['Mirrorless', 'Sony', 'A7-IV'],
    'Keep type, make and model as separate folders',
  ),
  layout(
    'camera',
    'Brand → Model',
    ['{brand}', '{model}'],
    ['Canon', 'EOS-R5'],
    'Use the embedded make and model',
  ),
  layout(
    'camera',
    'Camera model → Year → Month',
    ['{camera}', '{capture:%Y}', '{capture:%m}'],
    ['Sony-A7-IV', '2026', '11'],
    'Camera make/model, then capture month',
  ),
  layout(
    'camera',
    'Camera model → Event',
    ['{camera}', '{event}'],
    ['Sony-A7-IV', 'Lisbon-Trip'],
    'Name this event for the whole-library preview',
  ),
  layout(
    'camera',
    'Camera model → File format',
    ['{camera}', '{file_format}'],
    ['Sony-A7-IV', 'RAW'],
    'Group indexed formats beneath each camera',
  ),
];
