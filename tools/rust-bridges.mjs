// One-time migration helper. Reference sources are test oracles and TS contracts only.
import ts from "typescript";
import { readFileSync, writeFileSync, mkdirSync, copyFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const modules = ["rng", "rules", "match", "ai", "draft", "tag", "items", "loadout"];
const reference = resolve(root, "packages/core/reference");
mkdirSync(reference, { recursive: true });
for (const name of [...modules, "index"]) {
  const destination = resolve(reference, `${name}.ts`);
  if (!existsSync(destination)) copyFileSync(resolve(root, `packages/core/src/${name}.ts`), destination);
}
for (const name of modules) {
  const path = resolve(reference, `${name}.ts`);
  const file = ts.createSourceFile(path, readFileSync(path, "utf8"), ts.ScriptTarget.Latest, true);
  const constants = [], functions = [];
  for (const node of file.statements) {
    if (!node.modifiers?.some(m => m.kind === ts.SyntaxKind.ExportKeyword)) continue;
    if (ts.isFunctionDeclaration(node)) functions.push(node.name.text);
    if (ts.isVariableStatement(node)) for (const d of node.declarationList.declarations) {
      (d.initializer && ts.isArrowFunction(d.initializer) ? functions : constants).push(d.name.text);
    }
  }
  const lines = [
    '// Browser/Node ABI only. All game calculations are implemented in crates/game-core.',
    'import { callRust } from "../../runtime/src/index.ts";',
    `import type * as Contract from "../reference/${name}.ts";`,
    `export type * from "../reference/${name}.ts";`,
  ];
  if (constants.length) lines.push('const constants = callRust<Record<string, unknown>>("core.constants", []);');
  for (const key of constants) {
    lines.push(key === 'ITEM_BY_ID'
      ? 'export const ITEM_BY_ID: typeof Contract.ITEM_BY_ID = new Map(ITEMS.map(item => [item.id, item]));'
      : `export const ${key} = constants.${key} as typeof Contract.${key};`);
  }
  for (const key of functions) {
    if (["nextFloat", "nextInt", "roll", "pickWeighted"].includes(key)) {
      lines.push(`export const ${key} = ((...args: Parameters<typeof Contract.${key}>) => {
  const result = callRust<{value: unknown; state: number[]}>("core.${key}", args);
  args[0].splice(0, 4, ...result.state);
  return result.value;
}) as typeof Contract.${key};`);
    } else if (key === 'botOrder') {
      lines.push(`export const botOrder: typeof Contract.botOrder = (...args) => {
  if (!args[2]) return callRust("core.botOrder", args);
  const result = callRust<{value: ReturnType<typeof Contract.botOrder>; state: number[]}>("core.botOrder", args);
  args[2].splice(0, 4, ...result.state);
  return result.value;
};`);
    } else if (key === 'weaponSpec') {
      lines.push(`export const weaponSpec: typeof Contract.weaponSpec = (...args) => {
  const field = callRust<"weapon" | "sidearm" | "knife" | null>("core.weaponSpecSource", args);
  return field ? args[0].config.fighters[args[1]].kit![field]! : callRust("core.weaponSpec", args);
};`);
    } else if (key === 'formatTag') {
      lines.push(`export const formatTag: typeof Contract.formatTag = (...args) => {
  try { return callRust("core.formatTag", args); }
  catch (e) { throw new RangeError(e instanceof Error ? e.message : String(e)); }
};`);
    } else lines.push(`export const ${key}: typeof Contract.${key} = (...args) => callRust("core.${key}", args);`);
  }
  writeFileSync(resolve(root, `packages/core/src/${name}.ts`), lines.join('\n') + '\n');
}
