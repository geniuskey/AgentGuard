// Thin typed wrapper over Tauri's `invoke`. Falls back to mock data when running
// in a plain browser (e.g. `vite dev` without the Tauri shell) so the UI can be
// developed without building the desktop app.

export type Policy = 'allow' | 'ask' | 'deny';
export type ScopeName = 'managed' | 'user' | 'project' | 'local';
export type AppliesTo = 'file' | 'folder' | 'folder-and-children' | 'pattern';
export type Tool = 'Read' | 'Edit';

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
  /** Non-path tool denies toggled on (web/network capability block). */
  extraDeny: string[];
  /** Administrator policy makes managed allow/ask/deny rules exclusive. */
  enforceManagedOnly?: boolean;
}

export interface ScopedRulesDto {
  /** Administrator policy loaded from the local managed file tier; always read-only. */
  managed: ScopeRules;
  user: ScopeRules;
  project: ScopeRules;
  local: ScopeRules;
}

export interface DirEntry {
  path: string;
  name: string;
  isDir: boolean;
  excluded: boolean;
  /** Matched by the project's .gitignore — invisible to the agent's Grep search. */
  ignored: boolean;
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
  sensitivePaths: SensitivePathRecord[];
  risk: RiskScore;
  hasProjectSettings: boolean;
  hasLocalSettings: boolean;
}

export interface SensitivePathRecord {
  id: string;
  projectId: string;
  path: string;
  source: string;
  dismissed: boolean;
}

