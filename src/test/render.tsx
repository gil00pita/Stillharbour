import { render as testingRender, type RenderOptions } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { Provider } from '../components/ui/Provider';

export { act, fireEvent, renderHook, screen, waitFor, within } from '@testing-library/react';
export function render(ui: ReactElement, options: RenderOptions = {}) {
  const { wrapper: Wrapper, ...rest } = options;
  function WithProviders({ children }: { children: ReactNode }) {
    return <Provider>{Wrapper ? <Wrapper>{children}</Wrapper> : children}</Provider>;
  }
  return testingRender(ui, { ...rest, wrapper: WithProviders });
}
