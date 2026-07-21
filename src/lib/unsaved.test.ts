import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  clearAllUnsavedSources,
  clearUnsavedSource,
  confirmDiscardChanges,
  hasUnsavedChanges,
  isUnsavedSource,
  protectBeforeUnload,
  setUnsavedSource
} from './unsaved';
import { wrappedFocusIndex } from './modal';

describe('unsaved editor registry', () => {
  beforeEach(clearAllUnsavedSources);

  it('tracks independent editors without clearing another dirty editor', () => {
    setUnsavedSource('project-rules', true);
    setUnsavedSource('raw-user', true);
    setUnsavedSource('project-rules', false);

    expect(isUnsavedSource('project-rules')).toBe(false);
    expect(isUnsavedSource('raw-user')).toBe(true);
    expect(hasUnsavedChanges()).toBe(true);

    clearUnsavedSource('raw-user');
    expect(hasUnsavedChanges()).toBe(false);
  });

  it('asks only for destructive transitions with unsaved content', () => {
    const confirm = vi.fn(() => false);

    expect(confirmDiscardChanges(false, confirm)).toBe(true);
    expect(confirm).not.toHaveBeenCalled();
    expect(confirmDiscardChanges(true, confirm)).toBe(false);
    expect(confirm).toHaveBeenCalledOnce();
  });

  it('marks beforeunload only while content is dirty', () => {
    const clean = { preventDefault: vi.fn(), returnValue: 'unchanged' } as unknown as BeforeUnloadEvent;
    protectBeforeUnload(clean, false);
    expect(clean.preventDefault).not.toHaveBeenCalled();
    expect(clean.returnValue).toBe('unchanged');

    const dirty = { preventDefault: vi.fn(), returnValue: 'unchanged' } as unknown as BeforeUnloadEvent;
    protectBeforeUnload(dirty, true);
    expect(dirty.preventDefault).toHaveBeenCalledOnce();
    expect(dirty.returnValue).toBe('');
  });
});

describe('modal focus wrapping', () => {
  it('wraps forwards and backwards at dialog boundaries', () => {
    expect(wrappedFocusIndex(2, 3, false)).toBe(0);
    expect(wrappedFocusIndex(0, 3, true)).toBe(2);
    expect(wrappedFocusIndex(-1, 3, false)).toBe(0);
    expect(wrappedFocusIndex(-1, 3, true)).toBe(2);
    expect(wrappedFocusIndex(0, 0, false)).toBe(-1);
  });
});
