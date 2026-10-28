import type { THUMBNAIL_SIZES, ViewMode } from './model';

export function BrowserControls({
  mode,
  size,
  fit,
  onMode,
  onSize,
  onFit,
}: {
  mode: ViewMode;
  size: keyof typeof THUMBNAIL_SIZES;
  fit: 'contain' | 'cover';
  onMode: (mode: ViewMode) => void;
  onSize: (size: keyof typeof THUMBNAIL_SIZES) => void;
  onFit: (fit: 'contain' | 'cover') => void;
}) {
  return (
    <div className="view-switch" aria-label="Library view">
      {(['gallery', 'grid', 'list'] as const).map((view) => (
        <button type="button" key={view} aria-pressed={mode === view} onClick={() => onMode(view)}>
          {view === 'gallery' ? 'Timeline' : view === 'grid' ? 'Uniform grid' : 'Text list'}
        </button>
      ))}
      {mode !== 'list' && (
        <label className="thumbnail-control">
          Thumbnail size
          <select
            aria-label="Thumbnail size"
            value={size}
            onChange={(event) => {
              const value = event.target.value;
              if (value === 'compact' || value === 'comfortable' || value === 'large') {
                onSize(value);
              }
            }}
          >
            <option value="compact">Compact</option>
            <option value="comfortable">Comfortable</option>
            <option value="large">Large</option>
          </select>
        </label>
      )}
      {mode === 'grid' && (
        <label className="thumbnail-control">
          Image fit
          <select
            aria-label="Grid image fit"
            value={fit}
            onChange={(event) => onFit(event.target.value === 'cover' ? 'cover' : 'contain')}
          >
            <option value="contain">Fit whole image</option>
            <option value="cover">Crop to fill</option>
          </select>
        </label>
      )}
    </div>
  );
}
