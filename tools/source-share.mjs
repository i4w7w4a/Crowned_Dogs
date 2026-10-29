/** Reproducible source-language inventory. Dependencies/build outputs are excluded by Git. */
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { extname } from 'node:path';

const refIndex = process.argv.indexOf('--ref');
const ref = refIndex < 0 ? null : process.argv[refIndex + 1];
const git = (...args) => execFileSync('git', args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
const files = [...new Set((ref ? git('ls-tree', '-r', '--name-only', ref) : git('ls-files', '--cached', '--others', '--exclude-standard')).trim().split(/\r?\n/))];
const extensions = new Set(['.rs', '.ts', '.js', '.mjs', '.css', '.html', '.glsl', '.vert', '.frag', '.py', '.ps1', '.sh', '.cmd', '.bat', '.tsx', '.jsx', '.wgsl']);
const rows = [];
for (const file of files) {
  const language = extname(file).slice(1);
  if (!extensions.has(extname(file)) || file.includes('.generated.') || file.endsWith('.d.ts')) continue;
  let source;
  try { source = ref ? git('show', `${ref}:${file}`) : readFileSync(file, 'utf8'); } catch { continue; }
  const working = /^(apps\/(web|server)\/src|apps\/desktop\/src|packages\/[^/]+\/src|crates\/[^/]+\/(src|shaders))\//.test(file)
    || file === 'apps/web/index.html' || file === 'apps/desktop/shell/index.html';
  let runtimeSource = source;
  if (language === 'rs') runtimeSource = source.split(/#\[cfg\(test\)\]/)[0];
  const count = value => value.split(/\r?\n/).filter(line => line.trim()).length;
  rows.push({ file, language, all: count(source), working: working && !/(pom-check|surface-check)\.(ts|html)$/.test(file) ? count(runtimeSource) : 0 });
}
function summary(scope) {
  const languages = {};
  for (const row of rows) languages[row.language] = (languages[row.language] ?? 0) + row[scope];
  const total = Object.values(languages).reduce((a,b) => a+b, 0);
  return { rust: languages.rs ?? 0, total, percent: Number(((languages.rs ?? 0) / total * 100).toFixed(2)), languages };
}
console.log(JSON.stringify({ ref: ref ?? 'working tree', method: 'Nonempty source lines, including comments. Working: production paths, no inline Rust test tails, generated code, declarations or POM check pages. Static JSON data, dependencies, builds, tests and reference snapshots are not working code.', working: summary('working'), repository: summary('all'), largestNonRust: rows.filter(row => row.language !== 'rs' && row.working).sort((a,b) => b.working-a.working).slice(0,25) }, null, 2));
