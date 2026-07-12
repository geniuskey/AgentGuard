// Light/dark theme: applies CSS via <html data-theme>, syncs the native window
// (title bar) theme on Windows, and persists the choice in localStorage.
import { inTauri } from './ipc';

export type ThemeName = 'dark' | 'light';

const KEY = 'ag-theme';

export const themeState = $state({ current: 'dark' as ThemeName });

export async function applyTheme(t: ThemeName) {
  themeState.current = t;
  document.documentElement.dataset.theme = t;
  try {
    localStorage.setItem(KEY, t);
  } catch {
    /* storage unavailable — theme just won't persist */
  }
  if (inTauri()) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().setTheme(t);
    } catch {
      /* runtime without set_theme — CSS theme still applies */
    }
  }
}

export function initTheme() {
  let saved: string | null = null;
  try {
    saved = localStorage.getItem(KEY);
  } catch {
    /* ignore */
  }
  applyTheme(saved === 'light' ? 'light' : 'dark');
}

export function toggleTheme() {
  applyTheme(themeState.current === 'dark' ? 'light' : 'dark');
}
