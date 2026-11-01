// Svelte action: `use:tooltip={'설명'}`. Renders a fixed-position tooltip on
// hover or keyboard focus (Escape hides it) — immune to overflow clipping inside scrollable panels, flips above
// when there is no room below. Pass undefined/'' to disable.

export function tooltip(node: HTMLElement, text: string | undefined) {
  let el: HTMLDivElement | null = null;
  let timer: number | undefined;
  let current = text;

  function show() {
    if (!current || el) return;
    el = document.createElement('div');
    el.className = 'ag-tooltip';
    el.textContent = current;
    document.body.appendChild(el);
    const r = node.getBoundingClientRect();
    const tw = el.offsetWidth;
    const th = el.offsetHeight;
    let top = r.bottom + 6;
    if (top + th > window.innerHeight - 8) top = Math.max(8, r.top - th - 6);
    const left = Math.max(8, Math.min(r.left, window.innerWidth - tw - 8));
    el.style.top = `${top}px`;
    el.style.left = `${left}px`;
  }

  function scheduleShow() {
    window.clearTimeout(timer);
    timer = window.setTimeout(show, 300);
    document.addEventListener('keydown', onKeydown, true);
  }

  function hide() {
    window.clearTimeout(timer);
    el?.remove();
    el = null;
    document.removeEventListener('keydown', onKeydown, true);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && el) hide();
  }

  function onFocus() {
    if (node.matches(':focus-visible')) scheduleShow();
  }

  node.addEventListener('mouseenter', scheduleShow);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('mousedown', hide);
  node.addEventListener('focus', onFocus);
  node.addEventListener('blur', hide);

  return {
    update(t: string | undefined) {
      current = t;
      if (el) {
        if (!t) hide();
        else el.textContent = t;
      }
    },
    destroy() {
      hide();
      node.removeEventListener('mouseenter', scheduleShow);
      node.removeEventListener('mouseleave', hide);
      node.removeEventListener('mousedown', hide);
      node.removeEventListener('focus', onFocus);
      node.removeEventListener('blur', hide);
    }
  };
}
