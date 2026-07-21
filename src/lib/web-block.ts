export type WebBlockState = 'off' | 'partial' | 'on';

function uniqueRules(rules: readonly string[]): string[] {
  return [...new Set(rules)];
}

/** Classify only the configured web rules; unrelated denies do not imply PARTIAL. */
export function webBlockState(
  extraDeny: readonly string[],
  webRules: readonly string[]
): WebBlockState {
  const expected = uniqueRules(webRules);
  if (expected.length === 0) return 'off';

  const configured = new Set(extraDeny);
  const count = expected.filter((rule) => configured.has(rule)).length;
  if (count === 0) return 'off';
  return count === expected.length ? 'on' : 'partial';
}

/**
 * Complete a missing/partial web block, or remove it when fully enabled.
 * Denies unrelated to the web rule set are preserved in their original order.
 */
export function toggleWebBlockRules(
  extraDeny: readonly string[],
  webRules: readonly string[]
): string[] {
  const expected = uniqueRules(webRules);
  if (expected.length === 0) return [...extraDeny];

  if (webBlockState(extraDeny, expected) === 'on') {
    const webRuleSet = new Set(expected);
    return extraDeny.filter((rule) => !webRuleSet.has(rule));
  }

  const next = [...extraDeny];
  const configured = new Set(extraDeny);
  for (const rule of expected) {
    if (!configured.has(rule)) next.push(rule);
  }
  return next;
}
