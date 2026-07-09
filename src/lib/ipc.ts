// Thin typed wrapper over Tauri's `invoke`. Falls back to mock data when running
// in a plain browser (e.g. `vite dev` without the Tauri shell) so the UI can be
// developed without building the desktop app.

export type Policy = 'allow' | 'ask' | 'deny';
export type ScopeName = 'user' | 'project' | 'local';
export type AppliesTo = 'file' | 'folder' | 'folder-and-children' | 'pattern';
export type Tool = 'Read' | 'Edit' | 'Write' | 'Grep' | 'Glob' | 'NotebookEdit';

export interface AppInfo {
  name: string;
  version: string;
  dataDir: string;
  dbSchemaVersion: number;
}

export interface PolicyRule {
  path: string;
  policy: Policy;
  appliesTo: AppliesTo;
  tools?: Tool[] | null;
  reason?: string | null;
  riskLevel?: 'low' | 'medium' | 'high' | null;
  notes?: string | null;
}

export interface ScopeRules {
  rules: PolicyRule[];
  defaultMode: string | null;
}

export interface ScopedRulesDto {
  user: ScopeRules;
  project: ScopeRules;
  local: ScopeRules;
}

export interface DirEntry {
  path: string;
  name: string;
  isDir: boolean;
  excluded: boolean;
}

export interface ScanResult {
  signals: Record<string, boolean>;
  denyCandidates: string[];
  allowCandidates: string[];
}

export interface RiskScore {
  score: number;
  level: 'Low' | 'Medium' | 'High';
}

export interface ProjectRecord {
  id: string;
  path: string;
  name: string;
  lastOpenedAt: string;
  riskProfile?: string | null;
  riskScore?: number | null;
  riskLevel?: string | null;
  notes?: string | null;
}

export interface ProjectView {
  project: ProjectRecord;
  tree: DirEntry[];
  scan: ScanResult;
  risk: RiskScore;
  hasProjectSettings: boolean;
  hasLocalSettings: boolean;
}

export interface EffectivePolicy {
  path: string;
  effective: Policy;
  sourceScope: ScopeName | null;
  explicit: boolean;
  conflict: boolean;
}

export interface Permissions {
  allow: string[];
  ask: string[];
  deny: string[];
}

export interface DiffView {
  path: string;
  before: string;
  after: string;
  changed: boolean;
}

export interface SaveResult {
  written: string;
  backup: string | null;
}

export function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

/** `YYYY-MM-DD_HHmmss` used in backup filenames. */
export function backupTimestamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, '0');
  return (
    `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}` +
    `_${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`
  );
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

export async function listRecentProjects(): Promise<ProjectRecord[]> {
  if (!inTauri()) return [];
  return invoke<ProjectRecord[]>('list_recent_projects');
}

/** Open a native folder picker; returns the chosen path or null. */
export async function pickFolder(): Promise<string | null> {
  if (!inTauri()) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === 'string' ? selected : null;
}

export async function openProject(path: string): Promise<ProjectView> {
  return invoke<ProjectView>('open_project', {
    path,
    timestamp: new Date().toISOString()
  });
}

export async function listDir(projectRoot: string, relDir: string): Promise<DirEntry[]> {
  return invoke<DirEntry[]>('list_dir', { projectRoot, relDir });
}

export async function loadSettings(projectRoot: string): Promise<ScopedRulesDto> {
  return invoke<ScopedRulesDto>('load_settings', { projectRoot });
}

export async function computeEffective(scoped: ScopedRulesDto): Promise<EffectivePolicy[]> {
  return invoke<EffectivePolicy[]>('compute_effective', { scoped });
}

export async function toSettingsPreview(rules: PolicyRule[]): Promise<Permissions> {
  return invoke<Permissions>('to_settings_preview', { rules });
}

export async function buildDiff(
  projectRoot: string,
  scope: ScopeName,
  scopeRules: ScopeRules
): Promise<DiffView> {
  return invoke<DiffView>('build_diff', { projectRoot, scope, scopeRules });
}

export async function saveSettings(args: {
  projectRoot: string;
  projectId: string;
  scope: ScopeName;
  scopeRules: ScopeRules;
  projectName: string;
}): Promise<SaveResult> {
  return invoke<SaveResult>('save_settings', {
    projectRoot: args.projectRoot,
    projectId: args.projectId,
    scope: args.scope,
    scopeRules: args.scopeRules,
    timestamp: backupTimestamp(),
    projectName: args.projectName
  });
}
