---
name: persona-male-base
description: Finished male Persona base model (2026-10-27): files, specs, rest-pose standard, preview page, open questions to Art
metadata:
  type: project
---

Done 2026-10-27 ~13:00 UTC via Tripo website + Blender on Blade. Folder: C:\Users\qtas5\Documents\CrownedDogs\3d\01-persona-male\ (NOTES.md = prompts, settings, credits; ref\ = ChatGPT and Tripo reference images).
- blender\: master 138k tris and game 45k tris (body 30k, head 15k); each as .blend/GLB/FBX: whole, body-only, head-only, all with the armature.
- 1.80 m, scale 1, origin between the feet, facing forward. UE5 Mannequin skeleton, 61 bones, no face bones.
- **Rest-pose standard: upper arms 65.6° below horizontal.** The female base and every future suit must be generated/fitted in this pose.
- Head = separate mesh weighted 100% to head, lower neck blended to neck_01; neck connector ring hides the joint.
- Two 4K texture sets (body, head): base colour, normal (_N; flip green channel in UE), ORM. ChatGPT tile detail (fabric weave, quilting, gunmetal ring, boots, skin pores); suit tint is one value. Body kept its original Tripo UV (Tripo Smart UV was worse 3 times); head uses Tripo Smart UV, overlaps repacked in Blender.
- Tripo credits 6715 -> 6490 (55 wasted on an accidental duplicate body).
- Preview page (private): https://claude.ai/artifact/LwzMqtGx6XeBU2UboJb4iY ; JPEGs in preview\share\.
- 13:30 UTC Art approved it as is («всё гуд»): subtle texture detail stays, the thin cyan spine channel stays.
- Female base (02-persona-female, 1.70 m, same suit/skeleton/pose): Art requires approving the sketch BEFORE any 3D generation. Rule for future bases/suits too: show reference sketches first.
- 13:42 UTC Art paused the female base entirely (sketches too) until he says otherwise. Before the stop, ChatGPT made 5 unreviewed sketches (bodies 1-3: slim/medium/strong; heads 4-5: low bun/undercut) in 02-persona-female\ref\.dispatcher\images\ (PROMPTS.md there); no Tripo credits used. Show these to Art first when resuming.

- Web/game asset (2026-10-27 14:30): /mnt/project-files/crowned-dogs/3d/persona-male/persona-male.glb (1.41 MB, meshopt + WebP 1K, clips idle/walk/run baked from X Bot, faces +Z) + README.md + retarget.py (bpy 5.2.3 pip module works in the cloud; gltf-transform CLI for webp/meshopt). 14:36 live in the arena (v8, PR #10 into claude/integration, glb committed in apps/web/models); both fighters, hologram and reflection use it. Our foreground figure reads as a dark silhouette (black suit, backlight); arena thread can lighten it in one line if Art asks. Art's PC copy: 01-persona-male\web\persona_male_web.glb (no anims, 5.2 MB).

- 2026-10-27 16:40 Art set a NEW visual style (style-2: gritty near-photoreal, fully geared masked Personas; refs /mnt/project-files/crowned-dogs/art/style-2/) and said he will provide new Persona models himself. Hold all new Persona generation; when his models arrive, the reusable part is the web pipeline (retarget.py idle/walk/run bake + webp/meshopt) in /mnt/project-files/crowned-dogs/3d/persona-male/.

See [[crowned-dogs-3d-pipeline]], [[tripo-3d]].
