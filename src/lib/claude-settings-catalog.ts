// What AgentGuard knows about Claude Code's ~/.claude/settings.json: top-level
// settings and the documented environment variables (`env` block), each with a
// plain-Korean label, the right control and Claude Code's default behaviour.
// Sources: https://docs.claude.com/en/docs/claude-code/settings (+ env vars table)
// and the official schema (https://www.schemastore.org/claude-code-settings.json).
import type { Choice, FieldGroup, FieldSpec } from '$lib/setting-field';

const MODEL_CHOICES: Choice[] = [
  { v: 'opus', label: 'opus', hint: '가장 똑똑한 최신 Opus — 어려운 작업용' },
  { v: 'sonnet', label: 'sonnet', hint: '속도와 품질의 균형 — 대부분의 작업에 적합' },
  { v: 'haiku', label: 'haiku', hint: '가장 빠르고 저렴 — 간단한 작업용' },
  { v: 'opusplan', label: 'opusplan', hint: '계획은 Opus, 실제 작업은 Sonnet이 수행' },
  { v: 'sonnet[1m]', label: 'sonnet[1m]', hint: '아주 큰 코드베이스용 1M 토큰 컨텍스트' }
];

const FULL_MODEL_CHOICES: Choice[] = [
  { v: 'claude-opus-5-5', label: 'Opus 5.5' },
  { v: 'claude-sonnet-5-5', label: 'Sonnet 5.5' },
  { v: 'claude-haiku-5-5', label: 'Haiku 5.5' },
  { v: 'claude-fable-5-1', label: 'Fable 5.1' }
];

const on = (label: string): FieldSpec['caution'] => ({ when: (v) => v === true, text: label });

