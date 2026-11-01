import { describe, expect, it } from 'vitest';
import { MCP_CATALOG, applyInputs, windowsWrap } from './mcp-catalog';

const TOKEN_LIKE = /^(Bearer\s+)?(gh[pousr]_|github_pat_|sk-|xox[abp]-)?[A-Za-z0-9_\-]{20,}$/;

describe('MCP_CATALOG', () => {
  it('has unique ids and valid server names', () => {
    const ids = MCP_CATALOG.map((e) => e.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const e of MCP_CATALOG) expect(e.name).toMatch(/^[A-Za-z0-9_.-]+$/);
  });

  it('contains no literal secrets in headers or env', () => {
    for (const e of MCP_CATALOG) {
      for (const field of ['headers', 'env'] as const) {
        const values = Object.values((e.config[field] as Record<string, string> | undefined) ?? {});
        for (const v of values) {
          expect(v.includes('${') || !TOKEN_LIKE.test(v), `${e.id}.${field}`).toBe(true);
        }
      }
    }
  });

  it('every entry is either stdio with a command or remote with a url', () => {
    for (const e of MCP_CATALOG) {
      expect(typeof e.config.command === 'string' || typeof e.config.url === 'string').toBe(true);
    }
  });
});

describe('applyInputs', () => {
  it('appends arg inputs without mutating the catalog', () => {
    const fs = MCP_CATALOG.find((e) => e.id === 'filesystem')!;
    const before = JSON.stringify(fs.config);
    const out = applyInputs(fs, { folder: ' C:/work ' });
    expect((out.args as string[]).at(-1)).toBe('C:/work');
    expect(JSON.stringify(fs.config)).toBe(before);
  });
});

describe('windowsWrap', () => {
  it('wraps npx through cmd /c', () => {
    const out = windowsWrap({ type: 'stdio', command: 'npx', args: ['-y', 'pkg'] });
    expect(out).toEqual({ type: 'stdio', command: 'cmd', args: ['/c', 'npx', '-y', 'pkg'] });
  });

  it('is idempotent', () => {
    const once = windowsWrap({ command: 'npx', args: ['pkg'] });
    expect(windowsWrap(once)).toEqual(once);
  });

  it('leaves uvx and remote servers untouched', () => {
    const uv = { command: 'uvx', args: ['mcp-server-fetch'] };
    const remote = { type: 'http', url: 'https://example.com/mcp' };
    expect(windowsWrap(uv)).toBe(uv);
    expect(windowsWrap(remote)).toBe(remote);
  });
});
