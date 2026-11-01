import type { PolicyRule } from './ipc';

/** Preserve all existing rules unless replacement of that path was explicitly selected. */
export function mergeReviewedRules(
  existing: PolicyRule[],
  candidates: PolicyRule[],
  replacePaths: readonly string[] = []
): PolicyRule[] {
  const replacements = new Set(replacePaths);
  const accepted = candidates.filter(
    (candidate) => replacements.has(candidate.path) || !existing.some((rule) => rule.path === candidate.path)
  );
  const acceptedPaths = new Set(accepted.map((rule) => rule.path));
  return [...existing.filter((rule) => !acceptedPaths.has(rule.path)), ...accepted];
}

export function scopeSignature(value: unknown): string {
  return JSON.stringify(value);
}