export const CLAUDE_GROUPS: FieldGroup[] = [
  {
    title: '모델과 응답',
    description: 'Claude가 어떤 모델로, 어떤 언어와 깊이로 답할지 정합니다.',
    fields: [
      {
        key: 'model',
        label: '모델 (/model)',
        kind: 'combo',
        choices: MODEL_CHOICES,
        defaultText: '계정 기본',
        placeholder: '다른 모델 ID 직접 입력 (예: claude-opus-5-5)',
        help: '세션 중에는 /model 로 바꿀 수 있습니다'
      },
      {
        key: 'effortLevel',
        label: 'Effort (/effort)',
        kind: 'choice',
        choices: [
          { v: 'low', label: 'low', hint: '빠른 응답, 간단한 질문용' },
          { v: 'medium', label: 'medium', hint: '대부분의 작업에 적합' },
          { v: 'high', label: 'high', hint: '복잡한 문제를 더 오래 고민합니다' },
          { v: 'xhigh', label: 'xhigh', hint: '가장 깊게 생각 — 느리고 사용량이 많습니다' }
        ],
        defaultText: '모델 기본',
        help: '응답 전에 얼마나 깊이 생각할지 정합니다'
      },
      {
        key: 'alwaysThinkingEnabled',
        label: '항상 thinking 사용',
        kind: 'toggle',
        defaultText: '끔',
        help: '모든 응답 전에 확장 사고(thinking)를 사용합니다'
      },
      {
        key: 'language',
        label: '응답 언어',
        kind: 'combo',
        choices: [
          { v: 'korean', label: '한국어' },
          { v: 'english', label: 'English' },
          { v: 'japanese', label: '日本語' },
          { v: 'chinese', label: '中文' }
        ],
        defaultText: '질문 언어를 따름',
        placeholder: '다른 언어 (예: spanish)'
      },
      {
        key: 'outputStyle',
        label: 'Output style (/output-style)',
        kind: 'combo',
        choices: [
          { v: 'Explanatory', label: 'Explanatory', hint: '작업하면서 이유와 배경을 함께 설명합니다' },
          { v: 'Learning', label: 'Learning', hint: '일부 코드를 직접 작성하도록 안내합니다' }
        ],
        defaultText: 'Default (간결한 작업 위주)',
        placeholder: '/output-style 로 만든 스타일 이름'
      }
    ]
  },
  {
    title: '편의 기능',
    description: '대화 기록·메모리·되돌리기 같은 작업 흐름을 조정합니다.',
    fields: [
      {
        key: 'autoCompactEnabled',
        label: '자동 compact',
        kind: 'toggle',
        defaultText: '켬',
        help: '대화가 길어지면 앞부분을 요약해 계속 이어갑니다 (수동: /compact)'
      },
      {
        key: 'autoMemoryEnabled',
        label: 'Auto memory (/memory)',
        kind: 'toggle',
        defaultText: '켬',
        help: '프로젝트에서 배운 내용을 다음 세션에도 기억합니다'
      },
      {
        key: 'fileCheckpointingEnabled',
        label: '파일 checkpoint (/rewind)',
        kind: 'toggle',
        defaultText: '켬',
        help: 'Claude가 바꾼 파일을 이전 시점으로 되돌릴 수 있게 저장합니다'
      },
      {
        key: 'respectGitignore',
        label: '.gitignore 파일 숨기기',
        kind: 'toggle',
        defaultText: '켬',
        help: '파일 검색·@멘션에서 .gitignore 대상(빌드 결과물 등)을 제외합니다'
      },
      {
        key: 'cleanupPeriodDays',
        label: '대화 기록 보관 기간',
        kind: 'number',
        unit: '일',
        min: 1,
        defaultText: '30일',
        help: '이 기간이 지난 세션 기록은 자동으로 삭제됩니다'
      }
    ]
  },
  {
    title: '업데이트',
    fields: [
      {
        key: 'autoUpdates',
        label: '자동 업데이트',
        kind: 'toggle',
        defaultText: '켬'
      },
      {
        key: 'autoUpdatesChannel',
        label: '업데이트 채널',
        kind: 'choice',
        choices: [
          { v: 'stable', label: 'stable', hint: '검증을 거친 버전만 받습니다' },
          { v: 'latest', label: 'latest', hint: '새 기능을 가장 먼저 받습니다' }
        ],
        defaultText: 'latest'
      }
    ]
  },
  {
    title: 'Git',
    fields: [
      {
        key: 'includeCoAuthoredBy',
        label: '커밋에 Claude 공동 작성자 표시',
        kind: 'toggle',
        defaultText: '켬',
        help: '커밋 메시지 끝에 Co-Authored-By: Claude 줄을 붙입니다'
      },
      {
        key: 'includeGitInstructions',
        label: '기본 Git 작업 지침 사용',
        kind: 'toggle',
        defaultText: '켬',
        help: '커밋·PR 작성 방법에 대한 Claude Code 기본 지침을 포함합니다'
      }
    ]
  },
  {
    title: '안전',
    description: 'Claude가 확인 없이 할 수 있는 일의 범위를 제한합니다.',
    fields: [
      {
        key: 'permissions.disableBypassPermissionsMode',
        label: 'bypassPermissions 모드 막기',
        kind: 'toggle',
        on: 'disable',
        off: null,
        help: '켜두면 --dangerously-skip-permissions 로도 확인 절차를 끌 수 없습니다 (권장)'
      },
      {
        key: 'enableAllProjectMcpServers',
        label: '프로젝트 MCP 서버 자동 승인',
        kind: 'toggle',
        defaultText: '끔 (매번 확인)',
        caution: on(
          '프로젝트 .mcp.json에 적힌 모든 서버가 확인 없이 실행됩니다 — 신뢰하는 저장소에서만 켜세요'
        )
      },
      {
        key: 'forceLoginMethod',
        label: '로그인 방식 고정',
        kind: 'choice',
        choices: [
          { v: 'claudeai', label: 'claudeai', hint: 'Pro/Max 등 Claude.ai 구독 계정으로만 로그인' },
          { v: 'console', label: 'console', hint: 'Anthropic Console(API 과금) 계정으로만 로그인' }
        ],
        defaultText: '선택 가능'
      }
    ]
  }
];

// --- environment variables -----------------------------------------------------

export interface EnvGroup extends FieldGroup {
  id: string;
}

const flag = (key: string, label: string, help: string): FieldSpec => ({
  key,
  label,
  help,
  kind: 'toggle',
  on: '1',
  off: null
});

const AWS_REGIONS: Choice[] = [
  { v: 'us-east-1', label: '미국 동부 (버지니아) · us-east-1' },
  { v: 'us-west-2', label: '미국 서부 (오레곤) · us-west-2' },
  { v: 'ap-northeast-2', label: '아시아 (서울) · ap-northeast-2' },
  { v: 'ap-northeast-1', label: '아시아 (도쿄) · ap-northeast-1' },
  { v: 'ap-southeast-1', label: '아시아 (싱가포르) · ap-southeast-1' },
  { v: 'ap-southeast-2', label: '아시아 (시드니) · ap-southeast-2' },
  { v: 'eu-central-1', label: '유럽 (프랑크푸르트) · eu-central-1' },
  { v: 'eu-west-1', label: '유럽 (아일랜드) · eu-west-1' }
];

