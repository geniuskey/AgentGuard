// Two-letter monogram + stable hue so projects stay distinguishable in a collapsed sidebar.

const HUES = [255, 155, 25, 300, 195, 85, 340, 130, 225, 50];

function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function projectInitials(name: string): string {
  const words = name
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .split(/[\s\-_.@/\\]+/)
    .filter(Boolean);
  if (words.length === 0) return '?';
  const chars = (w: string) => [...w];
  if (!/^[a-z0-9]/i.test(words[0])) return chars(words[0])[0];
  if (words.length === 1) {
    const c = chars(words[0]);
    return (c[0].toUpperCase() + (c[1] ?? '').toLowerCase()).trim();
  }
  return (chars(words[0])[0] + chars(words[1])[0]).toUpperCase();
}

export function projectMark(name: string, key: string): { text: string; hue: number } {
  return { text: projectInitials(name), hue: HUES[hash(key.toLowerCase()) % HUES.length] };
}
