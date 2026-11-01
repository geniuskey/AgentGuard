<script lang="ts">
  // Structured settings form over a raw agent config (opencode.json / config.toml).
  // Reads the parsed tree, writes one dotted-path key per change via the backend
  // (all other keys preserved), and hands the new text back to the raw editor.
  // Field registries follow the official docs:
  //   OpenCode: https://opencode.ai/docs/config, /docs/permissions
  //   Codex:    https://developers.openai.com/codex/config-reference
  import { configGet, configSetValue, inTauri } from '$lib/ipc';
  import { getAt, type Choice, type FieldGroup, type FieldSpec } from '$lib/setting-field';
  import SettingControl from '$lib/components/SettingControl.svelte';

  let {
    agentId,
    format,
    text,
    onchange
  }: {
    agentId: string;
    format: string;
    text: string;
    onchange: (t: string) => void;
  } = $props();

  const bool = (key: string, label: string, help: string, defaultText: string): FieldSpec => ({
    key,
    label,
    help,
    kind: 'toggle',
    on: true,
    off: false,
    defaultText
  });

  const permChoices = (riskyAllow: boolean): Choice[] => [
    { v: 'ask', label: '물어보기', hint: '실행할 때마다 확인을 받습니다.' },
    {
      v: 'allow',
      label: '자동 허용',
      hint: riskyAllow ? '확인 없이 바로 실행합니다. 되돌리기 어려운 작업도 포함됩니다.' : '확인 없이 바로 실행합니다.',
      risky: riskyAllow
    },
    { v: 'deny', label: '차단', hint: '이 도구를 아예 쓰지 못하게 합니다.' }
  ];

  const perm = (
    tool: string,
    label: string,
    help: string,
    defaultText: string,
    riskyAllow = false
  ): FieldSpec => ({
    key: `permission.${tool}`,
    label,
    help,
    kind: 'choice',
    choices: permChoices(riskyAllow),
    defaultText
  });

  const codex: FieldGroup[] = [
    {
      title: '모델',
      description: '어떤 모델이 얼마나 깊이 생각하고 어떻게 답할지 정합니다.',
      fields: [
        {
          key: 'model',
          label: '사용할 모델',
          help: '목록에 없으면 모델 ID를 직접 입력하세요.',
          kind: 'combo',
          defaultText: 'Codex 권장 모델',
          placeholder: '모델 ID 직접 입력',
          choices: [
            { v: 'gpt-5.5', label: 'GPT-5.5' },
            { v: 'gpt-5.4', label: 'GPT-5.4' },
            { v: 'gpt-5.3-codex', label: 'GPT-5.3 Codex' },
            { v: 'gpt-5.2-codex', label: 'GPT-5.2 Codex' },
            { v: 'gpt-5.1-codex-max', label: 'GPT-5.1 Codex Max' },
            { v: 'gpt-5.1-codex-mini', label: 'GPT-5.1 Codex Mini' }
          ]
        },
        {
          key: 'model_reasoning_effort',
          label: '생각하는 깊이',
          help: '깊을수록 어려운 문제에 강하지만 느리고 비용이 늘어납니다.',
          kind: 'choice',
          defaultText: 'medium',
          choices: [
            { v: 'minimal', label: '최소', hint: '거의 생각하지 않고 바로 답합니다. 가장 빠릅니다.' },
            { v: 'low', label: '낮음', hint: '간단한 작업에 적합합니다.' },
            { v: 'medium', label: '보통', hint: '속도와 품질의 균형입니다.' },
            { v: 'high', label: '높음', hint: '복잡한 문제를 더 꼼꼼히 풉니다.' },
            { v: 'xhigh', label: '최고', hint: '가장 깊이 생각합니다. 지원 모델에서만 동작합니다.' }
          ]
        },
        {
          key: 'model_reasoning_summary',
          label: '생각 요약 보기',
          help: '모델이 어떻게 생각했는지 요약해서 보여줄지 정합니다.',
          kind: 'choice',
          defaultText: 'auto',
          choices: [
            { v: 'auto', label: '자동', hint: '모델이 알아서 적당히 요약합니다.' },
            { v: 'concise', label: '간단히', hint: '짧게 요약합니다.' },
            { v: 'detailed', label: '자세히', hint: '길고 자세하게 요약합니다.' },
            { v: 'none', label: '안 보기', hint: '요약을 만들지 않습니다.' }
          ]
        },
        {
          key: 'model_verbosity',
          label: '답변 길이',
          help: '설명을 얼마나 자세히 쓸지 정합니다.',
          kind: 'choice',
          defaultText: 'medium',
          choices: [
            { v: 'low', label: '짧게', hint: '핵심만 간결하게 답합니다.' },
            { v: 'medium', label: '보통' },
            { v: 'high', label: '자세히', hint: '배경과 이유까지 길게 설명합니다.' }
          ]
        },
        {
          key: 'personality',
          label: '말투',
          help: '대화할 때의 어조를 정합니다.',
          kind: 'choice',
          defaultText: '모델 기본',
          choices: [
            { v: 'friendly', label: '친근하게' },
            { v: 'pragmatic', label: '실용적으로', hint: '군더더기 없이 일 중심으로 말합니다.' },
            { v: 'none', label: '꾸밈 없음', hint: '특별한 말투를 입히지 않습니다.' }
          ]
        },
        {
          key: 'model_provider',
          label: '모델 제공자',
          help: '다른 제공자는 model_providers 표에 먼저 정의해야 합니다.',
          kind: 'combo',
          defaultText: 'openai',
          placeholder: '제공자 ID 직접 입력',
          choices: [
            { v: 'openai', label: 'OpenAI' },
            { v: 'oss', label: '로컬 오픈소스 (oss)' }
          ]
        }
      ]
    },
    {
      title: '안전 · 권한',
      description: '명령을 실행하기 전에 확인할지, 어디까지 손댈 수 있는지 정합니다.',
      fields: [
        {
          key: 'approval_policy',
          label: '명령 실행 전 확인',
          help: '모델이 명령을 실행하려 할 때 언제 나에게 물어볼지 정합니다.',
          kind: 'choice',
          defaultText: 'on-request',
          choices: [
            { v: 'untrusted', label: '대부분 확인', hint: '안전한 읽기 명령 외에는 모두 확인을 받습니다.' },
            { v: 'on-request', label: '필요할 때만', hint: '샌드박스 밖 작업처럼 모델이 필요하다고 판단할 때만 묻습니다.' },
            { v: 'never', label: '묻지 않음', hint: '확인 없이 실행합니다. 실패해도 묻지 않습니다.', risky: true }
          ]
        },
        {
          key: 'sandbox_mode',
          label: '파일 접근 범위',
          help: '실행하는 명령이 어디까지 읽고 쓸 수 있는지 제한합니다.',
          kind: 'choice',
          defaultText: 'read-only',
          choices: [
            { v: 'read-only', label: '읽기만', hint: '파일을 읽을 수만 있고 수정은 할 수 없습니다.' },
            { v: 'workspace-write', label: '작업 폴더만 쓰기', hint: '현재 프로젝트 폴더 안에서만 파일을 수정합니다.' },
            {
              v: 'danger-full-access',
              label: '제한 없음',
              hint: '샌드박스를 끕니다. 컴퓨터 전체와 네트워크에 접근할 수 있습니다.',
              risky: true
            }
          ]
        },
        {
          ...bool(
            'sandbox_workspace_write.network_access',
            '샌드박스 안 인터넷 허용',
            "'작업 폴더만 쓰기'일 때 명령이 인터넷에 접속할 수 있게 합니다.",
            '끔'
          ),
          caution: {
            when: (v) => v === true,
            text: '패키지 설치 등은 편해지지만, 실행한 명령이 외부로 데이터를 보낼 수 있습니다.'
          }
        },
        {
          key: 'web_search',
          label: '웹 검색',
          help: '모델이 답을 찾을 때 웹 검색을 쓸 수 있는지 정합니다.',
          kind: 'choice',
          defaultText: 'cached',
          choices: [
            { v: 'cached', label: '캐시 결과만', hint: '미리 수집된 검색 결과만 사용합니다.' },
            { v: 'live', label: '실시간 검색', hint: '실제 웹을 바로 검색합니다. 최신 정보를 얻을 수 있습니다.' },
            { v: 'disabled', label: '사용 안 함', hint: '웹 검색 도구를 끕니다.' }
          ]
        },
        {
          key: 'shell_environment_policy.inherit',
          label: '명령에 넘길 환경 변수',
          help: '실행하는 명령이 내 환경 변수(토큰, 경로 등)를 얼마나 물려받을지 정합니다.',
          kind: 'choice',
          defaultText: 'all',
          choices: [
            { v: 'all', label: '전부', hint: '내 셸의 환경 변수를 모두 넘깁니다. 비밀 값도 포함될 수 있습니다.' },
            { v: 'core', label: '필수만', hint: 'PATH, HOME 같은 기본 변수만 넘깁니다.' },
            { v: 'none', label: '없음', hint: '환경 변수를 넘기지 않습니다. 일부 명령이 동작하지 않을 수 있습니다.' }
          ]
        }
      ]
    },
    {
      title: '화면 · 동작',
      description: '작업 중에 보이는 정보와 편의 기능을 정합니다.',
      fields: [
        {
          key: 'file_opener',
          label: '파일 링크로 열 에디터',
          help: '답변 속 파일 경로를 클릭하면 어떤 에디터로 열지 정합니다.',
          kind: 'choice',
          defaultText: 'vscode',
          choices: [
            { v: 'vscode', label: 'VS Code' },
            { v: 'vscode-insiders', label: 'VS Code Insiders' },
            { v: 'cursor', label: 'Cursor' },
            { v: 'windsurf', label: 'Windsurf' },
            { v: 'none', label: '링크 안 만듦', hint: '파일 경로를 일반 글자로 보여줍니다.' }
          ]
        },
        bool('hide_agent_reasoning', '생각 과정 숨기기', '모델이 생각하는 중간 과정을 화면에 표시하지 않습니다.', '끔'),
        bool(
          'show_raw_agent_reasoning',
          '생각 원문 보기',
          '요약 대신 모델이 내놓은 생각 원문을 보여줍니다. 지원 모델에서만 동작합니다.',
          '끔'
        ),
        bool('check_for_update_on_startup', '시작할 때 업데이트 확인', '새 버전이 있으면 실행 시 알려줍니다.', '켬')
      ]
    },
    {
      title: '기록 · 고급',
      description: '대화 기록과 컨텍스트 관리 방식을 정합니다.',
      fields: [
        {
          key: 'history.persistence',
          label: '대화 기록 저장',
          help: '입력한 내용을 ~/.codex/history.jsonl에 남길지 정합니다.',
          kind: 'choice',
          defaultText: 'save-all',
          choices: [
            { v: 'save-all', label: '저장', hint: '이전 입력을 다시 불러올 수 있습니다.' },
            { v: 'none', label: '저장 안 함', hint: '기록 파일에 아무것도 남기지 않습니다.' }
          ]
        },
        {
          key: 'project_doc_max_bytes',
          label: 'AGENTS.md 최대 크기',
          help: '프로젝트 안내 문서를 이 크기까지만 읽어 모델에 전달합니다.',
          kind: 'number',
          unit: 'KB',
          scale: 1024,
          min: 0,
          defaultText: '32 KB'
        },
        {
          key: 'model_auto_compact_token_limit',
          label: '자동 요약 기준',
          help: '대화가 이 토큰 수를 넘으면 앞부분을 자동으로 요약해 공간을 확보합니다.',
          kind: 'number',
          unit: '토큰',
          min: 0,
          defaultText: '모델별 기본값'
        }
      ]
    }
  ];

  const opencode: FieldGroup[] = [
    {
      title: '모델',
      description: '대화와 보조 작업에 쓸 모델을 정합니다. "제공자/모델" 형식입니다.',
      fields: [
        {
          key: 'model',
          label: '사용할 모델',
          help: '목록에 없으면 "제공자/모델 ID"를 직접 입력하세요.',
          kind: 'combo',
          defaultText: '마지막 사용 모델',
          placeholder: 'provider/model',
          choices: [
            { v: 'anthropic/claude-sonnet-5-5', label: 'Claude Sonnet 5.5' },
            { v: 'anthropic/claude-opus-5-5', label: 'Claude Opus 5.5' },
            { v: 'anthropic/claude-haiku-5-5', label: 'Claude Haiku 5.5' },
            { v: 'openai/gpt-5.5', label: 'GPT-5.5' },
            { v: 'openai/gpt-5.1-codex', label: 'GPT-5.1 Codex' },
            { v: 'google/gemini-2.5-pro', label: 'Gemini 2.5 Pro' }
          ]
        },
        {
          key: 'small_model',
          label: '가벼운 작업용 모델',
          help: '대화 제목 생성처럼 간단한 일에 쓰는 저렴한 모델입니다.',
          kind: 'combo',
          defaultText: '제공자의 소형 모델',
          placeholder: 'provider/model',
          choices: [
            { v: 'anthropic/claude-haiku-5-5', label: 'Claude Haiku 5.5' },
            { v: 'openai/gpt-5-mini', label: 'GPT-5 mini' },
            { v: 'google/gemini-2.5-flash', label: 'Gemini 2.5 Flash' }
          ]
        }
      ]
    },
    {
      title: '안전 · 권한',
      description: '도구를 쓰기 전에 물어볼지, 바로 실행할지, 막을지 정합니다.',
      fields: [
        perm('edit', '파일 수정', '파일을 만들거나 고치는 작업입니다.', 'allow'),
        perm('bash', '터미널 명령 실행', '셸 명령을 실행합니다. 파일 삭제나 설치도 가능합니다.', 'allow', true),
        perm('read', '파일 읽기', '프로젝트 파일 내용을 읽습니다.', 'allow'),
        perm(
          'external_directory',
          '프로젝트 밖 폴더 접근',
          '현재 작업 폴더 바깥의 파일을 건드릴 때입니다.',
          'ask',
          true
        ),
        perm('webfetch', '웹 페이지 가져오기', '주소(URL)를 열어 내용을 읽습니다.', 'allow'),
        perm('websearch', '웹 검색', '검색 엔진으로 정보를 찾습니다.', 'allow'),
        perm('doom_loop', '같은 작업 반복 감지', '같은 도구를 같은 입력으로 계속 호출할 때입니다.', 'ask')
      ]
    },
    {
      title: '화면 · 동작',
      description: '보이는 모습과 공유, 업데이트 방식을 정합니다.',
      fields: [
        {
          key: 'theme',
          label: '테마',
          help: '터미널 화면의 색 구성을 정합니다.',
          kind: 'combo',
          defaultText: 'opencode',
          placeholder: '테마 이름 직접 입력',
          choices: [
            { v: 'opencode', label: 'opencode' },
            { v: 'system', label: '터미널 색 따르기' },
            { v: 'tokyonight', label: 'Tokyo Night' },
            { v: 'catppuccin', label: 'Catppuccin' },
            { v: 'gruvbox', label: 'Gruvbox' },
            { v: 'nord', label: 'Nord' },
            { v: 'everforest', label: 'Everforest' },
            { v: 'kanagawa', label: 'Kanagawa' },
            { v: 'ayu', label: 'Ayu' },
            { v: 'one-dark', label: 'One Dark' },
            { v: 'matrix', label: 'Matrix' }
          ]
        },
        {
          key: 'share',
          label: '대화 공유',
          help: '공유하면 대화 내용이 opencode 서버에 올라가 링크로 볼 수 있게 됩니다.',
          kind: 'choice',
          defaultText: 'manual',
          choices: [
            { v: 'manual', label: '직접 할 때만', hint: '/share 명령을 쓸 때만 공유합니다.' },
            { v: 'auto', label: '자동 공유', hint: '새 대화를 모두 자동으로 업로드합니다.', risky: true },
            { v: 'disabled', label: '공유 금지', hint: '공유 기능을 완전히 끕니다.' }
          ]
        },
        {
          key: 'autoupdate',
          label: '자동 업데이트',
          help: '새 버전이 나왔을 때 어떻게 할지 정합니다.',
          kind: 'choice',
          defaultText: '자동 설치',
          choices: [
            { v: true, label: '자동 설치', hint: '시작할 때 새 버전을 내려받아 설치합니다.' },
            { v: 'notify', label: '알림만', hint: '새 버전이 있다고 알려주기만 합니다.' },
            { v: false, label: '끔', hint: '업데이트를 확인하지 않습니다.' }
          ]
        },
        bool(
          'snapshot',
          '변경 되돌리기 지점 저장',
          '작업마다 파일 상태를 기록해 나중에 되돌릴 수 있게 합니다. 큰 저장소에서는 느려질 수 있습니다.',
          '켬'
        ),
        {
          key: 'username',
          label: '대화에 표시할 이름',
          help: '대화 화면에 내 이름 대신 보여줄 이름입니다.',
          kind: 'text',
          defaultText: '시스템 사용자 이름'
        }
      ]
    }
  ];

  const registries: Record<string, FieldGroup[]> = { codex, opencode };
  const sections = $derived(registries[agentId] ?? []);

  let tree = $state<Record<string, unknown> | null>(null);
  let parseError = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    const current = text;
    const fmt = format;
    if (!inTauri()) return;
    const t = setTimeout(async () => {
      try {
        tree = await configGet(current, fmt);
        parseError = false;
      } catch {
        parseError = true;
      }
    }, 300);
    return () => clearTimeout(t);
  });

  const isPermMap = (f: FieldSpec) => agentId === 'opencode' && f.key.startsWith('permission.');

  function patternMap(f: FieldSpec): Record<string, unknown> | null {
    if (!isPermMap(f)) return null;
    const raw = getAt(tree, f.key);
    return raw !== null && typeof raw === 'object' && !Array.isArray(raw)
      ? (raw as Record<string, unknown>)
      : null;
  }

  function fieldValue(f: FieldSpec): { value: unknown; patterns: number } {
    const m = patternMap(f);
    if (m) return { value: m['*'], patterns: Object.keys(m).filter((k) => k !== '*').length };
    return { value: getAt(tree, f.key), patterns: 0 };
  }

  async function commit(f: FieldSpec, v: unknown) {
    error = null;
    const path = patternMap(f) ? `${f.key}.*` : f.key;
    try {
      onchange(await configSetValue(text, format, path, v));
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="form">
  {#if !inTauri()}
    <p class="muted">데스크톱 앱에서만 설정을 편집할 수 있습니다.</p>
  {:else if parseError}
    <p class="callout callout-warn">현재 텍스트를 파싱할 수 없습니다 — Raw 탭에서 구문 오류를 먼저 고쳐주세요.</p>
  {:else if !tree}
    <p class="muted">읽는 중…</p>
  {:else}
    {#each sections as s (s.title)}
      <section class="card">
        <header>
          <h3>{s.title}</h3>
          {#if s.description}<p class="desc">{s.description}</p>{/if}
        </header>
        {#each s.fields as f (f.key)}
          {@const fv = fieldValue(f)}
          <SettingControl
            field={f}
            value={fv.value}
            keyLabel={fv.patterns > 0 ? `${f.key}."*"` : f.key}
            onchange={(v) => commit(f, v)}
          >
            {#snippet extra()}
              {#if fv.patterns > 0}
                <span class="pat" title="패턴별 세부 규칙은 Raw 탭에서 편집하세요">+패턴 {fv.patterns}</span>
              {/if}
            {/snippet}
          </SettingControl>
        {/each}
      </section>
    {/each}
    <p class="note">
      변경은 즉시 텍스트에 반영되고, 저장 버튼을 눌러야 파일에 기록됩니다. '기본값'을 고르면 키가
      지워져 에이전트 기본 동작을 따릅니다. 여기에 없는 키는 Raw 탭에서 편집하세요 — 그대로
      보존됩니다.
    </p>
  {/if}

  {#if error}<p class="callout callout-error" role="alert">{error}</p>{/if}
</div>

<style>
  .form {
    flex: 1;
    overflow: auto;
    padding: 0.2rem 0.1rem;
    container-type: inline-size;
  }
  .card {
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 0.7rem 0.9rem 0.3rem;
    margin-bottom: 0.75rem;
  }
  header {
    padding-bottom: 0.45rem;
    border-bottom: 1px solid var(--border);
  }
  h3 {
    margin: 0;
    font-size: var(--fs-lg);
    font-weight: 600;
    color: var(--text-1);
  }
  .desc {
    margin: 0.15rem 0 0;
    color: var(--text-3);
    font-size: var(--fs-xs);
  }
  .pat {
    font-size: 0.64rem;
    color: var(--ask);
    background: var(--ask-soft);
    border: 1px solid var(--ask-line);
    border-radius: 999px;
    padding: 0.05rem 0.45rem;
    white-space: nowrap;
  }
  .note {
    color: var(--text-3);
    font-size: var(--fs-xs);
    margin-top: 0.6rem;
    line-height: 1.5;
  }
  .callout-error {
    margin-top: 0.4rem;
    word-break: break-all;
  }
</style>
