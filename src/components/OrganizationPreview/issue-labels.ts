import type { OrganizationIssue } from '../../bindings';

export const issueLabels: Record<OrganizationIssue, string> = {
  missing_capture_date: 'No embedded capture date',
  missing_location: 'Required embedded location is missing or ambiguous',
  missing_tag: 'Required tag hierarchy is missing or ambiguous',
  missing_camera_metadata: 'Required embedded camera information is missing',
  missing_value: 'A required token value is missing',
  invalid_path: 'Name cannot be used under the portable naming policy',
  source_unavailable: 'Source is not currently recorded as available',
  non_unicode_path: 'Native name cannot be represented by this template',
  collision: 'Proposed paths collide',
  possible_case_collision: 'Proposed paths may collide on a case-insensitive destination',
};
