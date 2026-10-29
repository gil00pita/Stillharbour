// Only the retained renderer window is laid out. Bound each node's outgoing
// edges as well, so hostile aspect ratios cannot make layout quadratic.
const MAX_ROW_PHOTOS = 64;
export type JustifiedRow = { start: number; end: number; height: number };

export function justifiedLayout(
  ratios: readonly number[],
  width: number,
  target: number,
  gutter: number,
) {
  const costs = new Float64Array(ratios.length + 1).fill(Infinity);
  const next = new Uint32Array(ratios.length);
  const heights = new Float64Array(ratios.length);
  costs[ratios.length] = 0;
  for (let start = ratios.length - 1; start >= 0; start--) {
    let sum = 0;
    for (let end = start + 1; end <= Math.min(ratios.length, start + MAX_ROW_PHOTOS); end++) {
      sum += ratios[end - 1]!;
      const room = width - gutter * (end - start - 1);
      if (room <= 0) break;
      const fitted = room / sum;
      // Do not amortise one row's distortion cost across dozens of slivers.
      // A single full-width panorama may naturally be shorter than this floor.
      if (end > start + 1 && fitted < target * 0.5) break;
      const final = end === ratios.length;
      const height = Math.min(fitted, target * (final || end === start + 1 ? 1 : 1.25));
      const deviation = (height - target) / target;
      const unused = Math.max(0, (room - sum * height) / width);
      // A small row cost avoids leaving an orphan simply to perfect earlier
      // rows. Sparse final rows may stay ragged; interior whitespace costs more.
      const cost =
        costs[end]! + 0.2 + deviation ** 2 + 4 * deviation ** 4 + unused ** 2 * (final ? 0.5 : 4);
      if (cost < costs[start]!) {
        costs[start] = cost;
        next[start] = end;
        heights[start] = height;
      }
    }
  }
  const rows: JustifiedRow[] = [];
  for (let start = 0; start < ratios.length;) {
    const end = next[start]!;
    rows.push({ start, end, height: heights[start]! });
    start = end;
  }
  return rows;
}
