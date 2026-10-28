---
name: persona-female-base
description: Female Persona from Art (2026-10-27 18:10): his Tripo body/head/hair parts, what to build, animation plan and reference video pack
metadata:
  type: project
  modified: 2026-10-27T19:38:00.000Z
---

2026-10-27 18:10 Art sent the female Persona himself (overrides the earlier pause in [[persona-male-base]]): three Tripo FBX zips, FemBody (rigged, 61-bone UE skeleton, headless, 51k tris, 4K textures, 5 Tripo animations: fire, throws grenade, takes a shot hit, shoots with pistol, aims with pistol), Femhead (bald head with metal jaw pieces and a white collar, 51k tris) and FemHari (short messy bob, 56k tris), plus a full-body ref image (white cyborg suit, short bob). Originals: /root/.claude/uploads/.../83a4a574-FemBody.zip, c80c4a76-Femhead.zip, 77ae1259-FemHari.zip (copies unpacked in the core thread's scratchpad fem/in).

His ask: join head to body, fit the hair (later the hair will be redone as real hair in Blender), put her in the game with the animations as they are first, then adapt the animations to our game. 18:13 he sent the reference animation pack to base the adaptation on: four videos (three grok-video clips and Shotgun.mp4). He allows generating textures with ChatGPT where needed.

Done 18:37 (core thread, cloud bpy 5.2.3): files, scripts and README in /mnt/project-files/crowned-dogs/3d/persona-female/ (persona-female.glb 2.5 MB, 60k tris, Art's 5 clips untouched + idle/walk/run baked from X Bot). Viewer page with clip buttons: https://claude.ai/artifact/GeU1c51BY963iEvWFM1SYF. Arena v12: the enemy Искра is the female Persona, we still play the male (main.ts MY_MODEL/FOE_MODEL; models in metres keep their own height). Hair decimated to 15k looks like chunky cards; Art will redo hair in Blender later.

The Remote Control session on Art's PC started the same assembly in parallel at 18:13 (it restarts on Art's messages); I stopped RC at 18:36 to avoid two versions. Restart RC (start_remote_control, Art approves) when ChatGPT textures via his dispatcher are needed.

19:02 Art sent Fem_Hair_Strands.blend (curves groom: 334 guides, 66,584 strands; HAIR_ROOT empty, SOURCE mesh copy). Done 19:33: seat.py puts the groom on fem.blend (root bone-parented to head, scale 0.289, error 8e-7); old mesh hair kept hidden as FemHairMesh. For the game cards.py builds 668 hair strips (one per guide x 2 layers, 16k tris, texture hair-strands.png) plus a dark cap; in the glb as FemHairCards (MASK 0.15, double-sided, alphaToCoverage in arena.ts and viewer). Blender renders use his strands, the game the strips. Files: persona-female/hair/ and fem.blend (158 MB packed). Arena v17, viewer v2. If Art later wants denser game hair, the graphics thread could add an ultra hair LOD.

2026-10-28 06:48 Art: «дай мне блендер с женской персоной, ты даже не постарался голову соединить» (the head only stood on the body's turtleneck: a visible step). Sent him fem.blend, then fixed it: join_neck.py cuts the body's neck at z 1.405 and grows one smooth tube (72 columns x 10 rings, Hermite between the body's and head's surface directions) up to 6 mm over the head's rim, painted with a plain suit-white uv patch (0.1416, 0.358), weights blend to neck_01. Result persona-female/fem-v2.blend (+ neck-before-after.jpg); checked in poses. fem.blend kept unchanged. 07:10 Art: head a little lower (shorter neck): lower_head.py moves the head bone, head mesh and all hair down 2 cm, then join_neck.py again -> fem-v3.blend (+ neck-shorter.jpg) = current female source; game glb is rebuilt from it (apply_mocap -> hair/export_fem.py -> hairmask -> gltf-transform). Lesson: check joins in close renders before handing over; Art notices sloppy seams.

Next: adapt the clips to the game using Art's 4 reference videos (uploads 5e937e50, d4a1c21b, 8679ecac grok-video and ec221b08 Shotgun.mp4). Open point from the RC: the back-of-neck seam shows over the shoulder if she becomes our own figure.

See [[crowned-dogs-3d-pipeline]], [[tripo-skeleton]].
