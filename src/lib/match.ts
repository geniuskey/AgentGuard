// Client-side approximation of rule matching for the explorer tree — colors
// every folder/file by whether the agent could access it, including rules
// inherited from ancestor folders. Display-only; the authoritative merge
// lives in the Rust core (effective.rs).
import type { PolicyRule } from './ipc';

export type EffectiveState = 'allow' | 'ask' | 'deny' | 'none';

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

/** The concrete pattern a rule matches, expanding its `appliesTo` form. */
export function rulePattern(r: PolicyRule): string {
  switch (r.appliesTo) {
    case 'folder-and-children':
      return r.path + '/**';
    case 'folder':
      return r.path + '/*';
    default:
      return r.path;
  }
}

/** Does `r` cover `target` (the rule's own path always matches itself)? */
export function matchesRule(r: PolicyRule, target: string): boolean {
  return r.path === target || ruleMatches(rulePattern(r), target);
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
 * Effective display state for one path: deny > ask > allow (mirrors the
 * authoritative merge in effective.rs); with no match the path follows Claude
 * Code's default behavior (`none` — prompt when needed). Rules' `appliesTo`
 * forms are expanded, so children inherit folder rules.
 */
export function effectiveDisplay(rules: PolicyRule[], target: string): EffectiveDisplay {
  for (const policy of ['deny', 'ask', 'allow'] as const) {
    const hit = rules.find((r) => r.policy === policy && matchesRule(r, target));
    if (hit) return { state: policy, source: hit.path };
  }
  return { state: 'none', source: null };
}
