---
name: crowned-dogs-weapons-3d
description: 3D gun models for Crowned Dogs (started 2026-10-27 18:24 on Art's «пора собирать пушки»): plan, Blade folder, phases, approval gates
metadata:
  type: project
  modified: 2026-10-27T18:25:56.384Z
---

Started 2026-10-27 18:24 in the «Сделать оружие и обвесы» thread after Art wrote «пора собирать пушки».

Plan (modular, matches the socket spec /mnt/project-files/crowned-dogs/weapons/weapons-3d-spec.md):
1. Reference images of the 4 guns in style 2 (Tripo's own image generator on the website, via Remote Control on Blade). Art approves them before any 3D: he wants sketches shown before 3D spending.
2. 3D generation on the Tripo website: a stripped body per gun plus each part as its own generation (Tripo normalises everything to 1 unit, so scale by the spec's mm sizes). Test the whole chain on «Варяг» first.
3. Assemble in Blender: parts at sockets (empties socket_optic/muzzle/rail/mag/stock), a light game version (decimate, 1K textures, base64 .json for artifacts like the Persona). bpy 5.2.3 works in the cloud, so assembly can run here once the GLBs are fetched.
4. Remaining guns and mods, then hand the files to the dev and arena threads.

Blade folder: C:\Users\qtas5\Documents\CrownedDogs\3d\03-weapons\ (ref, tripo, blender, preview, NOTES.md with prompts and credits). Tripo web credits only; never the API key.

Progress: refs page https://claude.ai/artifact/8Ti5rFoAH3oNmRfXBxGMwp (8 images, 0 credits). 18:41 Art approved pistol A, shotgun A, rifle B, sniper A. Step 2 sent to the Blade session: «Варяг» only, with Tripo image-to-3D, stock, mag and muzzle split off, test mods optic-sokol and muzzle-tishina, socket empties, a game GLB and a page «Варяг: 3D». Decision: factory iron sights and handguard stay IN the body, so the factory optic and rail sockets are empty (the spec's *-optic-stock and *-rail-stock parts need no mesh). The Tripo account is Pro, so rights are fine.

PAUSED 2026-10-27 20:10 (Art: «все треды на паузу, готовь ноут к отключению»). Saved on Blade in 03-weapons: HD-based master + game version in blender\ (body 9,849 tris, parts ≤2,954, total 22,017, 1K textures); web\varyag.glb 2.7 MB + 4 renders; page web\varyag-page built but NOT published. Sockets in mm from the grip origin (forward/up): optic 85/161, muzzle 545/101, rail 325/67, mag 160/59, stock −43/81; parts seat cleanly. Smart Mesh P2 rifle (Tripo task 4b47e25e) downloaded to tripo\: crisper shapes, 16k tris, colour-only texture (no metal/roughness). HD vs P2 comparison unfinished. Tripo's own segmentation gave 22 pieces with holes, so cutting in Blender is cleaner. Credits: 305 spent by this work, balance 5090. Next: finish HD vs P2, build the game GLB from the better one, publish «Варяг: 3D», show Art, then the other guns with P2. Details in 03-weapons\NOTES.md, section «Где остановились».
RESUMED 2026-10-28 06:39 (Art: «продолжай»): a new Blade session was started to finish HD vs P2 and publish «Варяг: 3D».
PR #3 (catalogue, kits, icons) was merged 2026-10-27 16:22 into claude/combat-core-telegram-bot; it is in claude/integration and the branch claude/project-thread-3rmjpo was deleted on the remote.

See [[crowned-dogs-gear]], [[crowned-dogs-visual]], [[tripo-3d]], [[art-spending-approval]].
