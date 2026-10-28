---
name: crowned-dogs-armor
description: Armour kit A on both Personas (thread «Собрать броню и надеть на Персон», 2026-10-27/28): done files, fitting tool and its lessons, Tripo route, handoff to the arena thread
metadata:
  type: project
  modified: 2026-10-28T09:02:41.260Z
---

Art 2026-10-27 18:47: «нужно собрать броню и надеть на персон». **Done 2026-10-28 ~09:10**, handed to the arena thread.

- **Kit A** (Art picked 19:52): grey-green camo suit with pads, gloves, boots; «Плита-2» plate carrier (3 mag pouches, side pouch, grab handle); «Каска К-6» high-cut helmet (cover, rails, NVG mount, headset, cyan strobe). Kit B dropped. Sketch page https://claude.ai/artifact/1L87FZqSTy6mZupJHVZeZZ; result page https://claude.ai/artifact/4tGZxpLQDyaZS3NH38Uj8k.
- **Result**: /mnt/project-files/crowned-dogs/3d/armor/fitted/male-armor.blend, female-armor.blend = Persona file + Suit (slot suit), Vest (slot body), Helmet (slot head, bone head 100 %); body groups hide_Suit/hide_Vest; female FemHairCards_Helmet/FemHairCap_Helmet (hair_for=Helmet). 4K textures per garment, the game export (resize/webp) is the arena thread's job. Never commit into apps/web from here.
- Tripo GLBs in armor/tripo/ (suit-male, suit-female, vest, helmet; Smart Mesh + texture + PBR, 125 credits each; 4710 credits left). Blade copies in Documents\CrownedDogs\3d\04-armor\tripo (also free HD suits *-hd.glb).
- **Rebuild**: `sh armor/run_all.sh [OUT]` (bpy 5.2.3 + scipy, ~4 min) runs fit_garment.py with the per-piece settings; render_fit.py (pose2=1 crouch) and render_close.py for checks.
- Fitting lessons: Tripo GLB is split at UV seams → weld; generated head cut by colour (`cut_head=1`), female needs `neck_cut=1.472`, collar_trim hurt the male collar (opt-in only); `hide_below=auto` uses the neck opening only; don't push garments against hair meshes; vest/helmet are hollow → `inside` negative, per-axis multi-start refine against the suit surface, soft push (moves plates as a whole); hide_* groups must not count as skin weights.
- Tripo via RC on Blade: web only, never the API key. The RC session's built-in browser can lose the Tripo login: Art logs in there himself (Code tab → «Remote Control session», Ctrl+Shift+B). While RC runs, device folder tools are withheld; RC can publish files as base64 pieces in an artifact for the cloud to read.

See [[persona-male-art]], [[persona-female-base]], [[crowned-dogs-gear]], [[tripo-3d]].
