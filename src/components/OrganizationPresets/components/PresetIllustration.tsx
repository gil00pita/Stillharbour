import { chakra } from '@chakra-ui/react';
import type { SystemStyleObject } from '@chakra-ui/react';

// Explicit example paths: geometry and text only, with no external SVG assets.
export function PresetIllustration({ paths, css }: { paths: string[]; css: SystemStyleObject }) {
  return (
    <chakra.svg viewBox="0 0 280 120" aria-hidden="true" focusable="false" css={css}>
      {paths.slice(1).map((part, index) => (
        <path
          key={`${part}-${index}`}
          d={`M${17 + index * 20} ${27 + index * 25}v25h12`}
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
          opacity="0.45"
        />
      ))}
      {paths.map((path, index) => {
        const x = 6 + index * 20;
        const y = 9 + index * 25;
        const file = index === paths.length - 1;
        return (
          <g key={`${path}-${index}`} transform={`translate(${x} ${y})`}>
            {file ? (
              <>
                <path
                  d="M2 0h9l5 5v15H2Z M11 0v5h5"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.3"
                />
                <path d="m5 15 3-4 2 2 2-3 2 5Z" fill="currentColor" opacity="0.5" />
              </>
            ) : (
              <path
                d="M1 4V1h7l3 3h9v15H1Z"
                fill="currentColor"
                fillOpacity="0.12"
                stroke="currentColor"
                strokeWidth="1.3"
              />
            )}
            <text x="27" y="14" fill="currentColor" fontSize="11" fontFamily="inherit">
              {path}
            </text>
          </g>
        );
      })}
    </chakra.svg>
  );
}
