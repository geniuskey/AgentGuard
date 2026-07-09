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

// --- Iteration 2+ -----------------------------------------------------------

export interface BackupRecord {
  id: string;
  projectId: string | null;
  scope: string;
  originalPath: string;
  backupPath: string;
  createdAt: string;
}

export interface EnvVar {
  name: string;
  present: boolean;
  display: string;
  isSecret: boolean;
}

export interface EnvStatus {
  vars: EnvVar[];
  hasSecretInEnv: boolean;
  usesProfile: boolean;
}

export interface GitignoreStatus {
  exists: boolean;
  ignored: boolean;
}

export interface ProfilePlan {
  defaultMode: string | null;
  rules: PolicyRule[];
}

export async function readRawSettings(projectRoot: string, scope: ScopeName): Promise<string> {
  return invoke<string>('read_raw_settings', { projectRoot, scope });
}

export async function saveRawSettings(args: {
  projectRoot: string;
  projectId: string;
  scope: ScopeName;
  text: string;
  projectName: string;
}): Promise<SaveResult> {
  return invoke<SaveResult>('save_raw_settings', {
    projectRoot: args.projectRoot,
    projectId: args.projectId,
    scope: args.scope,
    text: args.text,
    timestamp: backupTimestamp(),
    projectName: args.projectName
  });
}

export async function validateJson(text: string): Promise<string | null> {
  return invoke<string | null>('validate_json', { text });
}

export async function listBackups(projectId: string): Promise<BackupRecord[]> {
  return invoke<BackupRecord[]>('list_backups', { projectId });
}

export async function previewBackup(backupPath: string): Promise<string> {
  return invoke<string>('preview_backup', { backupPath });
}

export async function restoreBackup(backupPath: string, targetPath: string): Promise<void> {
  return invoke<void>('restore_backup', {
    backupPath,
    targetPath,
    timestamp: backupTimestamp()
  });
}

export async function scanRecommendationRules(projectRoot: string): Promise<PolicyRule[]> {
  return invoke<PolicyRule[]>('scan_recommendation_rules', { projectRoot });
}

export async function applyProfile(projectRoot: string, profile: string): Promise<ProfilePlan> {
  return invoke<ProfilePlan>('apply_profile', { projectRoot, profile });
}

export async function getEnvStatus(): Promise<EnvStatus> {
  if (!inTauri()) {
    return { vars: [], hasSecretInEnv: false, usesProfile: false };
  }
  return invoke<EnvStatus>('get_env_status');
}

export async function gitignoreStatus(projectRoot: string): Promise<GitignoreStatus> {
  return invoke<GitignoreStatus>('gitignore_status', { projectRoot });
}

export async function addLocalToGitignore(projectRoot: string): Promise<boolean> {
  return invoke<boolean>('add_local_to_gitignore', { projectRoot });
}

export async function policyReport(args: {
  projectName: string;
  profile: string | null;
  scoped: ScopedRulesDto;
  riskScore: number;
  riskLevel: string;
}): Promise<string> {
  return invoke<string>('policy_report', args);
}

/** Export the current rule set to a user-chosen `.json` file. Returns the path, or null if cancelled. */
export async function exportTemplate(scoped: ScopedRulesDto, defaultName: string): Promise<string | null> {
  const text = await invoke<string>('export_template', { scoped });
  const { save } = await import('@tauri-apps/plugin-dialog');
  const path = await save({
    defaultPath: `${defaultName}-agentguard-template.json`,
    filters: [{ name: 'JSON', extensions: ['json'] }]
  });
  if (!path) return null;
  await invoke<void>('write_text_file', { path, contents: text });
  return path;
}

/** Import a rule set from a user-chosen `.json` file. Returns the parsed rules, or null if cancelled. */
export async function importTemplate(): Promise<ScopedRulesDto | null> {
  const { open } = await import('@tauri-apps/plugin-dialog');
  const path = await open({ multiple: false, filters: [{ name: 'JSON', extensions: ['json'] }] });
  if (typeof path !== 'string') return null;
  const text = await invoke<string>('read_text_file', { path });
  return invoke<ScopedRulesDto>('import_template', { text });
}

/** Save Markdown report text to a user-chosen file. Returns the path or null. */
export async function saveReportFile(markdown: string, defaultName: string): Promise<string | null> {
  const { save } = await import('@tauri-apps/plugin-dialog');
  const path = await save({
    defaultPath: `${defaultName}-policy-report.md`,
    filters: [{ name: 'Markdown', extensions: ['md'] }]
  });
  if (!path) return null;
  await invoke<void>('write_text_file', { path, contents: markdown });
  return path;
}
