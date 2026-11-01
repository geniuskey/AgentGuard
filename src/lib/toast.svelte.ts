export type ToastKind = 'ok' | 'info' | 'warn' | 'error';

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  action?: ToastAction;
}

let nextId = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export const toasts = $state<Toast[]>([]);

export function dismissToast(id: number) {
  clearTimeout(timers.get(id));
  timers.delete(id);
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

/** Errors stay until dismissed; everything else fades after a few seconds. */
export function toast(message: string, kind: ToastKind = 'ok', action?: ToastAction): number {
  const id = nextId++;
  toasts.push({ id, kind, message, action });
  if (toasts.length > 4) dismissToast(toasts[0].id);
  if (kind !== 'error') {
    timers.set(id, setTimeout(() => dismissToast(id), action ? 7000 : 3500));
  }
  return id;
}
