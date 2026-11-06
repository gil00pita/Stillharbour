// Photo-card interpretation of the owner's scanning-document animation.
export function PhotoScanningArt({ active }: { active: boolean }) {
  return (
    <svg
      className="photo-scanning-art"
      data-active={active}
      aria-hidden="true"
      focusable="false"
      width="302"
      height="215"
      viewBox="0 0 302 215"
      fill="none"
    >
      <ellipse cx="151" cy="189" rx="102" ry="8" fill="var(--scan-art-shadow)" opacity="0.45" />
      <g transform="rotate(-12 151 104)">
        <rect x="60" y="37" width="168" height="135" rx="12" fill="var(--scan-art-back)" />
        <rect x="70" y="47" width="148" height="101" rx="5" fill="var(--scan-art-lens)" />
      </g>
      <g className="photo-scanning-card">
        <rect
          x="70"
          y="34"
          width="168"
          height="140"
          rx="12"
          fill="var(--scan-art-frame)"
          stroke="var(--scan-art-outline)"
          strokeOpacity="0.5"
        />
        <rect x="82" y="46" width="144" height="103" rx="5" fill="var(--scan-art-sky)" />
        <circle cx="185" cy="71" r="11" fill="var(--scan-art-accent)" opacity="0.7" />
        <path
          d="m86 134 36-49a5 5 0 0 1 8 0l29 36 19-24a5 5 0 0 1 8 0l36 44H86Z"
          fill="var(--scan-art-accent)"
          opacity="0.45"
        />
        <path
          d="m86 134 36-49a5 5 0 0 1 8 0l29 36-17 20H86Z"
          fill="var(--scan-art-accent)"
          opacity="0.35"
        />
        <rect
          x="132"
          y="158"
          width="44"
          height="3"
          rx="1.5"
          fill="var(--scan-art-outline)"
          opacity="0.35"
        />
      </g>
      <g
        className="photo-scanning-lens"
        stroke="var(--scan-art-accent)"
        strokeWidth="6"
        strokeLinecap="round"
      >
        <circle cx="158" cy="105" r="27" fill="var(--scan-art-lens)" fillOpacity="0.3" />
        <path d="m178 125 22 22" strokeWidth="10" />
        <path d="M142 103a16 16 0 0 1 16-14" strokeWidth="3" opacity="0.65" />
      </g>
    </svg>
  );
}
