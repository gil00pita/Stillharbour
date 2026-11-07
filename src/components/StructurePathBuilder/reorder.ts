export function reorderFolders(folders: string[], from: number, to: number) {
  const parts = [...folders];
  const part = parts[from];
  if (part === undefined || to < 0 || to >= parts.length) return parts;
  parts.splice(from, 1);
  parts.splice(to, 0, part);
  return parts;
}

export function folderKeyDestination(key: string, current: number, count: number) {
  const destinations: Record<string, number> = {
    ArrowLeft: current - 1,
    ArrowUp: current - 1,
    ArrowRight: current + 1,
    ArrowDown: current + 1,
    Home: 0,
    End: count - 1,
  };
  const next = destinations[key];
  return next === undefined ? null : Math.max(0, Math.min(count - 1, next));
}

/** Find the closest level in either axis, including chips on wrapped rows. */
export function folderAtPoint(container: HTMLElement, x: number, y: number) {
  const bounds = container.getBoundingClientRect();
  if (x < bounds.left || x > bounds.right || y < bounds.top || y > bounds.bottom) return -1;
  let closest = -1;
  let distance = Infinity;
  container.querySelectorAll<HTMLElement>('[data-folder-index]').forEach((element) => {
    const rect = element.getBoundingClientRect();
    const dx = Math.max(rect.left - x, 0, x - rect.right);
    const dy = Math.max(rect.top - y, 0, y - rect.bottom);
    const next = dx * dx + dy * dy;
    if (next < distance) {
      closest = Number(element.dataset.folderIndex);
      distance = next;
    }
  });
  return closest;
}

export function scrollFolderEdge(container: HTMLElement, x: number, y: number) {
  const bounds = container.getBoundingClientRect();
  if (x < bounds.left || x > bounds.right) return false;
  for (let parent = container.parentElement; parent; parent = parent.parentElement) {
    if (!/(auto|scroll)/.test(getComputedStyle(parent).overflowY)) continue;
    const rect = parent.getBoundingClientRect();
    const edge = 32;
    const delta = y < rect.top + edge ? -8 : y > rect.bottom - edge ? 8 : 0;
    const before = parent.scrollTop;
    parent.scrollTop += delta;
    return parent.scrollTop !== before;
  }
  return false;
}
