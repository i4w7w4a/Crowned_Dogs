---
name: crowned-dogs-gear
description: Crowned Dogs weapon/gear catalogue: 4 guns × 5 mod sockets, 8 Persona slots, tiers, kit balance results, where the data, icons, 3D spec and gear screen live
metadata:
  type: project
  modified: 2026-10-27T12:46:20.151Z
---

Built 2026-10-27 in the «Сделать оружие и обвесы» thread, PR #3 in code-of-stalker/Crowned-Dogs (stacked on PR #1 branch claude/combat-core-telegram-bot).

- Guns: «Пустельга» pistol, «Таран» shotgun, «Варяг» rifle, «Беркут» sniper. Sockets: optic, muzzle, rail, mag, stock; factory part + 2–4 swaps each. Persona slots: weapon1, weapon2 (pistol only as sidearm), head, body, suit, boots, knife, grenade. Tiers: серийное / армейское / штучное. Grenades: РГ-1, «Град», ЭМИ «Затмение» (shield off next turn), дымовая «Завеса».
- Rule kept on purpose: no kit or the starter kit plays exactly like the plain rules (replay hash test). Changing starter items breaks that.
- Balance (800 bot fights/pair): veteran kits keep all classes 47–55% avg; veteran beats own starter 57–70%; one item moves win rate −4…+4 points. Smoke grenade is weakest (−1…−4) because the bot uses it badly. Re-run with `npm run kits` and `npm run kits -- items`.
- Not done yet: switching to the sidearm mid-fight (kit.sidearm unused by the engine). The dev thread owns it (plan: the swap replaces that turn's attack, moves and defence stay), along with the kit picker in bot/web and reading weaponSpec/grenadeSpec instead of WEAPONS/GRENADE. Merge plan is owned by the dev thread: everything rides branch claude/integration (PR #7 into main, one button for Art); push to claude/project-thread-3rmjpo and ping the dev thread with the commit. Never ask Art to merge (see [[art-preferences]]).
- Gear screen for Art: https://claude.ai/artifact/GS1Y2g5bEKbGkUHHb74PFi. 3D spec (bodies + sockets in mm, 53 part meshes): /mnt/project-files/crowned-dogs/weapons/weapons-3d-spec.md. Icons: packages/icons/svg/<item id>.svg, rebuilt by `npm run icons`.

- Style 2 (Art, 2026-10-27 16:40) applies to all weapon/gear visuals: the 3D spec says cold grey photoreal, worn black/dark-grey guns, salt dust. Filled one-colour silhouettes already match the item icons in Art's fight-screen ref (grenade, knife), so the icon set stays as is.

**Why:** ChatGPT refuses weapon schematics, so silhouettes are hand-built SVG; the 3D thread models from the socket spec.
**How to apply:** Add new items in packages/core/src/items.ts plus art in packages/icons/src/shapes.ts, then rerun the kit sims. See [[crowned-dogs-balance]], [[crowned-dogs-3d-pipeline]], [[crowned-dogs-visual]].
