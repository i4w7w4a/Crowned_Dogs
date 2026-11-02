# Weapons and gear: handoff (thread «Сделать оружие и обвесы»)

State on 2026-10-28. Written for the local Claude Code session that continues Crowned Dogs.

## 1. In the repo (done, merged)

Everything below is merged (PR #3 → `claude/combat-core-telegram-bot`, carried in `claude/integration`).

- `packages/core/src/items.ts`: the catalogue. 4 guns («Пустельга» pistol, «Таран» shotgun, «Варяг» rifle, «Беркут» sniper), each with 5 mod sockets (optic, muzzle, rail, mag, stock): a factory part plus 2–4 swaps each. Also helmets, body armour, suits, boots, knives and 4 grenades (РГ-1, «Град», ЭМИ «Затмение», дымовая «Завеса»). Three tiers: серийное, армейское, штучное.
- `packages/core/src/loadout.ts`: `Loadout` → `compileKit` → `CombatKit`, `kitFighter`, `starterLoadout`, `VETERAN_KITS`, `validateLoadout`, `describeItem` / `effectLines` (Russian words for the UI).
- `packages/core/src/match.ts`: kits in combat (`weaponSpec`, `grenadeSpec`, `guardShare`, NVG/thermal, camo, double shot, EMP shield-off, smoke, per-gun suppressor).
- **Rule:** no kit, or the starter kit, plays bit-for-bit like the plain rules. A test checks this, so changing starter items breaks it.
- `packages/core/scripts/kits.ts`: `npm run kits` (veteran matrices) and `npm run kits -- items` (each item alone).
- `packages/icons`: 88 one-colour SVG silhouettes in `svg/<item id>.svg`. Guns are composed from a body plus parts at anchors. Rebuild with `npm run icons`.

Balance (800 bot fights per pair): veteran kits keep all classes at 47–55% on average, a veteran beats its own starter kit 57–70% of the time, and a single item moves the win rate by −4 to +4 points. The smoke grenade is the weakest item (−1 to −4) because the bot uses it badly.

Open, owned elsewhere: the sidearm swap mid-fight (`kit.sidearm` is not used yet), the kit picker in bot and web, and switching code from `WEAPONS`/`GRENADE` to `weaponSpec`/`grenadeSpec`. All of these belong to the dev thread «Разработать бой, бота и арену».

## 2. Gear screen (draft only)

- Page: https://claude.ai/artifact/GS1Y2g5bEKbGkUHHb74PFi. Sources are in `gear-page/`: `template.html`, `gen.ts` and `sim.json`. `gen.ts` imports the repo by absolute cloud paths, so fix those to your checkout. Run it as `node gen.ts data.json "$(cat sim.json)"`, then put the data in place of `/*DATA*/null`.
- Art's verdict: he likes the idea, but the UI is overloaded and inconvenient, so it needs a real design later. Start from less: one big gun, tap a socket to get a short list, numbers on demand, phone first.

## 3. 3D guns (in progress, on Blade)

- Spec: `weapons-3d-spec.md` (here). One body per gun plus 5 sockets, with sizes and socket positions in mm. Factory sights and handguard stay in the body, and the factory stock, mag and muzzle are separate meshes.
- Art approved the references (https://claude.ai/artifact/8Ti5rFoAH3oNmRfXBxGMwp): pistol A, shotgun A, rifle B, sniper A.
- Work folder on Blade: `C:\Users\qtas5\Documents\CrownedDogs\3d\03-weapons\`. `NOTES.md` there, section «Где остановились», is the source of truth for files, credits and next steps.
- «Варяг» status: the HD version is built (body 9.8k triangles, parts up to 3k, 1K textures, `web\varyag.glb` 2.7 MB). Sockets in mm from the grip origin (forward/up): optic 85/161, muzzle 545/101, rail 325/67, mag 160/59, stock −43/81. The test mods «Сокол» (scope) and «Тишина» (suppressor) fit. A Smart Mesh P2 version exists (crisper, 16k triangles) but only has a colour texture, and the comparison is unfinished. The «Варяг: 3D» page was NOT published: on 2026-10-28 the publish hung for about 30 minutes and I stopped the PC session. The page sources are on Blade in `web\varyag-page` (renders, three.js viewer, GLB as base64 JSON). Publish from there, or just open `web\varyag.glb` in Blender. NOTES.md «Где остановились» was last fully written on 2026-10-27 20:10; work from the morning of 2026-10-28 (HD vs P2) may be unsaved or unrecorded, so check file dates in `blender\` and `tripo\`.
- Next: finish HD vs P2, then pistol, shotgun and sniper with Smart Mesh P2 (Art prefers it), then the mods.
- Rules: Tripo website in the browser (Pro plan), never the API key. Art approves reference images before any 3D. Art said «Разрешаю тратить сколько угодно кредитов», but the PC asks for his own words before spending.

## 4. Memory files

`crowned-dogs-gear`, `crowned-dogs-gear-ui`, `crowned-dogs-weapons-3d`, `tripo-3d`, `art-spending-approval`.
