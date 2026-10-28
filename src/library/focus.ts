// Compact Inspector temporarily hides the virtualized collection. Allow its
// visible rows to mount again, then restore the invoking asset's button.
export function restoreAssetFocus(
  id: string,
  previous: HTMLElement | null,
  scrollPosition?: number,
): void {
  let remaining = 20;
  const startingFocus = document.activeElement;
  function restore() {
    const asset = Array.from(document.querySelectorAll<HTMLElement>('[data-asset-id]')).find(
      (element) => element.dataset.assetId === id,
    );
    const button = asset?.querySelector<HTMLButtonElement>('.asset-open');
    const target = button ?? (previous?.isConnected ? previous : null);
    if (
      document.activeElement !== startingFocus &&
      document.activeElement !== document.body &&
      document.activeElement !== target
    )
      return;
    if (target && target.getClientRects().length) {
      target.focus({ preventScroll: true });
      const viewport = document.querySelector<HTMLElement>('.asset-viewport');
      if (viewport && scrollPosition !== undefined) viewport.scrollTop = scrollPosition;
      return;
    }
    if (--remaining > 0) setTimeout(restore, 25);
    else document.querySelector<HTMLElement>('.asset-viewport')?.focus();
  }
  // WebKit completes dialog.close() focus scrolling in a later native task.
  // Use the existing retry interval before restoring a saved modal position.
  setTimeout(restore, scrollPosition === undefined ? 0 : 25);
}
