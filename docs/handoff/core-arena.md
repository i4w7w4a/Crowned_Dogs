# Handoff: combat core, bot, arena, 3D (thread «Разработать бой, бота и арену»)

State on 2026-10-28 09:40 UTC.

## Where things are

- **Repo**: github.com/code-of-stalker/Crowned-Dogs. Art merged PR #7 at 10:09 UTC, so `main` has all finished
  work (integration 01c4e5f). Every `claude/*` branch is inside `main`; start new work from `main`.
- **Arena (trial web client)**: https://claude.ai/artifact/PxEDdVDNb7UTZtwBs6goxU, version 31, private (it carries
  Sonniss sounds, so it must stay private). It is the build of `apps/web`.
- **3D sources** (project files, `crowned-dogs/3d/`): see the READMEs in `persona-male-art/`, `persona-female/`,
  `anim-mocap/` (mocap clips; game sources `male-anim-v2.blend`, `fem-anim-v3.blend`), `armor/` (armour thread's
  fitting), `armor-game/` (armour in the game models: dressed files, steps).
- **Headless test scripts**: `crowned-dogs/web-test/*.js` (Playwright, Chromium with SwiftShader; serve
  `apps/web/dist` with `python3 -m http.server PORT` and pass the URL). Useful: `shot.js`, `close-test.js URL OUT 2,6 [WxH]`,
  `fp-test.js` (first person), `menu-cam.js` (camera setting), `tiers-test.js` (texture levels).

## Build, check, publish

```
npm install
npm run check        # typecheck (both tsconfigs) + 102 tests
npm run web          # dev server for apps/web
npm run web:build    # apps/web/dist (models served as base64 models/*.json by a vite plugin)
npm run bot          # Telegram bot, token in a local gitignored .env (never commit it)
```

Publishing the arena from claude.ai: Artifact publish `apps/web/dist/index.html` to the arena URL with files
`assets/arena.js`, `assets/index.css` (fixed names) plus any changed `models/*.json`, `img/`, `tex/` files
(files left out are kept). Only a `#hash` reaches the page (`#fp` forces first person). In a local Claude Code
session there is no Artifact tool: run `npm run web` or serve `dist` to look at it, and publish from a claude.ai
session when a shared link is needed.

Blender work runs as a Python module: `pip install bpy==5.2.3` (Python 3.11-3.13), scripts take `-- args`.
gltf-transform 4.5.1 for model compression.

## What was done last (this thread)

- v29: first person with bodycam grips (IK viewmodel), planted feet, softer stances.
- v30: menu «Камера»: «Из-за плеча» / «От первого лица», remembered in localStorage `cd-camera` (Art 09:02).
- v31: armour kit A on both Personas (suit, «Плита-2», «Каска К-6»); body under the suit hidden; female hair
  swapped for the helmet cut; first person shows sleeves and gloves; 2K/4K armour maps for high/ultra.

## Open tasks

1. Art has not chosen the default camera (now over the shoulder). If first person becomes the default, the UI
   damage numbers over our fighter (`showOnMe`) need a new spot.
2. Real gun models from the weapons thread («Сделать оружие и обвесы») to replace the stand-in boxes
   (`apps/web/src/arena/weapons.ts`; first-person holds in `viewmodel.ts` HOLD per stance).
3. Per-item armour: only one mesh per slot exists, so both fighters always wear kit A
   (`dress.ts` KIT_A). Map catalogue items (`packages/core/src/items.ts`) to meshes when more garments exist.
4. Rough edges: the pistol stand-in reads as a slab; left pistol fingers look spread; knife/grenade first-person
   frames show a big near upper arm.
5. The Persona viewer page still shows the old female.

## Rules Art set (keep them)

- Talk to Art in plain short Russian, no tech detail; he doesn't do git.
- Never modify Art's uploaded originals; never copy the Tripo API key anywhere.
- Sonniss sounds are never published standalone.
- Commit trailers / PR footers as the session requires; no model ids in commits or PRs.
