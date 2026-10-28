# Crowned Dogs: rules for Claude

Read `HANDOFF.md` first: it has the state of every area, where the files are and what is open. Older notes and decisions are in `docs/` (see the list at the end of HANDOFF.md).

## Talking to Art

- Art (GitHub `code-of-stalker`) owns the game and builds it with Claude, not with a studio. Write to him in Russian, in plain words, short. No deep technical detail in chat: say what changed and what he can look at. Depth goes into files, not messages.
- Show results he can see: a screenshot, a page, a render. Before spending Tripo credits on 3D, show reference images first and wait for his pick.
- Art does not do git. Do branches, commits, pushes and PRs for him. Ask him before merging into `main` or deleting branches.
- Art notices sloppy seams and crooked poses. Check close-up renders and screenshots before you hand anything over.
- Spending: Art gave standing approval for Tripo website credits and ChatGPT limits needed for the work. Ask before buying anything, changing a plan or topping up an API balance.

## Hard rules

- Tripo: the website only (studio.tripo3d.ai, Pro plan). Never use or copy the Tripo API key.
- Sonniss sounds (`packages/audio/assets`) may not be shared outside the game. Any page or build that holds them stays private.
- Visual style is «style 2» (Art, 2026-10-27): cold grey salt-factory near-photoreal, bodycam lens. Refs: `art/style-2/` in the project files. No cartoon look, no outlines, no fill or gradient over the arena picture, no water or drips in the salt hall.
- Combat UI: Apple-style liquid glass buttons, the same black 60% tint in every state, light 2px blur, white text and icons, no outlines (details in `docs/memory/crowned-dogs-combat-ui.md`).
- No kit, or the starter kit, must play exactly like the plain rules; a test checks this. Changing starter items breaks it on purpose.

## Code

- Node.js 22.18+, TypeScript run directly by Node (no build step), npm workspaces: `packages/core` (rules, match, bot AI), `packages/audio`, `packages/icons`, `apps/server` (Telegram bot, grammY), `apps/web` (three.js arena, Vite).
- Before every push: `npm run check` (types + 102 tests) and `npm run web:build`. CI runs the same.
- Balance numbers live in `packages/core/src/rules.ts`. After rule changes run `npm run balance` and `npm run kits`.
- Commit messages in English, short, describing the change for a player or a designer. README and texts for Art in Russian.
- Package.json scripts, `.gitignore`, README and `package-lock.json` are where branches usually conflict.

## 3D

- Blender 5.2 on this PC: `C:\Program Files\Blender Foundation\Blender 5.2\blender.exe` (not on PATH). Scripts in the project files also run with Blender as a Python module (`pip install bpy==5.2.3 scipy pillow`), always with `python -I`.
- Both Personas share the 61-bone UE Mannequin skeleton, so clips transfer by bone name. Each .blend keeps its own rest pose; fit garments to that file's pose.
- Game models: decimate, 1K WebP textures, meshopt via `@gltf-transform/cli` 4.5.1. 2K/4K tiers come from `apps/web/scripts/texture-tiers.py`.
