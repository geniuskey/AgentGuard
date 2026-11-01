// UI preferences persisted in localStorage: theme (with OS-follow mode) and
// sidebar collapse. Theme also syncs the native title bar on Windows.
import { inTauri } from './ipc';

export type ThemePref = 'dark' | 'light' | 'system';
export type ThemeName = 'dark' | 'light';

const THEME_KEY = 'ag-theme';
const SIDEBAR_KEY = 'ag-sidebar-collapsed';

export const prefs = $state({
  theme: 'dark' as ThemePref,
  resolved: 'dark' as ThemeName,
  sidebarCollapsed: false
});

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* storage unavailable — preference just won't persist */
  }
}

const media = () => window.matchMedia('(prefers-color-scheme: light)');

function resolve(pref: ThemePref): ThemeName {
  if (pref === 'system') return media().matches ? 'light' : 'dark';
  return pref;
}

async function paint(t: ThemeName) {
  prefs.resolved = t;
  document.documentElement.dataset.theme = t;
  if (inTauri()) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().setTheme(t);
    } catch {
      /* runtime without set_theme — CSS theme still applies */
    }
  }
}

export function setTheme(pref: ThemePref) {
  prefs.theme = pref;
  write(THEME_KEY, pref);
  paint(resolve(pref));
}

export function toggleTheme() {
  setTheme(prefs.resolved === 'dark' ? 'light' : 'dark');
}

export function setSidebarCollapsed(collapsed: boolean) {
  prefs.sidebarCollapsed = collapsed;
  write(SIDEBAR_KEY, collapsed ? '1' : '0');
}

export function toggleSidebar() {
  setSidebarCollapsed(!prefs.sidebarCollapsed);
}

/** Loads saved prefs; returns a cleanup for the OS theme listener. */
export function initPrefs(): () => void {
  const saved = read(THEME_KEY);
  prefs.theme = saved === 'light' || saved === 'system' ? saved : 'dark';
  const sidebar = read(SIDEBAR_KEY);
  prefs.sidebarCollapsed = sidebar === null ? window.innerWidth < 1280 : sidebar === '1';
  paint(resolve(prefs.theme));

  const mq = media();
  const onChange = () => {
    if (prefs.theme === 'system') paint(resolve('system'));
  };
  mq.addEventListener('change', onChange);
  return () => mq.removeEventListener('change', onChange);
}
