---
name: crowned-dogs-logo
description: Crowned Dogs logo: Art CHOSE the one-colour maw (ChatGPT r2-04 traced); clean vector, snapping SVG, 3D maw GLB, loading screen, PR #8, files, links
metadata:
  type: project
---

Art's logo idea (2026-10-27): "пасть пса из двух корон" — upper jaw = upside-down crown (its points = upper fangs), lower jaw = upright crown.

**DECIDED 2026-10-27 13:04 UTC: Art picked «Один цвет»** (one-colour bone #E9E3D3 on dark, from ChatGPT round-2 concept r2-04, traced) and said: make a 3D model from it and animate it for loading etc. Wordmark font Black Ops One. «Оскал» (round 1) stays plan B. No pearls/balls on crown points (Art: look silly).

Round 3 deliverables (2026-10-27):
- Clean vector: re-trace of r2-04 at 2x, specks removed, split into upper/lower jaw (`#jaw-upper`, `#jaw-lower`); maw-bite.svg snaps on its own (SMIL inside SVG, 43 KB).
- 3D maw built procedurally by a Blender script (bpy 5.2 module in a venv): crown rings with grooves/rims, rhombus gem studs, faceted bipyramid points, curved canines at ±90°, upper ring k=1.0, lower 0.9, hinge at the back; AO baked into vertex colours per jaw; clips Bite (2 s loop, snap at 0.53 s), Idle (4 s), Open (1 s); ~17k tris, 0.7 MB GLB. Rest pose = logo pose (upper -24°, lower +6°).
- Loading screen (three.js r186): each snap moves the progress bar; at 100% the jaws open, zoom and fade. Artifact: https://claude.ai/artifact/LV4FSuaNCdcVUKAzdkMMri (GLB shipped as base64 maw.json). "Вид" panel: clip, metal bone/steel/brass.
- Videos: maw-idle.mp4 (800², 8 s seamless breathing) and loading.mp4 (720x1280) rendered by frame-stepping the page (capture=1, renderAt(t)).
- Repo: PR #8 (branch claude/project-thread-06byqg) adds assets/brand/ (logo svg, png, 3d glb + build_maw.py, loader prototype, README).
- Project files: /mnt/project-files/crowned-dogs/logo/final/ (svg, png, 3d incl. .blend, video, loader). Older rounds: logo/ (round 1), logo/round2/.
- Gallery (all rounds, newest on top): https://claude.ai/artifact/8WwwBJmaQhjXrW2ENbNsFf

**Art 2026-10-27 13:39: keep «Дыхание» (Idle clip) and bone colour** for the loader; review panel removed; loader fills the bar on a fixed schedule, at 100% jaws gape (-44°/+12°), dive and fade. Art spotted a loose tooth inside the mouth: upper back teeth near the hinge (slots 7-8) pierced the lower ring when open; fixed by shortening them (BACK_UP {7: .45, 8: .35}), checked no upper vertex enters the lower ring for -2..-37°. 2D maw-breathe.svg added (same 4 s rhythm).

Next: port the loader into apps/web, Telegram kit (bot/channel avatar, cover, channel video avatar), optional Blender detailing (worn edges, scratches).

History: round 1 (front «Пасть», «Оскал» profile, ChatGPT art\08-logo), round 2 (premium angled maw, ChatGPT art\09-logo-r2, raw artifact SFBPwXvNTEc5hoUDA1Jor3). ChatGPT prompt lessons: say "upper crown upside down, its points ARE the upper fangs"; say "rim/band with sharp gems" or rings read as a trap.

See [[crowned-dogs-visual]], [[art-pc-tools]], [[art-preferences]], [[crowned-dogs-arena]].

2026-10-27 13:50: PR #8 CI green (d0e2f25). Core thread merged it into claude/integration (0544378, tests pass; one-button plan, see [[crowned-dogs-repo]]). Re-ping the core thread after any new logo push.
2026-10-27 13:51: Art accepted the logo, 3D maw and loader ("меня все устраивает") and said he does not need videos. No video files were made; the gallery's round-3 section still shows the older maw-loop clip, left as is.
