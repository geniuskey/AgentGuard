// Bundled offline MCP server catalog. Secrets appear only as ${VAR} references
// so nothing sensitive is ever written into a config file.

export interface McpCatalogInput {
  key: string;
  label: string;
  placeholder?: string;
  target: 'arg' | 'env' | 'header';
}

export interface McpCatalogEntry {
  id: string;
  name: string;
  title: string;
  description: string;
  usesWeb: boolean;
  config: Record<string, unknown>;
  inputs?: McpCatalogInput[];
  note?: string;
}

const npx = (...args: string[]) => ({ type: 'stdio', command: 'npx', args });
const uvx = (...args: string[]) => ({ type: 'stdio', command: 'uvx', args });
const http = (url: string, headers?: Record<string, string>) =>
  headers ? { type: 'http', url, headers } : { type: 'http', url };

const UV_NOTE = 'uv(uvx)가 설치되어 있어야 합니다 (https://docs.astral.sh/uv/).';
const LOGIN_NOTE = '처음 사용할 때 Claude Code에서 /mcp 로 로그인(OAuth)합니다.';

export const MCP_CATALOG: McpCatalogEntry[] = [
  {
    id: 'context7',
    name: 'context7',
    title: 'Context7 (최신 라이브러리 문서)',
    description: '라이브러리·프레임워크의 최신 문서와 코드 예시를 찾아 Claude에게 전달합니다.',
    usesWeb: true,
    config: npx('-y', '@upstash/context7-mcp')
  },
  {
    id: 'playwright',
    name: 'playwright',
    title: 'Playwright (브라우저 자동화)',
    description: '브라우저를 열어 웹페이지를 탐색·클릭·입력하고 화면을 확인합니다.',
    usesWeb: true,
    config: npx('@playwright/mcp@latest')
  },
  {
    id: 'chrome-devtools',
    name: 'chrome-devtools',
    title: 'Chrome DevTools (크롬 디버깅)',
    description: '크롬을 제어하며 콘솔·네트워크·성능 정보를 읽어 웹 문제를 진단합니다.',
    usesWeb: true,
    config: npx('chrome-devtools-mcp@latest')
  },
  {
    id: 'filesystem',
    name: 'filesystem',
    title: '파일 시스템 (지정 폴더)',
    description: '지정한 폴더 안의 파일을 읽고 쓸 수 있게 합니다. 꼭 필요한 폴더만 지정하세요.',
    usesWeb: false,
    config: npx('-y', '@modelcontextprotocol/server-filesystem'),
    inputs: [{ key: 'folder', label: '허용할 폴더', placeholder: 'C:/Users/me/Documents', target: 'arg' }]
  },
  {
    id: 'memory',
    name: 'memory',
    title: '메모리 (지식 그래프)',
    description: '대화 사이에 기억할 내용을 로컬 지식 그래프에 저장합니다.',
    usesWeb: false,
    config: npx('-y', '@modelcontextprotocol/server-memory')
  },
  {
    id: 'sequential-thinking',
    name: 'sequential-thinking',
    title: '단계별 사고',
    description: '복잡한 문제를 단계로 나누어 차근차근 추론하도록 돕습니다.',
    usesWeb: false,
    config: npx('-y', '@modelcontextprotocol/server-sequential-thinking')
  },
  {
    id: 'fetch',
    name: 'fetch',
    title: 'Fetch (웹페이지 가져오기)',
    description: '웹페이지를 내려받아 읽기 쉬운 텍스트로 변환합니다.',
    usesWeb: true,
    config: uvx('mcp-server-fetch'),
    note: UV_NOTE
  },
  {
    id: 'git',
    name: 'git',
    title: 'Git (저장소 조회·조작)',
    description: '로컬 Git 저장소의 상태·기록·차이를 조회하고 커밋 등을 수행합니다.',
    usesWeb: false,
    config: uvx('mcp-server-git'),
    note: UV_NOTE
  },
  {
    id: 'github',
    name: 'github',
    title: 'GitHub (원격)',
    description: 'GitHub 이슈·PR·저장소를 조회하고 다룹니다. GitHub 공식 원격 서버입니다.',
    usesWeb: true,
    config: http('https://api.githubcopilot.com/mcp/', {
      Authorization: 'Bearer ${GITHUB_PERSONAL_ACCESS_TOKEN}'
    }),
    note:
      'Windows 환경변수 GITHUB_PERSONAL_ACCESS_TOKEN 에 토큰을 설정하세요. 토큰을 설정 파일에 직접 붙여넣지 마세요.'
  },
  {
    id: 'sentry',
    name: 'sentry',
    title: 'Sentry (오류 모니터링)',
    description: 'Sentry의 오류·이슈 정보를 조회합니다.',
    usesWeb: true,
    config: http('https://mcp.sentry.dev/mcp'),
    note: LOGIN_NOTE
  },
  {
    id: 'linear',
    name: 'linear',
    title: 'Linear (이슈 관리)',
    description: 'Linear 이슈와 프로젝트를 조회하고 만듭니다.',
    usesWeb: true,
    config: http('https://mcp.linear.app/mcp'),
    note: LOGIN_NOTE
  },
  {
    id: 'notion',
    name: 'notion',
    title: 'Notion (문서)',
    description: 'Notion 페이지와 데이터베이스를 검색하고 편집합니다.',
    usesWeb: true,
    config: http('https://mcp.notion.com/mcp'),
    note: LOGIN_NOTE
  }
];

/** Fill a catalog entry's inputs into a fresh copy of its config. */
export function applyInputs(
  entry: McpCatalogEntry,
  values: Record<string, string>
): Record<string, unknown> {
  const config = structuredClone(entry.config);
  for (const input of entry.inputs ?? []) {
    const v = values[input.key]?.trim();
    if (!v) continue;
    if (input.target === 'arg') {
      config.args = [...((config.args as string[] | undefined) ?? []), v];
    } else {
      const field = input.target === 'env' ? 'env' : 'headers';
      config[field] = { ...((config[field] as Record<string, string> | undefined) ?? {}), [input.key]: v };
    }
  }
  return config;
}

/** On Windows, `npx` is a .cmd shim that stdio servers must launch through `cmd /c`. */
export function windowsWrap(config: Record<string, unknown>): Record<string, unknown> {
  if (config.command !== 'npx') return config;
  const args = Array.isArray(config.args) ? (config.args as unknown[]) : [];
  return { ...config, command: 'cmd', args: ['/c', 'npx', ...args] };
}
