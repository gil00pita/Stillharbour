import { Box, Text, IconButton } from '@chakra-ui/react';
import type { ReactNode, RefObject } from 'react';

const icons = {
  out: (
    <>
      <circle cx="10" cy="10" r="6" />
      <path d="m14.5 14.5 5 5M7 10h6" />
    </>
  ),
  in: (
    <>
      <circle cx="10" cy="10" r="6" />
      <path d="m14.5 14.5 5 5M7 10h6m-3-3v6" />
    </>
  ),
  fit: <path d="M8 4H4v4m12-4h4v4M4 16v4h4m12-4v4h-4" />,
  info: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v6m0-10v1" />
    </>
  ),
  filmstrip: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="1" />
      <path d="M7 4v16M17 4v16M3 9h4m-4 6h4m10-6h4m-4 6h4" />
    </>
  ),
} satisfies Record<string, ReactNode>;
function ToolIcon({ name }: { name: keyof typeof icons }) {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {icons[name]}
    </svg>
  );
}
export function ViewerControls({
  zoom,
  canZoom,
  onZoom,
  info,
  filmstrip,
  onInfo,
  onFilmstrip,
  infoButtonRef,
  favorite,
}: {
  zoom: number;
  canZoom: boolean;
  onZoom: (zoom: number) => void;
  info: boolean;
  filmstrip: boolean;
  onInfo: () => void;
  onFilmstrip: () => void;
  infoButtonRef: RefObject<HTMLButtonElement | null>;
  favorite: ReactNode;
}) {
  return (
    <Box
      className="viewer-controls viewer-zoom"
      css={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexWrap: 'nowrap',
        gap: '4px',
        width: 'max-content',
        maxWidth: '100%',
        padding: '8px',
        borderRadius: '12px',
        background: '{colors.bg.panel}',
        boxShadow: '0 6px 24px rgb(0 0 0 / 0.28)',
        '& button:hover:not(:disabled)': {
          background: '{colors.border}',
          color: '{colors.brand.solid}',
        },
        "& button[aria-pressed='true']": {
          background: '{colors.border}',
          color: '{colors.brand.solid}',
        },
        "& input[type='range']": {
          width: 'clamp(40px, 6vw, 80px)',
          minWidth: '24px',
          minHeight: '24px',
          margin: '0',
          padding: '0',
          border: '0',
          background: 'transparent',
          accentColor: '{colors.brand.solid}',
        },
        '& output': {
          minWidth: '64px',
          flexShrink: '0',
          whiteSpace: 'nowrap',
          fontSize: '12px',
          color: '{colors.fg.muted}',
          textAlign: 'center',
        },
      }}
      role="group"
      aria-label="Photo controls"
    >
      <IconButton
        unstyled
        type="submit"
        aria-label="Zoom out"
        title="Zoom out"
        disabled={zoom <= 1 || !canZoom}
        onClick={() => onZoom(Math.max(1, zoom - 0.25))}
      >
        <ToolIcon name="out" />
      </IconButton>
      <input
        type="range"
        aria-label="Preview zoom"
        min="100"
        max="400"
        step="25"
        value={zoom * 100}
        disabled={!canZoom}
        onChange={(event) => onZoom(Number(event.target.value) / 100)}
      />
      <output>{Math.round(zoom * 100)}% of fit</output>
      <IconButton
        unstyled
        type="submit"
        aria-label="Zoom in"
        title="Zoom in"
        disabled={zoom >= 4 || !canZoom}
        onClick={() => onZoom(Math.min(4, zoom + 0.25))}
      >
        <ToolIcon name="in" />
      </IconButton>
      <IconButton
        unstyled
        type="submit"
        aria-label="Fit image"
        title="Fit image"
        onClick={() => onZoom(1)}
      >
        <ToolIcon name="fit" />
      </IconButton>
      <Text
        as="span"
        className="viewer-control-divider"
        css={{ height: '24px', borderLeft: '1px solid {colors.border}', margin: '0 4px' }}
        aria-hidden="true"
      />
      {favorite}
      <IconButton
        unstyled
        type="submit"
        ref={infoButtonRef}
        aria-label="Info"
        title="Photo information"
        aria-pressed={info}
        onClick={onInfo}
      >
        <ToolIcon name="info" />
      </IconButton>
      <IconButton
        unstyled
        type="submit"
        aria-label="Filmstrip"
        title={filmstrip ? 'Hide filmstrip' : 'Show filmstrip'}
        aria-pressed={filmstrip}
        onClick={onFilmstrip}
      >
        <ToolIcon name="filmstrip" />
      </IconButton>
    </Box>
  );
}
