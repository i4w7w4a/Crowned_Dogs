---
name: crowned-dogs-graphics
description: Arena graphics levels (low/medium/high/ultra), what each adds, lens pipeline, texture tiers, Art's 8K ask and upscaled backdrop (2026-10-27)
metadata:
  type: project
  modified: 2026-10-27T20:11:24.140Z
---

Art 2026-10-27 18:50: the arena picture "рябит"; wants smoothing, volumetric light, graphics levels in settings, and ultra "ооочень качественная и дорогая". Asked for texture levels and suggested 8K on ultra; told him 8K adds nothing (Tripo sources are 4K, a phone shows the fighter 500–900 px tall, 8K x15 maps would crash phones), so levels are 512 / 1K / 2K / 4K. He accepted implicitly and sent an upscaled hall (grey-hall-2, 1882x3344, kept at /mnt/project-files/crowned-dogs/arena/backdrops/).

Shimmer causes found: CSS grain overlay swapping noise every 90 ms (removed), jittered 12-tap lens smear (now mip-based taps, no noise), our figure at 1/2.5 size without MSAA (now MSAA + B-spline read), PCF shadows with 5 IGN-rotated taps (now 8–32 fixed taps via materials.ts), glossy normal maps (specular AA).

Shipped: PR #11 merged into claude/integration 19:47 and published as arena v19 (19:48). Code: apps/web/src/arena/quality.ts (presets, guessQuality: phone low/medium, PC high, never ultra auto; fps < 24 steps down unless picked by hand; ?q=ultra forces), lens.ts + lens-shaders.ts (compose → haze → rays → lens passes, ultra SSAA 1.5x), dust.ts, materials.ts, textures.ts (TextureTiers: tex/<model>.json manifest, 2K/4K webp from scripts/texture-tiers.py run on Tripo zips; 512 shrunk in browser). Frames have src2x for high/ultra; look.lens has rays/haze/hazeColor. Env map from the painting; hemi light lowered to 0.22 when env is on so brightness matches what Art approved.

Female hair is now alpha-cut strips (M_Fem_HairCards, 512x1024, alphaToCoverage needs MSAA); no tier for it.

Focus on the enemy (Art 2026-10-27 19:56: "фокус на врага, а за ним все немного размыть"): merged as PR #12 into claude/integration and published as arena v23 (2026-10-28 07:01). Depth maps public/img/grey-hall-{1,2}.depth.png (value×1.25 = 1/z, Depth Anything V2 fitted to the floor; the calibration frame has none). The blur radius is dof·min(|1/zf−1/z|, 0.13)·picture height, with dof 0.035 (0.055 looked too strong) and the front at dofNear 0.3. Taps per level: 1/8/16/32. Far dust specks get the same blur. Before/after picture: /mnt/project-files/crowned-dogs/arena/focus/focus-before-after.jpg. A new hall needs its own depth map, or it stays sharp.

**How to apply:** new models need `python -I apps/web/scripts/texture-tiers.py <model> <Material>=<dir with color/normal/rough/metal.jpg> ...` or they just keep their 1K maps. Ask Art for an upscaled grey-hall-1 too (only hall 2 has a 2x copy). See [[crowned-dogs-arena]], [[crowned-dogs-visual]].
