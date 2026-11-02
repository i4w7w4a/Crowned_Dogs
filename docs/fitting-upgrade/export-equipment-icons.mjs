import { mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { ITEMS } from "@crowned-dogs/core";
import { equipmentItemIcon } from "../../apps/web/src/base/equipment-icons.ts";

const root = new URL("./", import.meta.url);
const output = new URL("icons/", root);
const kinds = ["weapon", "head", "body", "suit", "boots", "knife", "grenade"];
const items = ITEMS.filter(item => kinds.includes(item.kind));
const escape = value => String(value).replace(/[&<>"']/g, character => ({
  "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&apos;",
}[character]));
const manifest = {
  name: "Crowned Dogs equipment pictograms",
  source: "apps/web/src/base/equipment-icons.ts",
  catalogue: "packages/core/src/items.ts",
  count: items.length,
  viewBox: "0 0 48 48",
  defaultColor: "#f2f3f3",
  colorUsage: "All paths use currentColor. Standalone files set color on the SVG root for viewing. Override it with CSS on the inline svg, for example .cd-equipment-icon { color: #8f969b; }. External img elements cannot inherit their parent's color. Runtime equipmentIcon() has no fixed color.",
  regenerate: "node docs/fitting-upgrade/export-equipment-icons.mjs",
  items: [],
};

await mkdir(output, { recursive: true });
for (const item of items) {
  const runtime = equipmentItemIcon(item.id);
  if (!runtime.startsWith("<svg ")) throw new Error(`Missing equipment icon: ${item.id}`);
  if (!/^[a-z0-9-]+$/.test(item.id)) throw new Error(`Unsafe filename: ${item.id}`);
  const standalone = runtime
    .replace('aria-hidden="true"', `role="img" aria-label="${escape(item.name)}"`)
    .replace('<svg ', '<svg width="48" height="48" color="#f2f3f3" ');
  await writeFile(new URL(`${item.id}.svg`, output), `${standalone}\n`);
  manifest.items.push({
    id: item.id,
    name: item.name,
    kind: item.kind,
    slots: item.kind === "weapon" ? (item.weapon === "pistol" ? ["weapon1", "weapon2"] : ["weapon1"]) : [item.kind],
    file: `${item.id}.svg`,
    note: item.note,
  });
}
await writeFile(new URL("manifest.json", output), `${JSON.stringify(manifest, null, 2)}\n`);

// Preserve the reviewed glass appearance; the sheet remains a standalone file.
const previous = await readFile(new URL("equipment-icons.html", root), "utf8");
let style = previous.match(/<style>([\s\S]*?)<\/style>/)?.[1];
if (!style) throw new Error("Contact sheet CSS missing");
style = style.replace(/\/\* Full catalogue overview \*\/[\s\S]*?\/\* End overview \*\//, "").trimEnd();
style += `\n/* Full catalogue overview */
@media(min-width:1300px){main{max-width:1480px;padding:20px}header{margin-bottom:22px;padding-bottom:12px}h1{font-size:22px}.grid{grid-template-columns:repeat(8,minmax(0,1fr));gap:18px 12px}.row{gap:6px}.large{width:56px;height:56px;border-radius:12px}.large svg{width:44px;height:44px}.small{width:40px;height:48px}.small svg{width:32px;height:32px}h2{font-size:11px;line-height:1.35;margin:8px 0 2px;min-height:30px}.sample p{font-size:9px}footer{max-width:none;margin-top:20px;font-size:11px}}
/* End overview */`;
const samples = items.map(item => {
  const art = equipmentItemIcon(item.id);
  return `<article class="sample"><div class="row"><span class="glass icon large">${art}</span><span class="glass icon small selected">${art}</span><span class="glass icon small">${art}</span></div><h2>${escape(item.name)}</h2><p>${escape(item.id)}</p></article>`;
}).join("\n");
const html = `<!doctype html>
<html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Crowned Dogs · Equipment pictograms</title>
<style>${style}</style><main><header><h1>CROWNED DOGS / СНАРЯЖЕНИЕ</h1><p>${items.length} предмета каталога · сетка 48 × 48 · контур 1,7 · детали 1,2</p><p>Слева — увеличенный знак. Справа — 32 px, экипирован / доступен.</p></header><section class="grid">
${samples}
</section><footer>Полный комплект: четыре класса оружия, головные уборы, бронежилеты, костюмы, обувь, ножи и гранаты. Названия и идентификаторы сверены с каталогом игры. Цвет задаёт интерфейс через currentColor. Отдельные SVG и manifest.json находятся в папке icons.</footer></main></html>\n`;
await writeFile(new URL("equipment-icons.html", root), html);
console.log(`${items.length} equipment SVGs exported to ${fileURLToPath(output)}`);
