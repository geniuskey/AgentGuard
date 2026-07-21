import { describe, expect, it } from 'vitest';
import { buildPolicyRule } from './policy-rule';

const base = {
  path: 'src',
  policy: 'allow' as const,
  appliesTo: 'folder-and-children' as const,
  useRead: true,
  useEdit: true,
  reason: '',
  riskLevel: '' as const,
  notes: ''
};

describe('buildPolicyRule', () => {
  it('uses null tools for the deterministic Read/Edit fan-out', () => {
    expect(buildPolicyRule(base)?.tools).toBeNull();
  });

  it('preserves one tool and trims SQLite-only annotations', () => {
    expect(
      buildPolicyRule({
        ...base,
        useEdit: false,
        reason: '  source work  ',
        riskLevel: 'medium',
        notes: '  reviewed  '
      })
    ).toMatchObject({
      tools: ['Read'],
      reason: 'source work',
      riskLevel: 'medium',
      notes: 'reviewed'
    });
  });

  it('rejects a rule without a path or selected tool', () => {
    expect(buildPolicyRule({ ...base, path: '' })).toBeNull();
    expect(buildPolicyRule({ ...base, useRead: false, useEdit: false })).toBeNull();
  });
});
