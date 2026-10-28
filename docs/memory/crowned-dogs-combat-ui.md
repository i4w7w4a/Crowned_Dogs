---
name: crowned-dogs-combat-ui
description: Combat HUD (2026-10-27): Art's 18:21 mock-up layout + Apple-style liquid glass buttons (19:19), defence as glass buttons, glass menu; how HUD changes ship
metadata:
  type: project
  modified: 2026-10-27T20:00:14.425Z
---

History: v4 icons «по-детски» → thin line icons. 16:40 Art's new visual style: «по нему всё делаем»; top HUD of his screenshot 3 «супер», bottom must beat his mock. Branch claude/project-thread-kxnjvc → integration. See [[crowned-dogs-visual]], [[crowned-dogs-arena]].

Layout = Art's mock-up of 18:21 (2026-10-27):
- Top: name small caps, HP 24px, thin white line toward the centre (red lag on a hit); centre «ХОД 1 / 15» over the clock (red last 10 s).
- Picture fills the screen; the panel has no fill (Art 17:26), words carry dark shadows.
- Distance: chevron column at the right edge; middle line = distance after the move («7 шагов», tap = stay, amber «без атаки»).
- Defence bottom left, «Защита 2/2»; **hologram removed** (Art 18:25; old code: hud/holo.ts before 4098e7b).
- Attack: a shot = tap a ring on the enemy (no «Огонь»). Knife + grenade side by side over «Подтвердить ход». Aimed ring label on a short amber line: «Корпус · 73%».
- Under the controls: log line (opens upward, hides the controls) + menu button.

Look (Art 19:19, supersedes 19:07): **Apple-style liquid glass** on every button, fight and menu: the SAME black 60% tint in every state, even disabled (Art 10-28 07:10; was 15%, then 40%); light 2px blur (strong blur was «слишком сильный»); rim lit top-left/bottom-right; **no outlines** (Art 20:07): picked = brighter edge + inner glow, bold white text. Chromium also bends the picture at the edge (apps/web/src/hud/liquid.ts, --lens). Defence = 4 glass buttons, each with a small figure lighting its zone (Art disliked a plain list with ticks, then with radio circles). Menu = floating glass sheet, glass controls, switches, round slider knobs, «Звуки боя»/«Музыка» sliders. Words and icons in buttons are **white** (Art tried black at 19:56, back to white at 20:04).

Fonts: Inter Tight + Inter + IBM Plex Mono (Barlow/Saira lack Cyrillic). Movement numbers: Art said keep them, **explain in the tutorial**.

Shipping: arena URL is built from apps/web on claude/integration; the arena thread merges and publishes. Never publish /mnt/project-files/crowned-dogs/arena/index.html (obsolete). Branch off integration, `npm run check` + `npm run web:build`, ping the arena thread. Art doesn't do git.

While a turn plays out (confirm tap → next turn, 2.5 s) the controls and the rings hide; log line + menu stay; the enemy's hit on us floats over our fighter (Art, 2026-10-28 morning, commit 0d5a149).
State 2026-10-28 09:20: all HUD work merged (last commit 069fd2f, in integration 32f6b0f). Open: in the menu's «От первого лица» camera, showOnMe's numbers (30%/60% of the scene) float over empty ground; not adjusted yet. Screenshot harness: /mnt/project-files/crowned-dogs/arena-ui/tools/shot.js (Playwright + SwiftShader, `node shot.js out.png "js" waitMs`, env VP/DSF/CLIP/SITE).
