// Svelte action: `use:tooltip={'설명'}`. Renders a fixed-position tooltip on
// hover — immune to overflow clipping inside scrollable panels, flips above
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
    timer = window.setTimeout(show, 300);
  }

  function hide() {
    window.clearTimeout(timer);
    el?.remove();
    el = null;
  }

  node.addEventListener('mouseenter', scheduleShow);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('mousedown', hide);

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
    }
  };
}
