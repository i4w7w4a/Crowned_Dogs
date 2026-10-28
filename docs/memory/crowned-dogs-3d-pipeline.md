---
name: crowned-dogs-3d-pipeline
description: Art's 3D character pipeline for Crowned Dogs: Tripo body and head separately, join in Blender, Tripo rig + smart UV, male and female skeletons
metadata:
  type: project
---

Art's plan (2026-10-27), to start only after the art style is approved:
- Generate Persona models in Tripo with **body and head as separate generations** (face detail is much higher when the head is generated alone), then Claude joins them in Blender.
- Rig and smart UV are done in Tripo.
- Two skeletons: male and female.

Agreed refinements (told to Art): hide the neck seam under a collar, balaclava or the neck connector ring (the serial port on the neck from round-2 frame 08 fits the lore); both skeletons keep the same bone names and count, only proportions differ, so one set of animations serves both; jackets, trousers and vests must be fitted to both bodies, while helmets, masks, backpacks and weapons attach to bones and are made once. After joining, skin the head 100% to the head/neck bones.

Order of steps (Art, 2026-10-27): Smart UV in Tripo on head and body SEPARATELY, BEFORE joining; after joining Tripo demands a remesh and the face detail is destroyed. So: Tripo Smart UV on each part, Tripo auto-rig on the body only, then in Blender swap the head and weight it to the head bone; never re-upload the joined model to Tripo; keep two texture sets (body, head).

Textures (Art's idea, 2026-10-27): ChatGPT (dispatcher, Max) makes seamless tileable material samples (suit fabric, quilted pads, gunmetal ring, boot leather/rubber, skin micro-pores); they are layered in Blender over the Tripo base colour (keeps its AO/shading), drive bump and roughness, suit colour stays one tint value for player recolouring, then baked to one texture set. ChatGPT cannot paint a UV-exact face texture, so the face stays from Tripo plus skin micro-detail.

Tripo is unreachable from the cloud; run it through Remote Control on Blade (see [[tripo-3d]]). Relevant skills on Blade: Codex tripo-game-asset, blender-core/animation/export-pipelines (see [[art-pc-tools]]).

Faces (Art's decision 2026-10-27): a set of ready-made faces, NOT shape sliders. Each sex gets N prebuilt Tripo heads skinned only to head/neck_01; no face bones or blendshapes for now. Skeleton facts: [[tripo-skeleton]].
