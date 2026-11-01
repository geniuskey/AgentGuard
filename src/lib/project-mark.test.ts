import { describe, expect, it } from 'vitest';
import { projectInitials, projectMark } from './project-mark';

describe('projectInitials', () => {
  it('uses first letters of the first two words', () => {
    expect(projectInitials('AgentGuard')).toBe('AG');
    expect(projectInitials('my-web-app')).toBe('MW');
    expect(projectInitials('foo_bar')).toBe('FB');
  });

  it('uses the first two letters of a single word', () => {
    expect(projectInitials('web')).toBe('We');
    expect(projectInitials('x')).toBe('X');
  });

  it('handles non-latin names', () => {
    expect(projectInitials('프로젝트')).toBe('프');
  });
});

describe('projectMark', () => {
  it('is stable for the same key and ignores case', () => {
    expect(projectMark('a', 'D:\\Git\\App').hue).toBe(projectMark('a', 'd:\\git\\app').hue);
  });
});
