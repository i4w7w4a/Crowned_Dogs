# Armour, kit A, on both Personas

Thread «Собрать броню и надеть на Персон» (Art, 2026-10-27 18:47 → done 2026-10-28 09:00 UTC).
Art's ask: «нужно собрать броню и надеть на персон». Status: **done**, handed to the arena thread
«Разработать бой, бота и арену» for the game export (resize textures, webp/meshopt) and putting it on the fighters.
Nothing from this work is committed to the repo.

Pages for Art: result https://claude.ai/artifact/4tGZxpLQDyaZS3NH38Uj8k, sketches https://claude.ai/artifact/1L87FZqSTy6mZupJHVZeZZ,
Tripo downloads (base64 pieces + 3D viewer, made on Blade) https://claude.ai/artifact/EshEaixes3U7NduFejaLP6.

## The kit (Art picked kit A on 2026-10-27 19:52)
- **Suit** (catalogue slot `suit`): grey-green camo combat shirt and trousers, elbow and knee pads, gloves, tall boots (all one mesh).
- **Vest** (slot `body`, «Плита-2»): plate carrier, 3 mag pouches, side utility pouch, back grab handle.
- **Helmet** (slot `head`, «Каска К-6»): high-cut, camo cover, rails, empty NVG mount, headset, cyan strobe at the back.
Kit B (round steel helmet, boxy vest, rolled sleeves) was dropped.

## Files
- `fitted/male-armor.blend` (111 MB), `fitted/female-armor.blend` (238 MB, mostly her hair strands): the Persona file
  (`../persona-male-art/male.blend`, `../persona-female/fem.blend`, same Armature, bones and rest pose) plus three skinned meshes
  `Suit`, `Vest`, `Helmet` parented to the Armature (Armature modifier, props `garment=True`, `slot`). Max 4 bone influences.
  Helmet is 100 % on bone `head`.
  - Body masks: `MaleBody`/`FemBody` vertex groups `hide_Suit` (everything below the collar) and `hide_Vest`. Hide those faces
    when the suit is worn, otherwise the body pokes through in poses.
  - Female hair under the helmet: `FemHairCards_Helmet`, `FemHairCap_Helmet` (prop `hair_for=Helmet`), use instead of
    `FemHairCards`/`FemHairCap` when the helmet is on.
  - Textures: 4096 base colour + normal + metal/roughness per garment, packed in the .blend (rm PNGs 17–25 MB each).
- `fitted/check/`: check renders (`*-armor.jpg` rest + poses + crouch, `*-close.jpg` close-ups).
- `tripo/`: the four Tripo downloads, normalised to 1.0 m: `suit-male.glb`, `suit-female.glb`, `vest.glb`, `helmet.glb`
  (Smart Mesh P2 quad 25k + texture 4K + PBR), `tripo-log.md` (task ids, credits), `preview/`.
- `from-pc/img/`: the kit A sketches (A-1..A-4) and the part images Tripo was made from (`parts/A/`).
- `ref-input/`: Persona reference renders and the two ChatGPT requests (kit A, kit B).
- `NOTES-blade.md`: copy of the Blade NOTES (sketch and Tripo steps in detail).

## Rebuild
Blender 5.2 as a Python module (`pip install bpy==5.2.3 scipy pillow`), about 4 minutes:

    sh run_all.sh [OUTDIR]        # default OUTDIR = fitted/

- `fit_garment.py`: fits one Tripo GLB onto a Persona .blend (`--help` is the docstring at the top: modes body/rigid,
  every key). Steps: weld the GLB's UV-seam splits; scale and place (per-axis multi-start refine for vest/helmet);
  swing arm and leg bones into sleeves; cut the generated head by colour; copy skin weights from the body; un-skin to
  rest pose; push out of the body (soft push keeps plates whole); hide groups; hair squeeze under a helmet.
- `render_fit.py IN.blend OUT.jpg hide=1 pose2=1`: rest + test poses sheet. `render_close.py IN.blend OUT.jpg z= scale=`:
  close-up sheet (front, side, back, top, above). `render_ref.py`: Persona reference renders for sketches.
- Per-piece settings that matter (all in run_all.sh): male vest `height=0.39 top_z=1.52`, female vest `height=0.35 top_z=1.40`
  (measured on the sketches), helmet `width=0.236` / `0.238`, female suit `neck_cut=1.472 head_max=0.45`.

## Tripo route (for more gear)
Web studio only (never the API key, which lives in the thread «Подключить Tripo для 3D-моделей»), run on Art's PC «Blade»:
upload tool `C:\Users\qtas5\Documents\CrownedDogs\research\tripo-upload\`, settings «Умная сетка» P2, quad 25000, Приватный,
multi-view, then «Текстура» 4K and «PBR», export GLB 4k. 125 credits per part; 4710 credits left on 2026-10-28.
Art consented in the thread to spend credits on armour. The built-in browser of a session can lose the Tripo login:
Art logs in again himself.

## Possible next steps (not started)
- Boots as a separate piece (slot `boots`) if the catalogue needs them apart from the suit.
- Kit B or more catalogue wearables with the same pipeline.
- Lighter textures (2K, rm as JPEG) if the arena thread asks.
