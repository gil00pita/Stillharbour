import type { SavedTemplate, TemplateSample, OrganizationMetadata } from '../bindings';

export function retainedExamples(saved: SavedTemplate): TemplateSample[] {
  const { sampleInputs, sampleOutputs } = saved.policies;
  if (sampleInputs.length > 4 || sampleInputs.length !== sampleOutputs.length) {
    throw new Error('The retained template examples are invalid. Select another template.');
  }
  return sampleInputs.map((input, index) => {
    const value: unknown = JSON.parse(input);
    if (
      !value ||
      typeof value !== 'object' ||
      !('relativePath' in value) ||
      typeof value.relativePath !== 'string' ||
      !('capture' in value) ||
      !(value.capture === null || typeof value.capture === 'string') ||
      !('sequence' in value) ||
      typeof value.sequence !== 'number' ||
      !Number.isInteger(value.sequence) ||
      value.sequence < 1 ||
      value.sequence > 240 ||
      !('expectedTarget' in value) ||
      typeof value.expectedTarget !== 'string' ||
      value.expectedTarget !== sampleOutputs[index]
    ) {
      throw new Error('The retained template examples are invalid. Select another template.');
    }
    return {
      relativePath: value.relativePath,
      capture: value.capture,
      sequence: value.sequence,
      expectedTarget: value.expectedTarget,
      ...('metadata' in value && value.metadata !== undefined
        ? { metadata: retainedNamingMetadata(value.metadata) }
        : {}),
    };
  });
}

const namingKeys: readonly (keyof OrganizationMetadata)[] = [
  'country',
  'region',
  'city',
  'place',
  'tag',
  'subcategory',
  'subject',
  'cameraType',
  'brand',
  'model',
  'fileFormat',
];
export function retainedNamingMetadata(value: unknown): OrganizationMetadata {
  if (
    !value ||
    typeof value !== 'object' ||
    Array.isArray(value) ||
    Object.keys(value).some((key) => !namingKeys.some((known) => known === key))
  )
    throw new Error('The retained naming examples are invalid.');
  const result: OrganizationMetadata = {};
  for (const key of namingKeys) {
    const field: unknown = Reflect.get(value, key);
    if (field !== undefined) {
      if (typeof field !== 'string' || new TextEncoder().encode(field).length > 512)
        throw new Error('The retained naming examples are invalid.');
      result[key] = field;
    }
  }
  return result;
}
