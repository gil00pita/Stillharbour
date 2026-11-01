// Presentation geometry only. Byte strings remain lossless in the table.
export type MapValue = { id: string; label: string; bytes: string };
export type MapTile = MapValue & { x: number; y: number; width: number; height: number };

export function byteValue(value: string): bigint {
  if (!/^(0|[1-9]\d{0,38})$/.test(value)) throw new Error('Invalid retained byte count.');
  const bytes = BigInt(value);
  if (bytes > (1n << 128n) - 1n) throw new Error('Retained byte count exceeds its bound.');
  return bytes;
}

export function mapTiles(values: MapValue[], width = 100, height = 50): MapTile[] {
  if (
    values.length > 512 ||
    !Number.isFinite(width) ||
    !Number.isFinite(height) ||
    width <= 0 ||
    height <= 0
  )
    throw new Error('The storage map exceeds its display boundary.');
  const seen = new Set<string>();
  const entries = values
    .map((value) => {
      if (seen.has(value.id)) throw new Error('Repeated storage-map identity.');
      seen.add(value.id);
      return { value, weight: byteValue(value.bytes) };
    })
    .filter((entry) => entry.weight > 0n)
    .sort((a, b) =>
      a.weight === b.weight ? (a.value.id < b.value.id ? -1 : 1) : a.weight > b.weight ? -1 : 1,
    );
  const tiles: MapTile[] = [];
  function place(items: typeof entries, x: number, y: number, w: number, h: number) {
    const first = items[0];
    if (!first) return;
    if (items.length === 1) {
      tiles.push({ ...first.value, x, y, width: w, height: h });
      return;
    }
    const total = items.reduce((sum, entry) => sum + entry.weight, 0n);
    let split = 1;
    let part = first.weight;
    while (split < items.length - 1 && part * 2n < total) {
      const next = items[split];
      if (!next) break;
      part += next.weight;
      split += 1;
    }
    const ratio = Number(part) / Number(total);
    if (w >= h) {
      place(items.slice(0, split), x, y, w * ratio, h);
      place(items.slice(split), x + w * ratio, y, w * (1 - ratio), h);
    } else {
      place(items.slice(0, split), x, y, w, h * ratio);
      place(items.slice(split), x, y + h * ratio, w, h * (1 - ratio));
    }
  }
  place(entries, 0, 0, width, height);
  return tiles;
}

export function remainingBytes(total: string, shown: string[]): string {
  const remaining = byteValue(total) - shown.reduce((sum, value) => sum + byteValue(value), 0n);
  if (remaining < 0n) throw new Error('The retained storage hierarchy does not balance.');
  return remaining.toString();
}
