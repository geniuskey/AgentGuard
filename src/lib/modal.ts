export type ModalClose = () => void;

const FOCUSABLE_SELECTOR = [
  'button:not([disabled])',
  '[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])'
].join(',');

export function wrappedFocusIndex(current: number, count: number, backwards: boolean): number {
  if (count <= 0) return -1;
  if (current < 0) return backwards ? count - 1 : 0;
  return (current + (backwards ? -1 : 1) + count) % count;
}

function focusableElements(node: HTMLElement): HTMLElement[] {
  return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (element) => element.getAttribute('aria-hidden') !== 'true'
  );
}

/** Svelte action providing initial focus, Escape dismissal, focus trapping and focus recovery. */
export function modalFocus(node: HTMLElement, close: ModalClose) {
  let onClose = close;
  const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;

  function focusInitial() {
    const preferred = node.querySelector<HTMLElement>('[data-modal-initial]');
    (preferred ?? focusableElements(node)[0] ?? node).focus();
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== 'Tab') return;

    const focusable = focusableElements(node);
    if (focusable.length === 0) {
      event.preventDefault();
      node.focus();
      return;
    }

    const current = focusable.indexOf(document.activeElement as HTMLElement);
    const atBoundary = event.shiftKey ? current <= 0 : current === focusable.length - 1;
    if (!atBoundary && current >= 0) return;
    event.preventDefault();
    focusable[wrappedFocusIndex(current, focusable.length, event.shiftKey)]?.focus();
  }

  node.addEventListener('keydown', onKeydown);
  queueMicrotask(focusInitial);

  return {
    update(nextClose: ModalClose) {
      onClose = nextClose;
    },
    destroy() {
      node.removeEventListener('keydown', onKeydown);
      previousFocus?.focus();
    }
  };
}