const VERTEX_REGIONS: Choice[] = [
  { v: 'global', label: '글로벌 (자동 선택) · global' },
  { v: 'us-east5', label: '미국 (오하이오) · us-east5' },
  { v: 'europe-west1', label: '유럽 (벨기에) · europe-west1' },
  { v: 'asia-southeast1', label: '아시아 (싱가포르) · asia-southeast1' }
];

const modelEnv = (key: string, label: string, help: string): FieldSpec => ({
  key,
  label,
  help,
  kind: 'combo',
  choices: FULL_MODEL_CHOICES,
  defaultText: 'Claude Code 기본',
  placeholder: '모델 ID 직접 입력 (Bedrock/Vertex ID도 가능)'
});

export const ENV_GROUPS: EnvGroup[] = [
  {
    id: 'cloud',
    title: '클라우드 · 게이트웨이 연결',
    description:
      'Anthropic API 대신 AWS Bedrock, Google Vertex AI, Microsoft Foundry 또는 사내 게이트웨이로 연결합니다.',
    fields: [
      flag('CLAUDE_CODE_USE_BEDROCK', 'AWS Bedrock 사용', 'AWS 계정의 Bedrock을 통해 Claude를 호출합니다'),
      {
        key: 'AWS_REGION',
        label: 'AWS 리전',
        kind: 'choice',
        choices: AWS_REGIONS,
        help: 'Bedrock 사용 시 필수 — Claude 모델이 활성화된 리전을 고르세요'
      },
      {
        key: 'AWS_PROFILE',
        label: 'AWS 프로필',
        kind: 'text',
        placeholder: '~/.aws/config 의 프로필 이름 (예: default)',
        help: '키를 직접 넣는 대신 프로필을 쓰는 것이 안전합니다'
      },
      flag(
        'CLAUDE_CODE_SKIP_BEDROCK_AUTH',
        'Bedrock 인증 건너뛰기',
        '사내 LLM 게이트웨이가 AWS 인증을 대신 처리할 때만 켜세요'
      ),
      {
        key: 'ANTHROPIC_BEDROCK_BASE_URL',
        label: 'Bedrock 주소 변경',
        kind: 'text',
        placeholder: 'https://bedrock-gateway.example.com',
        help: '사내 게이트웨이나 별도 Bedrock 엔드포인트를 거칠 때 입력합니다'
      },
      flag('CLAUDE_CODE_USE_VERTEX', 'Google Vertex AI 사용', 'Google Cloud의 Vertex AI를 통해 Claude를 호출합니다'),
      {
        key: 'CLOUD_ML_REGION',
        label: 'Vertex 리전',
        kind: 'choice',
        choices: VERTEX_REGIONS,
        help: 'Vertex AI 사용 시 필수'
      },
      {
        key: 'ANTHROPIC_VERTEX_PROJECT_ID',
        label: 'GCP 프로젝트 ID',
        kind: 'text',
        placeholder: 'my-gcp-project',
        help: 'Vertex AI 사용 시 필수'
      },
      flag(
        'CLAUDE_CODE_SKIP_VERTEX_AUTH',
        'Vertex 인증 건너뛰기',
        '게이트웨이가 Google 인증을 대신 처리할 때만 켜세요'
      ),
      {
        key: 'ANTHROPIC_VERTEX_BASE_URL',
        label: 'Vertex 주소 변경',
        kind: 'text',
        placeholder: 'https://vertex-gateway.example.com',
        help: '사내 게이트웨이나 별도 Vertex 엔드포인트를 거칠 때 입력합니다'
      },
      flag('CLAUDE_CODE_USE_FOUNDRY', 'Microsoft Foundry 사용', 'Azure의 Microsoft Foundry를 통해 Claude를 호출합니다'),
      {
        key: 'ANTHROPIC_FOUNDRY_RESOURCE',
        label: 'Foundry 리소스 이름',
        kind: 'text',
        placeholder: 'my-resource',
        help: 'URL이 아니라 리소스 이름만 입력합니다'
      },
      flag(
        'CLAUDE_CODE_SKIP_FOUNDRY_AUTH',
        'Foundry 인증 건너뛰기',
        '게이트웨이가 Azure 인증을 대신 처리할 때만 켜세요'
      ),
      {
        key: 'ANTHROPIC_BASE_URL',
        label: 'API 주소 변경',
        kind: 'text',
        placeholder: 'https://llm-gateway.example.com',
        help: '사내 LLM 게이트웨이나 프록시 서버를 거쳐 호출할 때 입력합니다'
      }
    ]
  },
  {
    id: 'models',
    title: '모델 세부 지정',
    description: '별칭(opus/sonnet/haiku/fable)이 가리킬 모델이나 보조 작업용 모델을 바꿉니다.',
    fields: [
      modelEnv('ANTHROPIC_MODEL', '시작 모델 (환경변수)', '위 "사용할 모델" 설정보다 우선합니다'),
      modelEnv('ANTHROPIC_DEFAULT_OPUS_MODEL', '"opus" 가 가리킬 모델', 'Bedrock/Vertex에서 모델 ID가 다를 때 유용합니다'),
      modelEnv('ANTHROPIC_DEFAULT_SONNET_MODEL', '"sonnet" 이 가리킬 모델', 'Bedrock/Vertex에서 모델 ID가 다를 때 유용합니다'),
      modelEnv('ANTHROPIC_DEFAULT_HAIKU_MODEL', '"haiku" 가 가리킬 모델', '제목 생성 등 가벼운 백그라운드 작업에도 쓰입니다'),
      modelEnv('ANTHROPIC_DEFAULT_FABLE_MODEL', '"fable" 이 가리킬 모델', 'Bedrock/Vertex에서 모델 ID가 다를 때 유용합니다'),
      modelEnv('CLAUDE_CODE_SUBAGENT_MODEL', '서브에이전트 모델', '작업을 나눠 맡는 서브에이전트가 사용할 모델')
    ]
  },
  {
    id: 'limits',
    title: 'Thinking · 출력 한도',
    fields: [
      {
        key: 'MAX_THINKING_TOKENS',
        label: 'Thinking 토큰 예산',
        kind: 'choice',
        choices: [
          { v: '0', label: '0', hint: 'thinking을 사용하지 않습니다' },
          { v: '4000', label: '4000' },
          { v: '16000', label: '16000' },
          { v: '31999', label: '31999', hint: '복잡한 설계·디버깅에 유리하지만 느려집니다' }
        ],
        defaultText: '모델 기본',
        help: '한 번의 응답에서 생각에 쓸 수 있는 최대 토큰'
      },
      {
        key: 'CLAUDE_CODE_MAX_OUTPUT_TOKENS',
        label: '최대 응답 길이',
        kind: 'number',
        unit: '토큰',
        min: 1,
        asString: true,
        defaultText: '모델 기본',
        help: '응답이 중간에 끊기면 늘려보세요'
      },
      {
        key: 'MAX_MCP_OUTPUT_TOKENS',
        label: 'MCP 도구 응답 최대 길이',
        kind: 'number',
        unit: '토큰',
        min: 1,
        asString: true,
        defaultText: '25,000',
        help: 'MCP 서버가 큰 결과를 돌려줄 때 잘리지 않게 늘립니다'
      },
      {
        key: 'BASH_MAX_OUTPUT_LENGTH',
        label: '셸 출력 최대 길이',
        kind: 'number',
        unit: '글자',
        min: 1,
        asString: true,
        defaultText: '30,000',
        help: '이보다 긴 명령 출력은 중간이 생략됩니다'
      },
      {
        key: 'CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS',
        label: '파일 읽기 최대 길이',
        kind: 'number',
        unit: '토큰',
        min: 1,
        asString: true,
        defaultText: '25,000',
        help: '큰 파일을 한 번에 끝까지 읽어야 할 때 늘립니다'
      },
      {
        key: 'CLAUDE_CODE_MAX_TURNS',
        label: '한 요청당 최대 작업 단계',
        kind: 'number',
        unit: '회',
        min: 1,
        asString: true,
        defaultText: '제한 없음',
        help: 'Claude가 도구를 연속으로 쓰는 횟수의 상한 — 폭주를 막고 싶을 때 지정합니다'
      }
    ]
  },
  {
    id: 'timeouts',
    title: '시간 제한',
    fields: [
      {
        key: 'BASH_DEFAULT_TIMEOUT_MS',
        label: '셸 명령 기본 제한 시간',
        kind: 'number',
        unit: '분',
        scale: 60000,
        min: 0,
        asString: true,
        defaultText: '2분',
        help: '빌드·테스트가 오래 걸려 중단된다면 늘리세요'
      },
      {
        key: 'BASH_MAX_TIMEOUT_MS',
        label: '셸 명령 최대 제한 시간',
        kind: 'number',
        unit: '분',
        scale: 60000,
        min: 0,
        asString: true,
        defaultText: '10분',
        help: 'Claude가 요청할 수 있는 제한 시간의 상한'
      },
      {
        key: 'MCP_TIMEOUT',
        label: 'MCP 서버 시작 대기',
        kind: 'number',
        unit: '초',
        scale: 1000,
        min: 0,
        asString: true,
        defaultText: '30초'
      },
      {
        key: 'MCP_TOOL_TIMEOUT',
        label: 'MCP 도구 실행 제한',
        kind: 'number',
        unit: '초',
        scale: 1000,
        min: 0,
        asString: true,
        defaultText: '제한 없음'
      },
      {
        key: 'API_TIMEOUT_MS',
        label: 'API 응답 대기 시간',
        kind: 'number',
        unit: '분',
        scale: 60000,
        min: 0,
        asString: true,
        defaultText: '10분',
        help: '느린 네트워크나 프록시에서 요청이 시간 초과될 때 늘리세요'
      },
      {
        key: 'CLAUDE_CODE_MAX_RETRIES',
        label: 'API 실패 시 재시도 횟수',
        kind: 'number',
        unit: '회',
        min: 0,
        asString: true,
        defaultText: '10회'
      }
    ]
  },
  {
    id: 'network',
    title: '네트워크 · 인증서',
    description: '회사 프록시나 사내 인증서가 필요한 환경에서 설정합니다.',
    fields: [
      {
        key: 'HTTPS_PROXY',
        label: 'HTTPS 프록시',
        kind: 'text',
        placeholder: 'http://proxy.example.com:8080'
      },
      {
        key: 'HTTP_PROXY',
        label: 'HTTP 프록시',
        kind: 'text',
        placeholder: 'http://proxy.example.com:8080'
      },
      {
        key: 'NO_PROXY',
        label: '프록시 제외 주소',
        kind: 'text',
        placeholder: 'localhost,127.0.0.1,.internal.example.com',
        help: '쉼표로 구분합니다'
      },
      {
        key: 'NODE_EXTRA_CA_CERTS',
        label: '추가 루트 인증서',
        kind: 'text',
        placeholder: 'C:/certs/corp-root.pem',
        help: '사내 SSL 검사로 "인증서 오류"가 날 때 회사 인증서 파일 경로를 넣으세요'
      },
      {
        key: 'CLAUDE_CODE_CERT_STORE',
        label: '신뢰할 인증서 저장소',
        kind: 'choice',
        choices: [
          { v: 'bundled,system', label: 'bundled,system', hint: '내장 목록 + OS 인증서 저장소' },
          { v: 'system', label: 'system', hint: 'OS 저장소만 — 회사가 배포한 루트 인증서만 신뢰합니다' },
          { v: 'bundled', label: 'bundled', hint: '내장 목록만 — OS에 설치된 인증서를 무시합니다' }
        ],
        defaultText: 'bundled,system'
      },
      {
        key: 'CLAUDE_CODE_CLIENT_CERT',
        label: '클라이언트 인증서 (mTLS)',
        kind: 'text',
        placeholder: 'C:/certs/client.pem',
        help: '게이트웨이가 클라이언트 인증서를 요구할 때만 입력합니다'
      },
      {
        key: 'CLAUDE_CODE_CLIENT_KEY',
        label: '클라이언트 개인 키 (mTLS)',
        kind: 'text',
        placeholder: 'C:/certs/client-key.pem',
        help: '키 파일의 경로만 넣습니다 — 비밀번호는 설정 파일에 저장하지 마세요'
      }
    ]
  },
  {
    id: 'safety',
    title: '도구 제한 · 자격 증명 보호',
    description: 'Claude가 실행하는 명령과 MCP 서버에 비밀 값이 새지 않게 하고, 웹 도구 사용을 줄입니다.',
    fields: [
      flag(
        'CLAUDE_CODE_SUBPROCESS_ENV_SCRUB',
        '실행 명령에서 비밀 값 제거',
        '셸 명령·hooks·MCP 서버에 API 키 같은 자격 증명 환경변수를 넘기지 않습니다 (GitHub 토큰·프록시는 유지)'
      ),
      flag(
        'CLAUDE_CODE_MCP_ALLOWLIST_ENV',
        'MCP 서버에 최소 환경만 전달',
        '로컬 MCP 서버가 내 셸 환경 전체 대신 안전한 기본값과 서버별 env만 받습니다'
      ),
      flag(
        'CLAUDE_CODE_DISABLE_WEB_FETCH',
        '웹 페이지 가져오기(WebFetch) 끄기',
        'URL 내용을 읽는 도구를 끕니다. 웹 검색은 계속 사용할 수 있습니다'
      ),
      {
        key: 'CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION',
        label: '세션당 웹 검색 상한',
        kind: 'number',
        unit: '회',
        min: 1,
        asString: true,
        defaultText: '200회',
        help: '상한에 도달하면 이미 찾은 정보로 답하게 합니다 (완전히 끌 수는 없음)'
      }
    ]
  },
  {
    id: 'privacy',
    title: '개인정보 · 외부 전송',
    description: 'Anthropic 서버로 보내는 부가 정보를 줄입니다. 대화 자체에는 영향이 없습니다.',
    fields: [
      flag(
        'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC',
        '필수 외 통신 모두 끄기',
        '아래 사용 통계·오류 보고·버그 신고·자동 업데이트를 한 번에 끕니다'
      ),
      flag('DISABLE_TELEMETRY', '사용 통계 전송 끄기', '기능 사용 통계(Statsig)를 보내지 않습니다'),
      flag('DISABLE_ERROR_REPORTING', '오류 보고 끄기', '오류 발생 시 Sentry로 보고하지 않습니다'),
      flag(
        'DISABLE_FEEDBACK_COMMAND',
        '/feedback · /bug · /share 끄기',
        '대화 내용을 담은 피드백·버그 신고를 보낼 수 없게 합니다'
      ),
      flag('CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY', '만족도 설문 끄기', '"How is Claude doing?" 설문을 표시하지 않습니다'),
      flag('DISABLE_AUTOUPDATER', '자동 업데이트 막기', '회사에서 버전을 고정해야 할 때 사용합니다'),
      flag(
        'DISABLE_UPDATES',
        '모든 업데이트 차단',
        '자동 업데이트뿐 아니라 claude update 같은 수동 업데이트도 막습니다'
      ),
      flag('DISABLE_COST_WARNINGS', '비용 경고 숨기기', 'API 사용 비용 안내 메시지를 표시하지 않습니다')
    ]
  },
  {
    id: 'shell',
    title: '셸 · 도구 동작',
    fields: [
      {
        key: 'CLAUDE_CODE_GIT_BASH_PATH',
        label: 'Git Bash 위치 (Windows)',
        kind: 'combo',
        choices: [
          { v: 'C:\\Program Files\\Git\\bin\\bash.exe', label: '기본 설치 위치' },
          { v: 'C:\\Program Files (x86)\\Git\\bin\\bash.exe', label: '32비트 설치 위치' }
        ],
        defaultText: '자동 탐색',
        placeholder: 'bash.exe 전체 경로',
        help: 'Windows에서 "Git Bash를 찾을 수 없음" 오류가 날 때 지정하세요'
      },
      {
        key: 'CLAUDE_CODE_USE_POWERSHELL_TOOL',
        label: 'PowerShell 도구 사용 (Windows)',
        kind: 'toggle',
        on: '1',
        off: '0',
        defaultText: '자동',
        help: '켜면 Git Bash를 거치지 않고 PowerShell 명령을 직접 실행합니다'
      },
      flag(
        'CLAUDE_BASH_MAINTAIN_PROJECT_WORKING_DIR',
        '명령 후 프로젝트 폴더로 복귀',
        'cd 로 이동해도 매 명령마다 프로젝트 루트에서 다시 시작합니다'
      ),
      {
        key: 'USE_BUILTIN_RIPGREP',
        label: '내장 검색 도구(ripgrep) 사용',
        kind: 'toggle',
        on: '1',
        off: '0',
        defaultText: '켬',
        help: '끄면 시스템에 설치된 rg 를 사용합니다'
      },
      flag('DISABLE_PROMPT_CACHING', '프롬프트 캐싱 끄기', '디버깅용 — 켜면 비용과 응답 시간이 늘어납니다'),
      flag('CLAUDE_CODE_DISABLE_TERMINAL_TITLE', '터미널 제목 변경 끄기', '대화 내용에 따라 터미널 창 제목을 바꾸지 않습니다'),
      flag('CLAUDE_CODE_IDE_SKIP_AUTO_INSTALL', 'IDE 확장 자동 설치 끄기', 'VS Code 등에서 실행할 때 확장을 자동 설치하지 않습니다')
    ]
  }
];