export interface ClaudeProjectTrustStatus {
  entryFound: boolean;
  accepted: boolean;
  sharedAllowRules: number;
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

export async function diagnosticReport(projectRoot?: string): Promise<string> {
  return invoke<string>('diagnostic_report', { projectRoot: projectRoot || null });
}

/** Claude Code trust state for shared project allow rules; no other app state is exposed. */
export async function claudeProjectTrustStatus(
  projectRoot: string
): Promise<ClaudeProjectTrustStatus> {
  if (!inTauri()) return { entryFound: false, accepted: false, sharedAllowRules: 0 };
  return invoke<ClaudeProjectTrustStatus>('claude_project_trust_status', { projectRoot });
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

export async function previewBackup(backupId: string): Promise<string> {
  return invoke<string>('preview_backup', { backupId });
}

export async function restoreBackup(backupId: string): Promise<void> {
  return invoke<void>('restore_backup', {
    backupId,
    timestamp: backupTimestamp()
  });
}

export async function listSensitivePaths(projectId: string): Promise<SensitivePathRecord[]> {
  return invoke<SensitivePathRecord[]>('list_sensitive_paths', { projectId });
}

export async function setSensitivePathDismissed(
  projectId: string,
  id: string,
  dismissed: boolean
): Promise<void> {
  return invoke<void>('set_sensitive_path_dismissed', { projectId, id, dismissed });
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

/** Is this path matched by the project's top-level .gitignore? */
export async function pathIgnored(projectRoot: string, relPath: string): Promise<boolean> {
  return invoke<boolean>('path_ignored', { projectRoot, relPath });
}

/**
 * Note an accessible git-ignored path in CLAUDE.md (read by explicit path /
 * `rg --no-ignore`). Returns false when the note already exists.
 */
export async function noteIgnoredPath(projectRoot: string, relPath: string): Promise<boolean> {
  return invoke<boolean>('note_ignored_path', { projectRoot, relPath });
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
  return invoke<string | null>('save_export_file', {
    kind: 'policy-template',
    defaultName,
    contents: text
  });
}

/** Import a rule set from a user-chosen `.json` file. Returns the parsed rules, or null if cancelled. */
export async function importTemplate(): Promise<ScopedRulesDto | null> {
  const selected = await invoke<{ path: string; text: string } | null>('pick_policy_template');
  if (!selected) return null;
  return invoke<ScopedRulesDto>('import_template', { text: selected.text });
}

/** Save Markdown report text to a user-chosen file. Returns the path or null. */
export async function saveReportFile(markdown: string, defaultName: string): Promise<string | null> {
  return invoke<string | null>('save_export_file', {
    kind: 'policy-report',
    defaultName,
    contents: markdown
  });
}

// --- Multi-agent global settings hub ----------------------------------------

export interface AgentGlobal {
  id: string;
  name: string;
  description: string;
  path: string;
  format: 'json' | 'toml';
  structured: boolean;
  route: string;
  exists: boolean;
}

export async function listAgentGlobals(): Promise<AgentGlobal[]> {
  if (!inTauri()) return [];
  return invoke<AgentGlobal[]>('list_agent_globals');
}

export async function getAgentGlobal(id: string): Promise<AgentGlobal> {
  return invoke<AgentGlobal>('get_agent_global', { id });
}

export async function readAgentConfig(agentId: string): Promise<string> {
  return invoke<string>('read_agent_config', { agentId });
}

export async function validateConfig(text: string, format: string): Promise<string | null> {
  return invoke<string | null>('validate_config', { text, format });
}

export async function saveAgentConfig(args: {
  text: string;
  agentId: string;
}): Promise<SaveResult> {
  return invoke<SaveResult>('save_agent_config', {
    text: args.text,
    agentId: args.agentId,
    timestamp: backupTimestamp()
  });
}

/** Convert an absolute folder path into a Claude Code glob pattern (`~/rel/**`). */
export async function homeRelativePattern(path: string): Promise<string> {
  return invoke<string>('home_relative_pattern', { path });
}

// --- System explorer (all drives) --------------------------------------------

export interface SystemEntry {
  name: string;
  /** Absolute OS path. */
  path: string;
  /** Claude pattern base (`~/x`, `//c/x`) — append `/**` for folder rules. */
  pattern: string;
  isDir: boolean;
}

/** Explorer roots: home folder + every mounted drive. */
export async function listDrives(): Promise<SystemEntry[]> {
  if (!inTauri()) return [];
  return invoke<SystemEntry[]>('list_drives');
}

/** List one directory anywhere on the machine (read-only), folders first. */
export async function listSystemDir(path: string): Promise<SystemEntry[]> {
  return invoke<SystemEntry[]>('list_system_dir', { path });
}

/** Claude Code security baseline as neutral Deny rules. */
export async function securityBaselineRules(): Promise<PolicyRule[]> {
  return invoke<PolicyRule[]>('security_baseline_rules');
}

/** Merge an agent's security baseline into its current config text (Codex/OpenCode). */
export async function securityBaseline(agentId: string, currentText: string): Promise<string> {
  return invoke<string>('security_baseline', { agentId, currentText });
}

/** One security-relevant setting parsed from an agent config. */
export interface AgentSecItem {
  label: string;
  value: string;
  /** true = 안전, false = 주의, null = 정보성 */
  ok: boolean | null;
}

/** Security summary of an agent config text (Codex/OpenCode). Throws on parse errors. */
export async function agentSecurityStatus(agentId: string, text: string): Promise<AgentSecItem[]> {
  return invoke<AgentSecItem[]>('agent_security_status', { agentId, text });
}

/** Parse an agent config text (JSON or TOML) into a plain JSON tree. */
export async function configGet(text: string, format: string): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('config_get', { text, format });
}

/**
 * Set one dotted-path key in a config text (null removes the key, pruning empty
 * parents). Returns the new full text; all other keys are preserved.
 */
export async function configSetValue(
  text: string,
  format: string,
  path: string,
  value: unknown
): Promise<string> {
  return invoke<string>('config_set_value', { text, format, path, value: value ?? null });
}

/** One lint finding on Claude Code settings text (known-key type/enum + secret checks). */
export interface LintItem {
  level: 'warn' | 'info';
  /** Dotted key path, e.g. `env.MY_TOKEN` or `permissions.defaultMode`. */
  path: string;
  message: string;
}

export async function lintClaudeSettings(text: string): Promise<LintItem[]> {
  if (!inTauri()) return [];
  return invoke<LintItem[]>('lint_claude_settings', { text });
}

/** The web/network deny specifiers a "block web access" toggle applies (Claude Code). */
export async function webBlockSpecifiers(): Promise<string[]> {
  if (!inTauri()) {
    return [
      'WebSearch',
      'WebFetch',
      'Bash(curl:*)',
      'Bash(wget:*)',
      'PowerShell(Invoke-WebRequest *)',
      'PowerShell(Invoke-RestMethod *)',
      'PowerShell(Start-BitsTransfer *)',
      'PowerShell(iwr *)',
      'PowerShell(irm *)',
      'PowerShell(curl *)',
      'PowerShell(wget *)',
      'PowerShell(curl.exe *)',
      'PowerShell(wget.exe *)'
    ];
  }
  return invoke<string[]>('web_block_specifiers');
}

// --- Policy simulator ---------------------------------------------------------

export interface SimMatch {
  scope: ScopeName;
  list: Policy;
  rule: string;
  decisive: boolean;
}

export interface SimResult {
  query: string;
  kind: 'path' | 'command';
  decision: Policy;
  matches: SimMatch[];
  fallback: boolean;
}

/**
 * Simulate a query. `path` evaluates the current editor rules (unsaved edits
 * included); `command` evaluates the selected shell tool's saved rules.
 */
export async function simulateAccess(
  projectRoot: string,
  scoped: ScopedRulesDto,
  query: string,
  kind: 'path' | 'command',
  shellTool: 'Bash' | 'PowerShell' = 'PowerShell'
): Promise<SimResult> {
  return invoke<SimResult>('simulate_access', { projectRoot, scoped, query, kind, shellTool });
}

// --- Agent surface: hooks & MCP servers ----------------------------------------

export interface HookEntry {
  scope: ScopeName;
  event: string;
  matcher: string | null;
  handlerType: 'command' | 'prompt' | 'agent' | 'http' | 'mcp' | string;
  command: string;
  riskLevel: 'medium' | 'high';
  usesWeb: boolean;
}

export interface McpServer {
  name: string;
  source: string;
  transport: string;
  target: string;
  /** Likely talks to the internet (remote transport or web-fetching stdio server). */
  usesWeb: boolean;
  active: boolean;
  statusReason: string;
}

export interface AgentSurface {
  hooks: HookEntry[];
  mcpServers: McpServer[];
}

export async function inspectAgentSurface(projectRoot: string): Promise<AgentSurface> {
  if (!inTauri()) return { hooks: [], mcpServers: [] };
  return invoke<AgentSurface>('inspect_agent_surface', { projectRoot });
}

// --- External-change watcher ----------------------------------------------------

export async function watchProject(projectRoot: string): Promise<void> {
  if (!inTauri()) return;
  return invoke<void>('watch_project', { projectRoot });
}

export async function unwatchProject(): Promise<void> {
  if (!inTauri()) return;
  return invoke<void>('unwatch_project');
}

/** Subscribe to settings-file change events; returns an unlisten function. */
export async function onSettingsFileChanged(
  cb: (path: string) => void
): Promise<() => void> {
  if (!inTauri()) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<string>('settings-file-changed', (e) => cb(e.payload));
}
