import { folderParts } from './folder-parts';
export function pathParts(pattern: string) {
  const parts = pattern.split('/');
  const filename = parts.pop() || '{original_stem}.{ext}';
  return { folders: parts.filter(Boolean), filename };
}
export function withFolders(folders: string[], filename: string) {
  return [...folders, filename].join('/');
}

export function folderPartLabel(part: string) {
  return folderParts.find((option) => option.value === part)?.label ?? part;
}
export function folderChoices(draft: string): { label: string; value: string }[] {
  const literal = draft.trim();
  const query = literal.toLocaleLowerCase();
  const choices: { label: string; value: string }[] = folderParts.filter((part) =>
    part.label.toLocaleLowerCase().includes(query),
  );
  const safeLiteral =
    literal.length > 0 &&
    literal.length <= 128 &&
    !/[\\/{}<>:"|?*]/u.test(literal) &&
    !/\p{Cc}/u.test(literal) &&
    !['.', '..'].includes(literal) &&
    !literal.endsWith('.');
  if (safeLiteral && !folderParts.some((part) => part.label.toLocaleLowerCase() === query))
    choices.push({ label: `Folder “${literal}”`, value: literal });
  return choices;
}
