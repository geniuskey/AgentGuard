import { describe, expect, it } from 'vitest';
import type { PolicyRule } from './ipc';
import { mergeReviewedRules } from './rule-merge';

const deny: PolicyRule = { path: 'src', policy: 'deny', appliesTo: 'folder-and-children', tools: ['Edit'], reason: 'manual' };
const allow: PolicyRule = { path: 'src', policy: 'allow', appliesTo: 'folder-and-children' };

describe('reviewed candidate merge', () => {
  it('preserves manual restrictions and annotations by default', () => {
    expect(mergeReviewedRules([deny], [allow])).toEqual([deny]);
  });
  it('requires explicit path replacement and preserves unrelated rules', () => {
    const other = { ...deny, path: 'private' };
    expect(mergeReviewedRules([deny, other], [allow], ['src'])).toEqual([other, allow]);
  });
  it('preserves distinct tool rules until replacement is selected', () => {
    const read = { ...allow, tools: ['Read'] as PolicyRule['tools'] };
    expect(mergeReviewedRules([deny, read], [allow])).toEqual([deny, read]);
    expect(mergeReviewedRules([deny, read], [allow], ['src'])).toEqual([allow]);
  });

});
