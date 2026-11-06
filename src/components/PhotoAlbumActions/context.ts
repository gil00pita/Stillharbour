import { createContext, useContext, type MouseEvent } from 'react';
import type { AssetSummary } from '../../bindings';
export type OpenMenu = (asset: AssetSummary, event: MouseEvent<HTMLElement>) => void;
export const AlbumActions = createContext<{ show: OpenMenu; activeId: string | null } | null>(null);
export function usePhotoAlbumMenu(asset: AssetSummary) {
  const open = useContext(AlbumActions);
  return open
    ? (event: MouseEvent<HTMLElement>) => {
        event.preventDefault();
        event.stopPropagation();
        open.show(asset, event);
      }
    : undefined;
}