/** Credential env vars: never offered in the UI, flagged when found in settings.json. */
export const SECRET_ENV: Record<string, string> = {
  ANTHROPIC_API_KEY: 'Anthropic API 키',
  ANTHROPIC_AUTH_TOKEN: 'Anthropic 인증 토큰',
  AWS_BEARER_TOKEN_BEDROCK: 'Bedrock API 키',
  AWS_ACCESS_KEY_ID: 'AWS 액세스 키',
  AWS_SECRET_ACCESS_KEY: 'AWS 비밀 키',
  AWS_SESSION_TOKEN: 'AWS 세션 토큰',
  ANTHROPIC_AWS_API_KEY: 'Claude Platform on AWS API 키',
  ANTHROPIC_FOUNDRY_API_KEY: 'Microsoft Foundry API 키',
  ANTHROPIC_FOUNDRY_AUTH_TOKEN: 'Microsoft Foundry 인증 토큰',
  CLAUDE_CODE_OAUTH_TOKEN: 'Claude 로그인 토큰',
  CLAUDE_CODE_OAUTH_REFRESH_TOKEN: 'Claude 로그인 갱신 토큰',
  CLAUDE_CODE_CLIENT_KEY_PASSPHRASE: 'mTLS 개인 키 비밀번호',
  MCP_CLIENT_SECRET: 'MCP OAuth 클라이언트 비밀'
};

