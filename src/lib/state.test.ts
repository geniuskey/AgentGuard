import { beforeEach, describe, expect, it, vi } from 'vitest';
vi.mock('./ipc', () => ({ computeEffective: vi.fn().mockResolvedValue([]) }));
import { app, isScopeDirty, markScopeSaved, mergeRules, reset, setPolicy } from './state.svelte';

describe('scope save tracking', () => {
  beforeEach(reset);
  it('saving project keeps local changes dirty', () => {
    setPolicy('src', 'allow', 'folder-and-children');
    app.activeScope = 'local';
    setPolicy('.env', 'deny', 'file');
    markScopeSaved('project');
    expect(isScopeDirty('project')).toBe(false);
    expect(isScopeDirty('local')).toBe(true);
    expect(app.dirty).toBe(true);
  });
  it('tracks direct edits, reverts, and edits made while a save is pending', () => {
    setPolicy('src', 'allow', 'folder-and-children');
    const snapshot = JSON.parse(JSON.stringify(app.scoped.project));
    app.scoped.project.rules[0].policy = 'deny';
    markScopeSaved('project', snapshot);
    expect(app.dirty).toBe(true);
    app.scoped.project.rules[0].policy = 'allow';
    expect(app.dirty).toBe(false);
  });
  it('candidate merge cannot silently replace a manual deny', () => {
    setPolicy('src', 'deny', 'folder-and-children');
    mergeRules('project', [{ path: 'src', policy: 'allow', appliesTo: 'folder-and-children' }]);
    expect(app.scoped.project.rules[0].policy).toBe('deny');
  });
});
