// Client-side approximation of rule matching for the explorer tree — colors
// every folder/file by whether the agent could access it, including rules
// inherited from ancestor folders. Display-only; the authoritative merge
// lives in the Rust core (effective.rs).
import type { PolicyRule } from './ipc';

export type EffectiveState = 'allow' | 'ask' | 'deny' | 'deny-default' | 'none';

export interface EffectiveDisplay {
  state: EffectiveState;
  /** The rule path that decided the state (null for defaults). */
  source: string | null;
}

function globToRegExp(glob: string): RegExp {
  let re = '';
  for (let i = 0; i < glob.length; i++) {
    const c = glob[i];
    if (c === '*') {
      if (glob[i + 1] === '*') {
        re += '.*';
        i++;
      } else {
        re += '[^/]*';
      }
    } else if (c === '?') {
      re += '.';
    } else {
      re += c.replace(/[.+^${}()|[\]\\]/, '\\$&');
    }
  }
  return new RegExp(`^${re}$`);
}

/** Does a rule pattern cover a target pattern base (folder itself included)? */
export function ruleMatches(rulePath: string, target: string): boolean {
  // `//**/X` means "X anywhere on any drive" — home paths included, so match
  // by suffix rather than by the (untranslatable) `//` prefix.
  const rp = rulePath.startsWith('//**/') ? '**/' + rulePath.slice(5) : rulePath;
  if (rp === '**' || rp === '//**') return true;

  if (rp.endsWith('/**')) {
    const base = rp.slice(0, -3);
    if (!/[*?]/.test(base)) return target === base || target.startsWith(base + '/');
  }
  try {
    return globToRegExp(rp).test(target);
  } catch {
    return false;
  }
}

/**
 * Effective display state for one path: deny rules win, then allow, then ask;
 * with no match the default mode decides (Default Deny → blocked).
 */
export function effectiveDisplay(
  rules: PolicyRule[],
  defaultMode: string | null,
  target: string
): EffectiveDisplay {
  for (const policy of ['deny', 'allow', 'ask'] as const) {
    const hit = rules.find((r) => r.policy === policy && ruleMatches(r.path, target));
    if (hit) return { state: policy, source: hit.path };
  }
  return defaultMode === 'dontAsk'
    ? { state: 'deny-default', source: null }
    : { state: 'none', source: null };
}
