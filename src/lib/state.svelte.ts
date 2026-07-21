// Shared app state (Svelte 5 runes). A single module-level object holds the open
// project and its editable rule set; both routes import it so navigation preserves
// state without a server round-trip.

import {
  computeEffective,
  type EffectivePolicy,
  type PolicyRule,
  type ProjectView,
  type ScopeName,
  type ScopedRulesDto
} from './ipc';

function emptyScoped(): ScopedRulesDto {
  return {
    managed: { rules: [], extraDeny: [], enforceManagedOnly: false },
    user: { rules: [], extraDeny: [], enforceManagedOnly: false },
    project: { rules: [], extraDeny: [], enforceManagedOnly: false },
    local: { rules: [], extraDeny: [], enforceManagedOnly: false }
  };
}

export const app = $state({
  loaded: false,
  projectId: '',
  projectRoot: '',
  projectName: '',
  view: null as ProjectView | null,
  scoped: emptyScoped(),
  activeScope: 'project' as ScopeName,
  selectedPath: '',
  dirty: false,
  effective: [] as EffectivePolicy[]
});

export function reset() {
  app.loaded = false;
  app.projectId = '';
  app.projectRoot = '';
  app.projectName = '';
  app.view = null;
  app.scoped = emptyScoped();
  app.selectedPath = '';
  app.dirty = false;
  app.effective = [];
}

export function setProject(view: ProjectView, scoped: ScopedRulesDto) {
  app.loaded = true;
  app.view = view;
  app.projectId = view.project.id;
  app.projectRoot = view.project.path;
  app.projectName = view.project.name;
  app.scoped = scoped;
  app.selectedPath = '';
  app.dirty = false;
}

/** Upsert a rule for a path into the active scope (one rule per path+scope). */
export function setPolicy(path: string, policy: PolicyRule['policy'], appliesTo: PolicyRule['appliesTo']) {
  const bucket = app.scoped[app.activeScope];
  const idx = bucket.rules.findIndex((r) => r.path === path);
  const rule: PolicyRule = { path, policy, appliesTo };
  if (idx >= 0) bucket.rules[idx] = rule;
  else bucket.rules.push(rule);
  app.dirty = true;
}

/** Remove any explicit rule for a path in the active scope. */
export function clearPolicy(path: string) {
  const bucket = app.scoped[app.activeScope];
  bucket.rules = bucket.rules.filter((r) => r.path !== path);
  app.dirty = true;
}

/** Upsert a full rule into an explicit scope (one rule per path). */
export function upsertRule(scope: ScopeName, rule: PolicyRule) {
  const bucket = app.scoped[scope];
  const idx = bucket.rules.findIndex((r) => r.path === rule.path);
  if (idx >= 0) bucket.rules[idx] = rule;
  else bucket.rules.push(rule);
  app.dirty = true;
}

/** Remove the rule for `path` in an explicit scope. */
export function removeRule(scope: ScopeName, path: string) {
  const bucket = app.scoped[scope];
  bucket.rules = bucket.rules.filter((r) => r.path !== path);
  app.dirty = true;
}

/** Set the non-path capability denies (web/network block) for a scope. */
export function setExtraDeny(scope: ScopeName, list: string[]) {
  app.scoped[scope].extraDeny = list;
  app.dirty = true;
}

/** Merge a batch of rules into a scope (upsert by path). */
export function mergeRules(scope: ScopeName, rules: PolicyRule[]) {
  const bucket = app.scoped[scope];
  for (const rule of rules) {
    const idx = bucket.rules.findIndex((r) => r.path === rule.path);
    if (idx >= 0) bucket.rules[idx] = rule;
    else bucket.rules.push(rule);
  }
  app.dirty = true;
}

export async function refreshEffective() {
  app.effective = await computeEffective(app.scoped);
}
