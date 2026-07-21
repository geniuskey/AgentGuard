import {
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync
} from 'node:fs';
import { basename, dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { marked } from 'marked';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const docsDir = join(repoRoot, 'docs');
const outputDir = join(repoRoot, 'pages-dist');

if (dirname(outputDir) !== repoRoot || basename(outputDir) !== 'pages-dist') {
  throw new Error(`Refusing to replace unexpected output directory: ${outputDir}`);
}

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });

const preferredOrder = [
  'user-guide',
  'requirements',
  'architecture',
  'policy-model',
  'effective-policy',
  'security',
  'risk-scanner',
  'data-model',
  'ui-spec',
  'tech-stack',
  'roadmap',
  'claude-code-settings-plan',
  'open-issues'
];

const documents = readdirSync(docsDir, { withFileTypes: true })
  .filter((entry) => entry.isFile() && extname(entry.name) === '.md')
  .map((entry) => {
    const slug = basename(entry.name, '.md');
    const markdown = readFileSync(join(docsDir, entry.name), 'utf8');
    const title = markdown.match(/^#\s+(.+)$/m)?.[1]?.replace(/`/g, '') ?? slug;
    return { slug, title, markdown };
  })
  .sort((a, b) => {
    const ai = preferredOrder.indexOf(a.slug);
    const bi = preferredOrder.indexOf(b.slug);
    if (ai === -1 && bi === -1) return a.title.localeCompare(b.title, 'ko');
    if (ai === -1) return 1;
    if (bi === -1) return -1;
    return ai - bi;
  });

if (documents.length === 0) {
  throw new Error(`No Markdown documents found in ${docsDir}`);
}

if (!documents.some(({ slug }) => slug === 'user-guide')) {
  throw new Error('docs/user-guide.md is required as the GitHub Pages home page');
}

function escapeHtml(value) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}

function rewriteDocumentLinks(html) {
  return html.replace(/href="(?:\.\/)?([^"/]+)\.md"/g, 'href="$1.html"');
}

const navigation = documents
  .map(
    ({ slug, title }) =>
      `<a data-doc="${escapeHtml(slug)}" href="${escapeHtml(slug)}.html">${escapeHtml(title)}</a>`
  )
  .join('\n');

const styles = `
:root {
  color-scheme: dark;
  --bg: #080f1d;
  --panel: #0d1729;
  --panel-2: #111e34;
  --border: #233552;
  --text: #dce8f8;
  --muted: #8fa4c2;
  --accent: #67a7ff;
  --accent-soft: rgba(59, 130, 246, 0.14);
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font: 15px/1.7 Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}
.shell { min-height: 100vh; display: grid; grid-template-columns: 280px minmax(0, 1fr); }
.sidebar {
  position: sticky;
  top: 0;
  height: 100vh;
  overflow: auto;
  padding: 28px 20px;
  border-right: 1px solid var(--border);
  background: var(--panel);
}
.brand { color: white; font-size: 21px; font-weight: 750; text-decoration: none; }
.tagline { margin: 3px 0 22px; color: var(--muted); font-size: 12px; }
.sidebar nav { display: grid; gap: 3px; }
.sidebar nav a {
  color: var(--muted);
  text-decoration: none;
  padding: 7px 10px;
  border-left: 2px solid transparent;
  border-radius: 0 6px 6px 0;
  font-size: 13px;
}
.sidebar nav a:hover, .sidebar nav a.active {
  color: var(--accent);
  border-left-color: var(--accent);
  background: var(--accent-soft);
}
.repo-link { display: inline-block; margin-top: 22px; color: var(--accent); font-size: 12px; }
.content { width: min(100%, 980px); padding: 42px 52px 80px; }
.preview {
  width: 100%;
  margin: 8px 0 28px;
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 18px 50px rgba(0, 0, 0, 0.28);
}
article h1 { margin: 0 0 20px; font-size: 30px; line-height: 1.25; }
article h2 { margin: 34px 0 10px; padding-bottom: 7px; border-bottom: 1px solid var(--border); color: var(--accent); font-size: 21px; }
article h3 { margin: 26px 0 8px; font-size: 17px; }
article p, article li { color: #c8d6ea; }
article a { color: var(--accent); }
article code, article kbd {
  padding: 2px 5px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--panel-2);
  color: #b9d7ff;
  font-size: 0.88em;
}
article pre { overflow: auto; padding: 16px; border: 1px solid var(--border); border-radius: 8px; background: #070d18; }
article pre code { padding: 0; border: 0; background: transparent; }
article blockquote { margin: 16px 0; padding: 4px 16px; border-left: 3px solid var(--accent); background: var(--accent-soft); }
article table { width: 100%; border-collapse: collapse; margin: 16px 0; font-size: 13px; }
article th, article td { padding: 8px 10px; border: 1px solid var(--border); text-align: left; vertical-align: top; }
article th { background: var(--panel-2); }
article hr { border: 0; border-top: 1px solid var(--border); margin: 28px 0; }
.source { margin-top: 36px; color: var(--muted); font-size: 12px; }
@media (max-width: 800px) {
  .shell { grid-template-columns: 1fr; }
  .sidebar { position: static; height: auto; border-right: 0; border-bottom: 1px solid var(--border); }
  .sidebar nav { grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); }
  .content { padding: 28px 20px 60px; }
  article table { display: block; overflow-x: auto; }
}
`;

function renderPage(document) {
  const article = rewriteDocumentLinks(marked.parse(document.markdown, { async: false }));
  const preview =
    document.slug === 'user-guide' && existsSync(join(docsDir, 'assets', 'agentguard-overview.png'))
      ? '<img class="preview" src="assets/agentguard-overview.png" alt="Agent Guard project policy editor and command simulator" />'
      : '';
  const activeNavigation = navigation.replace(
    `data-doc="${document.slug}"`,
    `class="active" data-doc="${document.slug}"`
  );

  return `<!doctype html>
<html lang="ko">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta name="description" content="Agent Guard documentation" />
  <title>${escapeHtml(document.title)} · Agent Guard</title>
  <link rel="stylesheet" href="styles.css" />
</head>
<body>
  <div class="shell">
    <aside class="sidebar">
      <a class="brand" href="index.html">Agent Guard</a>
      <p class="tagline">Local-first coding agent policy editor</p>
      <nav aria-label="문서 목록">${activeNavigation}</nav>
      <a class="repo-link" href="https://github.com/geniuskey/AgentGuard">GitHub 저장소 →</a>
    </aside>
    <main class="content">
      ${preview}
      <article>${article}</article>
      <p class="source">Source: docs/${escapeHtml(document.slug)}.md</p>
    </main>
  </div>
</body>
</html>`;
}

for (const document of documents) {
  const html = renderPage(document);
  writeFileSync(join(outputDir, `${document.slug}.html`), html);
  if (document.slug === 'user-guide') writeFileSync(join(outputDir, 'index.html'), html);
}

writeFileSync(join(outputDir, 'styles.css'), styles);
writeFileSync(join(outputDir, '.nojekyll'), '');

const assetsDir = join(docsDir, 'assets');
if (existsSync(assetsDir)) cpSync(assetsDir, join(outputDir, 'assets'), { recursive: true });

console.log(`Built ${documents.length} documentation pages in ${outputDir}`);
