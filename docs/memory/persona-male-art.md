---
name: persona-male-art
description: Male Persona from Art's own Tripo parts (2026-10-27 18:45): files, scale, rest pose, now our fighter in the arena (replaces the older male)
metadata:
  type: project
  modified: 2026-10-27T18:53:26.266Z
---

2026-10-27 18:45 Art sent MaleBody.zip (rigged 61-bone UE skeleton, headless, 55k tris, 4K textures, NO clips) and malehead.zip (bald male head with metal jaw pieces and a white collar, 52k tris), no text. Same family as the female ([[persona-female-base]]); no hair this time.

Done 18:53 (core thread, cloud bpy 5.2.3): /mnt/project-files/crowned-dogs/3d/persona-male-art/ (male.blend with textures packed, persona-male.glb 1.47 MB = 30k body + 15k head, assemble.py/export_game.py/pack.py/render.py, README.md, head-check.jpg). Body 1.5618x, head 0.2632x, collar slid 25 mm down over the neck (body neck + collar looked too long), skull top 1.80 m. Rest pose: upper arms 50° below horizontal (female 61.5°); clips retarget in world space so it doesn't matter for them, but armour must fit each file's own pose. Clips: only idle/walk/run baked from X Bot.

In the arena as apps/web/models/persona-male.glb (same name, replaced the older Blade-made male from [[persona-male-base]]), our fighter; arena v14 (integration c135935). From behind it reads as a big bald skin-coloured head in the bottom left.

The armour thread («Собрать броню и надеть на Персон») fits armour on these .blend files and delivers skinned meshes to /mnt/project-files/crowned-dogs/3d/armor/; the core thread does the game export and arena. fem.blend in persona-female/ now has textures packed too (relative paths broke when copied, so pack from the scratchpad original).

See [[crowned-dogs-3d-pipeline]], [[tripo-skeleton]], [[crowned-dogs-arena]].
