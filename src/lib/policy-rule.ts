import type { AppliesTo, Policy, PolicyRule, Tool } from './ipc';

export interface PolicyRuleDraft {
  path: string;
  policy: Policy;
  appliesTo: AppliesTo;
  useRead: boolean;
  useEdit: boolean;
  reason: string;
  riskLevel: '' | 'low' | 'medium' | 'high';
  notes: string;
}

/** Build the exact neutral rule sent over IPC from the Policy Editor form. */
export function buildPolicyRule(draft: PolicyRuleDraft): PolicyRule | null {
  if (!draft.path || (!draft.useRead && !draft.useEdit)) return null;
  const selectedTools: Tool[] = [];
  if (draft.useRead) selectedTools.push('Read');
  if (draft.useEdit) selectedTools.push('Edit');
  return {
    path: draft.path,
    policy: draft.policy,
    appliesTo: draft.appliesTo,
    tools: selectedTools.length === 2 ? null : selectedTools,
    reason: draft.reason.trim() || null,
    riskLevel: draft.riskLevel || null,
    notes: draft.notes.trim() || null
  };
}
