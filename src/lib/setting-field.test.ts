import { describe, expect, it } from 'vitest';
import { sameValue, toggleState, type FieldSpec } from './setting-field';
import { CLAUDE_GROUPS, ENV_GROUPS, ENV_PRESETS, KNOWN_ENV, SECRET_ENV } from './claude-settings-catalog';

describe('toggleState', () => {
  const tri: FieldSpec = { key: 'autoUpdates', label: '', kind: 'toggle' };
  const flag: FieldSpec = { key: 'DISABLE_TELEMETRY', label: '', kind: 'toggle', on: '1', off: null };

  it('three-state toggles distinguish default/on/off', () => {
    expect(toggleState(tri, undefined)).toBe('default');
    expect(toggleState(tri, true)).toBe('on');
    expect(toggleState(tri, false)).toBe('off');
  });

  it('env flags accept 1/true as on and anything else as off', () => {
    expect(toggleState(flag, undefined)).toBe('default');
    expect(toggleState(flag, '1')).toBe('on');
    expect(toggleState(flag, 'true')).toBe('on');
    expect(toggleState(flag, '0')).toBe('off');
  });

  it('sameValue matches env strings against numeric presets', () => {
    expect(sameValue('31999', 31999)).toBe(true);
    expect(sameValue(undefined, null)).toBe(false);
  });
});

describe('claude settings catalog', () => {
  const envKeys = ENV_GROUPS.flatMap((g) => g.fields.map((f) => f.key));

  it('env names are unique, valid and never secrets', () => {
    expect(new Set(envKeys).size).toBe(envKeys.length);
    for (const k of envKeys) {
      expect(k).toMatch(/^[A-Z_][A-Z0-9_]*$/);
      expect(k in SECRET_ENV).toBe(false);
    }
  });

  it('setting keys are unique', () => {
    const keys = CLAUDE_GROUPS.flatMap((g) => g.fields.map((f) => f.key));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('presets only set catalogued env vars', () => {
    for (const p of ENV_PRESETS) {
      for (const k of Object.keys(p.values({ AWS_PROFILE: 'dev' }))) expect(KNOWN_ENV.has(k)).toBe(true);
    }
  });

  it('bedrock preset reuses the system region and profile', () => {
    const bedrock = ENV_PRESETS.find((p) => p.id === 'bedrock')!;
    expect(bedrock.values({ AWS_REGION: 'ap-northeast-2', AWS_PROFILE: 'dev' })).toEqual({
      CLAUDE_CODE_USE_BEDROCK: '1',
      AWS_REGION: 'ap-northeast-2',
      AWS_PROFILE: 'dev'
    });
  });
});
