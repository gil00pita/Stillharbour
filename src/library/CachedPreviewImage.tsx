import { useState, type ComponentProps } from 'react';

type Props = Omit<ComponentProps<'img'>, 'src'> & { src: string };

// A WebView transport failure is not a permanent decode failure. Retry the
// owned-cache URL once, bypassing any cached error; never read an original here.
export function CachedPreviewImage(props: Props) {
  return <RetryingImage key={props.src} {...props} />;
}
function RetryingImage({ src, onError, ...props }: Props) {
  const [attempt, setAttempt] = useState(0);
  return (
    <img
      {...props}
      src={attempt ? `${src}${src.includes('?') ? '&' : '?'}retry=1` : src}
      onError={(event) => {
        if (attempt === 0) setAttempt(1);
        else onError?.(event);
      }}
    />
  );
}
