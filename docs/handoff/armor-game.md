# Armour kit A in the game models

Since arena v31 (claude/integration 01c4e5f, 2026-10-28) both Personas wear the armour kit A that the armour thread
fitted (armor/fitted/*-armor.blend): Suit (field suit with gloves and boots), Vest («Плита-2»), Helmet («Каска К-6»).

## How the game models are made

1. `python3 -I dress_anim.py -- ANIM.blend ARMOR.blend OUT.blend` puts the garments on the Persona's game
   animation file (male: ../anim-mocap/male-anim-v2.blend, female: ../anim-mocap/fem-anim-v3.blend).
   - The female's head bone is 2 cm lower in the game file than in the fitted file, so the helmet and the
     helmet hair are moved with it.
   - The body's hide_Suit / hide_Vest groups become the point attributes `_suit` / `_vest`.
   - Garments are thinned: suit 0.4, vest 0.25, helmet 0.2, helmet hair cap 0.25.
   Results: male-armor-anim.blend, fem-armor-anim.blend (here, images packed).
2. Export with ../anim-mocap/scripts/export_male.py or export_fem.py (they export the attributes and the garments):
   `python3 -I export_male.py -- male-armor-anim.blend ../xbot/Xbot.glb raw.glb`
3. Female only: `node ../persona-female/hair/hairmask.mjs raw.glb raw-mask.glb` (needs npm @gltf-transform/core and /extensions; CLI @gltf-transform/cli 4.5.1).
4. `gltf-transform resample → dedup → prune → resize --width 1024 --height 1024 → webp --quality 88 → meshopt --level medium`,
   then copy to apps/web/models/persona-male.glb / persona-female.glb.
5. 2K/4K maps for high/ultra: `unpack.py` writes each garment's full maps (color.jpg, normal.jpg, rm.png), then
   `apps/web/scripts/texture-tiers.py` (armour pieces both wear live in public/tex/armor/, linked with NAME=@tex/armor/NAME).

## In the game

apps/web/src/arena/dress.ts: `dress(model, KIT_A)` shows Suit/Vest/Helmet, hides body triangles with a covered corner,
and swaps FemHairCards/FemHairCap for the *_Helmet cut. Both fighters wear KIT_A for now (one model per slot exists).
First person (rig.ts armsOnly) keeps only the suit's sleeves and gloves.

Checks: check-close.jpg (Cycles, ready_pistol frame 12, `look.py`), check-arena.jpg (arena: over the shoulder ×2, first person).

Next: per-item meshes when more garments exist (map catalogue items in packages/core items.ts to meshes in dress.ts).
