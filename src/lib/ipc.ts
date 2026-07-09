// Thin typed wrapper over Tauri's `invoke`. Falls back to mock data when running
// in a plain browser (e.g. `vite dev` without the Tauri shell) so the UI can be
// developed without building the desktop app.

export interface AppInfo {
  name: string;
  version: string;
  dataDir: string;
  dbSchemaVersion: number;
}

export interface RecentProject {
  projectPath: string;
  projectName: string;
  lastOpenedAt: string;
  riskProfile?: string;
  riskLevel?: string;
}

function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export async function appInfo(): Promise<AppInfo> {
  if (!inTauri()) {
    return {
      name: 'Agent Guard',
      version: '0.1.0 (browser mock)',
      dataDir: '(mock) %APPDATA%/AgentGuard',
      dbSchemaVersion: 1
    };
  }
  return invoke<AppInfo>('app_info');
}

export async function listRecentProjects(): Promise<RecentProject[]> {
  if (!inTauri()) return [];
  return invoke<RecentProject[]>('list_recent_projects');
}
