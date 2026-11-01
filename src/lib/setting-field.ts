// Shared field model for the structured "일반 설정" forms (Claude Code, Codex,
// OpenCode). Values are raw config values; `undefined` = key absent, and
// committing `null` removes the key so the agent falls back to its own default.

export interface Choice {
  v: unknown;
  label: string;
  /** One-line explanation shown under the control when this choice is selected. */
  hint?: string;
  /** Selecting this value is risky — rendered in amber. */
  risky?: boolean;
}

export type FieldKind = 'toggle' | 'choice' | 'combo' | 'number' | 'text';

export interface FieldSpec {
  key: string;
  label: string;
  help?: string;
  kind: FieldKind;
  /** choice: the options. combo: preset chips (free input is always available). */
  choices?: Choice[];
  /** toggle: stored values for on/off. `off: null` = a plain switch (off removes the key). */
  on?: unknown;
  off?: unknown;
  /** What the agent does when the key is absent, e.g. '켬', '30일', 'medium'. */
  defaultText?: string;
  /** number: display unit; stored value = shown value × scale. */
  unit?: string;
  scale?: number;
  min?: number;
  /** number/text: store as a string (environment variables). */
  asString?: boolean;
  placeholder?: string;
  caution?: { when: (v: unknown) => boolean; text: string };
}

export interface FieldGroup {
  title: string;
  description?: string;
  fields: FieldSpec[];
}

export function getAt(tree: unknown, path: string): unknown {
  let cur: unknown = tree;
  for (const seg of path.split('.')) {
    if (cur === null || typeof cur !== 'object') return undefined;
    cur = (cur as Record<string, unknown>)[seg];
  }
  return cur;
}

/** Loose equality so env strings ('1') and config values (true) both match presets. */
export function sameValue(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a === undefined || a === null || b === undefined || b === null) return false;
  return String(a) === String(b);
}

export function toggleState(f: FieldSpec, v: unknown): 'on' | 'off' | 'default' {
  if (v === undefined || v === null) return 'default';
  if (sameValue(v, f.on ?? true)) return 'on';
  if (f.off === null) return String(v).toLowerCase() === 'true' ? 'on' : 'off';
  if (sameValue(v, f.off ?? false)) return 'off';
  return 'on';
}
