import { createContext, useContext, type KeyboardEvent, type MouseEvent } from 'react';
import type { AssetSummary } from '../../bindings';
export type OpenMenu = (
  asset: AssetSummary,
  event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>,
) => void;
export type PhotoSelection = {
  active: boolean;
  disabled: boolean;
  onSelect: (asset: AssetSummary) => void;
};
export const AlbumActions = createContext<{
  show: OpenMenu;
  selection: PhotoSelection | undefined;
} | null>(null);
export function usePhotoAlbumMenu(asset: AssetSummary) {
  const context = useContext(AlbumActions);
  return context
    ? (event: MouseEvent<HTMLElement>) => {
        event.preventDefault();
        event.stopPropagation();
        context.show(asset, event);
      }
    : undefined;
}
export function usePhotoAlbumKeys(asset: AssetSummary) {
  const context = useContext(AlbumActions);
  return context
    ? (event: KeyboardEvent<HTMLElement>) => {
        if (event.key !== 'ContextMenu' && !(event.key === 'F10' && event.shiftKey)) return;
        event.preventDefault();
        event.stopPropagation();
        context.show(asset, event);
      }
    : undefined;
}
export function usePhotoSelection() {
  return useContext(AlbumActions)?.selection;
}