export const KNOWN_ENV = new Set(ENV_GROUPS.flatMap((g) => g.fields.map((f) => f.key)));

export interface EnvPreset {
  id: string;
  label: string;
  description: string;
  /** Values to set; a function receives OS env values (for AWS_REGION etc.). */
  values: (os: Record<string, string>) => Record<string, string>;
}

export const ENV_PRESETS: EnvPreset[] = [
  {
    id: 'bedrock',
    label: 'AWS Bedrock로 연결',
    description: 'Bedrock 사용을 켜고 리전·프로필을 시스템 값(없으면 us-east-1)으로 채웁니다',
    values: (os) => ({
      CLAUDE_CODE_USE_BEDROCK: '1',
      AWS_REGION: os.AWS_REGION || os.AWS_DEFAULT_REGION || 'us-east-1',
      ...(os.AWS_PROFILE ? { AWS_PROFILE: os.AWS_PROFILE } : {})
    })
  },
  {
    id: 'vertex',
    label: 'Google Vertex AI로 연결',
    description: 'Vertex 사용을 켜고 리전을 global 로 설정합니다 (프로젝트 ID는 직접 입력)',
    values: () => ({ CLAUDE_CODE_USE_VERTEX: '1', CLOUD_ML_REGION: 'global' })
  },
  {
    id: 'private',
    label: '외부 전송 최소화',
    description: '사용 통계·오류 보고·버그 신고·자동 업데이트 통신을 모두 끕니다',
    values: () => ({ CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: '1' })
  },
  {
    id: 'scrub-secrets',
    label: '비밀 값 유출 막기',
    description: '셸 명령·hooks·MCP 서버에 자격 증명 환경변수가 전달되지 않게 합니다',
    values: () => ({ CLAUDE_CODE_SUBPROCESS_ENV_SCRUB: '1', CLAUDE_CODE_MCP_ALLOWLIST_ENV: '1' })
  },
  {
    id: 'long-builds',
    label: '오래 걸리는 빌드 허용',
    description: '셸 명령 제한 시간을 기본 10분, 최대 30분으로 늘립니다',
    values: () => ({ BASH_DEFAULT_TIMEOUT_MS: '600000', BASH_MAX_TIMEOUT_MS: '1800000' })
  }
];
