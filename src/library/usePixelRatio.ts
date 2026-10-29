import { useEffect, useState } from 'react';
import { pixelRatio } from './renditionPlan';

export function usePixelRatio() {
  const [ratio, setRatio] = useState(() => pixelRatio(window.devicePixelRatio));
  useEffect(() => {
    const update = () => setRatio(pixelRatio(window.devicePixelRatio));
    // A display/zoom change may alter DPR without changing row CSS geometry.
    const media = window.matchMedia?.(`(resolution: ${window.devicePixelRatio}dppx)`);
    window.addEventListener('resize', update);
    media?.addEventListener('change', update);
    return () => {
      window.removeEventListener('resize', update);
      media?.removeEventListener('change', update);
    };
  }, [ratio]);
  return ratio;
}
