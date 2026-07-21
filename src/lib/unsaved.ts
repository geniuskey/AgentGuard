export const UNSAVED_CHANGES_MESSAGE =
  '저장하지 않은 변경 사항이 있습니다. 변경을 폐기하고 이동할까요?';

const unsavedSources = new Set<string>();

/** Keep one editor's dirty state in the application-wide unsaved registry. */
export function setUnsavedSource(id: string, dirty: boolean): void {
  if (dirty) unsavedSources.add(id);
  else unsavedSources.delete(id);
}

export function clearUnsavedSource(id: string): void {
  unsavedSources.delete(id);
}

export function isUnsavedSource(id: string): boolean {
  return unsavedSources.has(id);
}

export function hasUnsavedChanges(): boolean {
  return unsavedSources.size > 0;
}

/** Returns true when it is safe to continue with a destructive transition. */
export function confirmDiscardChanges(
  dirty: boolean,
  confirm: (message: string) => boolean,
  message = UNSAVED_CHANGES_MESSAGE
): boolean {
  return !dirty || confirm(message);
}

/** Browser beforeunload cannot show custom text, but returnValue triggers its native prompt. */
export function protectBeforeUnload(event: BeforeUnloadEvent, dirty: boolean): void {
  if (!dirty) return;
  event.preventDefault();
  event.returnValue = '';
}

/** Intended for route teardown and isolated tests, not for accepting a single editor's changes. */
export function clearAllUnsavedSources(): void {
  unsavedSources.clear();
}
