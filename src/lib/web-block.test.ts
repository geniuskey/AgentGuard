import { describe, expect, it } from 'vitest';
import { toggleWebBlockRules, webBlockState } from './web-block';

const WEB_RULES = [
  'WebSearch',
  'WebFetch',
  'PowerShell(Invoke-WebRequest *)'
];
const CUSTOM_DENY = 'PowerShell(Remove-Item *)';

describe('webBlockState', () => {
  it('reports off when no web rule is configured', () => {
    expect(webBlockState([], WEB_RULES)).toBe('off');
    expect(webBlockState([CUSTOM_DENY], WEB_RULES)).toBe('off');
  });

  it('reports partial only for a subset of web rules', () => {
    expect(webBlockState([CUSTOM_DENY, WEB_RULES[0]], WEB_RULES)).toBe('partial');
  });

  it('reports on when every web rule exists alongside custom denies', () => {
    expect(webBlockState([CUSTOM_DENY, ...WEB_RULES], WEB_RULES)).toBe('on');
  });

  it('treats an empty web rule set as off', () => {
    expect(webBlockState([CUSTOM_DENY], [])).toBe('off');
  });
});

describe('toggleWebBlockRules', () => {
  it('completes a partial block without duplicating rules or losing custom denies', () => {
    const current = [CUSTOM_DENY, WEB_RULES[0]];

    expect(toggleWebBlockRules(current, WEB_RULES)).toEqual([
      CUSTOM_DENY,
      WEB_RULES[0],
      WEB_RULES[1],
      WEB_RULES[2]
    ]);
    expect(current).toEqual([CUSTOM_DENY, WEB_RULES[0]]);
  });

  it('removes only web rules when a full block is disabled', () => {
    expect(toggleWebBlockRules([WEB_RULES[0], CUSTOM_DENY, ...WEB_RULES.slice(1)], WEB_RULES)).toEqual([
      CUSTOM_DENY
    ]);
  });

  it('leaves rules unchanged when the web rule set is unavailable', () => {
    expect(toggleWebBlockRules([CUSTOM_DENY], [])).toEqual([CUSTOM_DENY]);
  });
});
