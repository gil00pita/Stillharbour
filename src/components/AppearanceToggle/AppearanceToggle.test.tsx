import { act, fireEvent, render, screen } from '../../test/render';
import { afterEach, expect, it, vi } from 'vitest';
import { AppearanceToggle } from '@/components/AppearanceToggle';

afterEach(() => {
  vi.restoreAllMocks();
  localStorage.removeItem('stillharbour-appearance');
});

it.each([false, true])(
  'follows the system appearance until explicitly overridden (dark=%s)',
  (initialDark) => {
    let dark = initialDark;
    const listeners = new Set<NonNullable<Parameters<MediaQueryList['addListener']>[0]>>();
    const media: MediaQueryList = {
      get matches() {
        return dark;
      },
      media: '(prefers-color-scheme: dark)',
      onchange: null,
      addListener: (listener) => {
        if (listener) listeners.add(listener);
      },
      removeListener: (listener) => {
        if (listener) listeners.delete(listener);
      },
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: () => false,
    };
    function changeSystem() {
      dark = !dark;
      act(() => {
        const event = Object.assign(new Event('change'), { matches: dark, media: media.media });
        for (const listener of [...listeners]) listener.call(media, event);
      });
    }
    vi.spyOn(window, 'matchMedia').mockReturnValue(media);
    localStorage.removeItem('stillharbour-appearance');
    render(<AppearanceToggle />);
    expect(document.documentElement).toHaveClass(dark ? 'dark' : 'light');
    expect(localStorage.getItem('stillharbour-appearance')).toBeNull();
    changeSystem();
    expect(document.documentElement).toHaveClass(dark ? 'dark' : 'light');
    fireEvent.click(
      screen.getByRole('button', { name: `Switch to ${dark ? 'light' : 'dark'} appearance` }),
    );
    const saved = dark ? 'light' : 'dark';
    expect(localStorage.getItem('stillharbour-appearance')).toBe(saved);
    changeSystem();
    expect(document.documentElement).toHaveClass(saved);
    changeSystem();
    expect(document.documentElement).toHaveClass(saved);
  },
);

it('switches the document appearance and remembers it when the app remounts', () => {
  localStorage.setItem('stillharbour-appearance', 'dark');
  const view = render(<AppearanceToggle />);
  fireEvent.click(screen.getByRole('button', { name: 'Switch to light appearance' }));
  expect(document.documentElement).toHaveClass('light');
  expect(localStorage.getItem('stillharbour-appearance')).toBe('light');
  // The client-rendered desktop must not need an executable inline bootstrap.
  expect(document.querySelector('script:not([type="application/json"])')).toBeNull();
  view.unmount();
  render(<AppearanceToggle />);
  fireEvent.click(screen.getByRole('button', { name: 'Switch to dark appearance' }));
  expect(document.documentElement).toHaveClass('dark');
  expect(localStorage.getItem('stillharbour-appearance')).toBe('dark');
  localStorage.removeItem('stillharbour-appearance');
});
