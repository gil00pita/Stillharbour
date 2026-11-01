import type { SavedTemplate, TemplateSample } from '../bindings';

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
    };
  });
}
