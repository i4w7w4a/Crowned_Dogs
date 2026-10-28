---
name: crowned-dogs-arena
description: Trial arena (painted backdrop + live three.js figures) link, how it is built from apps/web, calibration numbers, lens and artifact gotchas
metadata:
  type: project
  modified: 2026-10-27T17:22:35.078Z
---

Trial arena: https://claude.ai/artifact/PxEDdVDNb7UTZtwBs6goxU (private; holds Sonniss sounds, so it must stay private). Version 30 at 2026-10-28 09:30 (integration 32f6b0f): v30 camera choice in the menu (over the shoulder / first person, remembered); v29 bodycam grips + planted feet + softer stances; v28 lens follows our knife stab and grenade throw in first person; v27 first-person arms + gun, steady guns on the run, rings above the controls; v26 first person on trial via #fp (only a #hash passes into artifacts, never ?query); v25 mocap combat animations + stand-in guns + grenade; v22 camera stepped back 0.45 m; v19 graphics levels; v14 Art's own male Persona as our fighter, female Persona as the enemy Искра. Since v11 the panel has no fill and lies over the picture (Art's pick 17:26): .scene covers the whole screen, the picture cover-fits it (same vertical framing as his ref 03), panel height in --panel-h on #app.

**The arena IS the repo's web client build** (apps/web on claude/integration): real core rules (LocalDuel + bot), weapon picker + veteran kit, sound via packages/audio. To republish: `npm run web:build`, then Artifact publish apps/web/dist/index.html to that url with files assets/arena.js + assets/index.css (fixed names), plus img/ or models/*.json only when they change (files left out are kept). Headless check: scratchpad web-test/shot.js (one screenshot) or test_web.js (whole fight, slow under SwiftShader); serve dist with python http.server on a fresh port each time.

How it works: a portrait backdrop drawn as a texture inside the three.js canvas, figures over it with a perspective matched to the painting. Since v10 the backdrops are Art's empty grey salt hall frames (img/grey-hall-1/2.jpg, 941x1672, from his style-2 refs 04/05); calibration from his fight frame 03: fov 55, camera height 2.0 m, enemy feet at image (0.572, 0.608), enemy height 0.212 of image at 7 steps; our own figure at me {x -0.15, y 0.17, z -0.66} (close, bottom left, head top ≈ 0.43 like the ref). Attack rings are DOM buttons projected from bones.

Lens (src/arena/lens.ts, look in frames.ts BODYCAM; since PR #11 rebuilt per graphics level, see [[crowned-dogs-graphics]]): colour fringes with red spreading outwards, radial smear + soft blur growing to the edges (12 jittered taps), glow from the backdrop's mipmaps, extra contrast, vignette that stays bright almost to the edge then goes black in the corners. Our own figure renders on layer NEAR into a 1/2.5-size target, so it is soft. Art's rule (16:59): no fill and no gradient over the picture at all; the picture runs to the top and edges (body margin 0). Its comparison file: /mnt/project-files/crowned-dogs/arena/v10-obrazec-i-arena.jpg.

Figures: we are apps/web/models/persona-male.glb (Art's male, [[persona-male-art]]), the enemy persona-female.glb ([[persona-female-base]]); served as models/*.json by a vite plugin; rig.ts maps Mixamo or UE bones. Since 18:51 a separate graphics thread («Улучшить графику и добавить уровни качества») owns lens/lighting/AA and quality presets; it pings the core thread to merge and publish.

Movement row: cells show the distance after the move; Art said keep it and explain it in the tutorial.

Artifact gotchas: .glb is not a served type, ship models as base64 .json. Fonts since v10: Inter Tight + Inter (UI thread's HUD), IBM Plex Mono.

Camera step-back (Art 20:23 ask, done 10-28): frames.ts cal `back` 0.45 m added to every live depth, our figure keeps its screen corner, only smaller; `?back=` overrides for tuning.
Ownership: the UI thread does HUD/panel on its own branch and pings; the core/bot/3D thread merges into integration, publishes, and owns the scene and lens.

First person trial: see [[crowned-dogs-first-person]]. See [[crowned-dogs-visual]], [[crowned-dogs-audio]], [[crowned-dogs-repo]], [[persona-male-base]].

