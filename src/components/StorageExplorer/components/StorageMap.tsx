import type { InspectionFinding } from '../../../bindings';
import { mapTiles } from '../../../inspection/treemap';
import { displayBytes } from '../../../inspection/format';
import { Button } from '../../ui/controls';
import { colorOf, totalBytes, type CameraRecords } from '../model';

export type FileGroup = {
  id: string;
  name: string;
  files: InspectionFinding[];
  path: string[] | null;
};
export function StorageMap({
  groups,
  mode,
  cameras,
  selected,
  select,
  selectGroup,
  openGroup,
}: {
  groups: FileGroup[];
  mode: string;
  cameras: CameraRecords;
  selected: string | null;
  select: (file: InspectionFinding) => void;
  selectGroup: (group: FileGroup) => void;
  openGroup: (group: FileGroup) => void;
}) {
  const tiles = mapTiles(
    groups.map((group) => ({ id: group.id, label: group.name, bytes: totalBytes(group.files) })),
  );
  return (
    <div className="storage-map" aria-label="Logical-byte storage map">
      {tiles.map((tile) => {
        const group = groups.find((group) => group.id === tile.id)!;
        const visible = group.files.filter((file) => file.bytes !== '0').slice(0, 100);
        const rest = group.files.filter((file) => file.bytes !== '0').slice(100);
        const children = mapTiles([
          ...visible.map((file) => ({ id: file.id, label: file.name, bytes: file.bytes })),
          ...(rest.length
            ? [
                {
                  id: 'aggregate',
                  label: `${rest.length} smaller files · open list`,
                  bytes: totalBytes(rest),
                },
              ]
            : []),
        ]);
        return (
          <div
            key={tile.id}
            className="storage-map-group"
            style={{
              left: `${tile.x}%`,
              top: `${tile.y * 2}%`,
              width: `${tile.width}%`,
              height: `${tile.height * 2}%`,
            }}
          >
            <Button
              aria-label={`${group.name}: ${displayBytes(tile.bytes)}, ${group.files.length} files`}
              title={`${group.name} · ${displayBytes(tile.bytes)} · Double-click to open`}
              onClick={() => selectGroup(group)}
              onDoubleClick={() => openGroup(group)}
            >
              {group.name} · {displayBytes(tile.bytes)}
            </Button>
            <div className="storage-map-files">
              {children.map((child) => {
                const file = visible.find((file) => file.id === child.id);
                return (
                  <Button
                    key={child.id}
                    className="storage-tile"
                    aria-label={`${child.label}: ${displayBytes(child.bytes)}${file ? ` · ${file.displayPath}` : ''}`}
                    aria-pressed={selected === child.id}
                    data-small={tile.width * tile.height * child.width * child.height < 400000}
                    style={{
                      left: `${child.x}%`,
                      top: `${child.y * 2}%`,
                      width: `${child.width}%`,
                      height: `${child.height * 2}%`,
                      background: file ? colorOf(file, mode, cameras) : 'var(--muted)',
                    }}
                    title={`${child.label} · ${displayBytes(child.bytes)}${file ? ` · ${file.displayPath}` : ''}`}
                    onClick={() => (file ? select(file) : openGroup(group))}
                  >
                    <span>{child.label}</span>
                    <small>{displayBytes(child.bytes)}</small>
                  </Button>
                );
              })}
            </div>
          </div>
        );
      })}
    </div>
  );
}
