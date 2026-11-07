import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ButtonHTMLAttributes,
} from 'react';
import { folderAtPoint, folderKeyDestination, scrollFolderEdge } from './reorder';
import { folderPartLabel } from './path-parts';

type Selection = {
  from: number;
  to: number;
  pattern: string;
  kind: 'pick' | 'drag';
  ghost?: { x: number; y: number; width: number };
};
type Gesture = {
  pointer: number;
  from: number;
  pattern: string;
  handle: HTMLButtonElement;
  startX: number;
  startY: number;
  x: number;
  y: number;
  dragging: boolean;
  rect: DOMRect;
};

export function useFolderReorder(
  pattern: string,
  folders: string[],
  disabled: boolean,
  onReorder: (from: number, to: number) => void,
) {
  const container = useRef<HTMLDivElement>(null);
  const gesture = useRef<Gesture | null>(null);
  const focusAt = useRef<number | null>(null);
  const suppressClick = useRef(false);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [announcement, announce] = useState('');
  if (selection && (disabled || selection.pattern !== pattern)) setSelection(null);
  const current = !disabled && selection?.pattern === pattern ? selection : null;
  const releaseGesture = useCallback(() => {
    const drag = gesture.current;
    gesture.current = null;
    if (drag?.dragging) suppressClick.current = true;
    if (drag?.handle.hasPointerCapture?.(drag.pointer))
      drag.handle.releasePointerCapture(drag.pointer);
  }, []);
  const clear = useCallback(() => {
    releaseGesture();
    setSelection(null);
  }, [releaseGesture]);
  useLayoutEffect(() => releaseGesture, [pattern, disabled, releaseGesture]);
  useEffect(() => {
    window.addEventListener('blur', clear);
    return () => {
      window.removeEventListener('blur', clear);
    };
  }, [clear]);
  useLayoutEffect(() => {
    if (focusAt.current !== null) {
      container.current
        ?.querySelector<HTMLButtonElement>(`[data-folder-index="${focusAt.current}"]`)
        ?.focus({ preventScroll: true });
      focusAt.current = null;
    }
  });
  const updateDrag = useCallback((drag: Gesture) => {
    if (!container.current) return;
    const to = folderAtPoint(container.current, drag.x, drag.y);
    setSelection({
      from: drag.from,
      to,
      pattern: drag.pattern,
      kind: 'drag',
      ghost: {
        x: drag.rect.left + drag.x - drag.startX,
        y: drag.rect.top + drag.y - drag.startY,
        width: drag.rect.width,
      },
    });
  }, []);
  const dragging = current?.kind === 'drag';
  useEffect(() => {
    if (!dragging) return;
    let frame = 0;
    function scroll() {
      const drag = gesture.current;
      if (drag && container.current && scrollFolderEdge(container.current, drag.x, drag.y))
        updateDrag(drag);
      frame = requestAnimationFrame(scroll);
    }
    frame = requestAnimationFrame(scroll);
    return () => cancelAnimationFrame(frame);
  }, [dragging, updateDrag]);
  function commit(from: number, to: number) {
    clear();
    if (to < 0 || to >= folders.length) {
      announce('Move canceled. Folder order unchanged.');
      return;
    }
    focusAt.current = to;
    if (from !== to) onReorder(from, to);
    announce(
      `${folderPartLabel(folders[from] ?? '')} placed at level ${to + 1} of ${folders.length}.`,
    );
  }
  function activate(index: number) {
    if (disabled) return;
    if (current?.kind === 'pick') commit(current.from, current.from === index ? current.to : index);
    else {
      setSelection({ from: index, to: index, pattern, kind: 'pick' });
      announce(
        `${folderPartLabel(folders[index] ?? '')} picked up. Choose a destination grip, or use arrow keys and Enter. Escape cancels.`,
      );
    }
  }
  function handleProps(index: number): ButtonHTMLAttributes<HTMLButtonElement> {
    return {
      onPointerDown(event) {
        if (disabled || event.button !== 0 || event.isPrimary === false || gesture.current) return;
        suppressClick.current = false;
        event.currentTarget.focus({ preventScroll: true });
        gesture.current = {
          pointer: event.pointerId,
          from: index,
          pattern,
          handle: event.currentTarget,
          startX: event.clientX,
          startY: event.clientY,
          x: event.clientX,
          y: event.clientY,
          dragging: false,
          rect: event.currentTarget.getBoundingClientRect(),
        };
        if (event.isTrusted) event.currentTarget.setPointerCapture(event.pointerId);
      },
      onPointerMove(event) {
        const drag = gesture.current;
        if (!drag || drag.pointer !== event.pointerId || disabled || drag.pattern !== pattern)
          return;
        drag.x = event.clientX;
        drag.y = event.clientY;
        if (!drag.dragging && Math.hypot(drag.x - drag.startX, drag.y - drag.startY) < 5) return;
        drag.dragging = true;
        event.preventDefault();
        updateDrag(drag);
      },
      onPointerUp(event) {
        const drag = gesture.current;
        if (!drag || drag.pointer !== event.pointerId) return;
        if (drag.dragging) {
          suppressClick.current = true;
          const to = container.current
            ? folderAtPoint(container.current, event.clientX, event.clientY)
            : -1;
          if (!disabled && drag.pattern === pattern) commit(drag.from, to);
          else clear();
        } else {
          gesture.current = null;
        }
      },
      onPointerCancel: clear,
      onLostPointerCapture(event) {
        if (gesture.current?.pointer === event.pointerId) clear();
      },
      onClick() {
        if (suppressClick.current) {
          suppressClick.current = false;
          return;
        }
        activate(index);
      },
      onKeyDown(event) {
        suppressClick.current = false;
        if (event.key === 'Escape' || event.key === 'Tab') {
          if (!current && !gesture.current) return;
          if (event.key === 'Escape') {
            event.preventDefault();
            event.stopPropagation();
          }
          clear();
          announce('Move canceled. Folder order unchanged.');
        } else if (event.key === ' ' || event.key === 'Enter') {
          event.preventDefault();
          activate(index);
        } else if (current?.kind === 'pick') {
          const to = folderKeyDestination(event.key, current.to, folders.length);
          if (to === null) return;
          event.preventDefault();
          setSelection({ ...current, to });
          announce(`Destination level ${to + 1} of ${folders.length}. Press Enter to place.`);
        }
      },
    };
  }
  return { container, interaction: current, announcement, handleProps };
}
