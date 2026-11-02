# Varyag fitting preview: asset provenance

This is the existing, user-approved P2 Varyag rifle asset, reused for the fitting room. No new model or textures were generated.

- Source repository commit: `4473d7f1e25430d7af7843cc1d4552b8e24dc3b8` (`Put the real Varyag rifle model in the game`).
- Source file: `C:\Users\qtas5\Documents\CrownedDogs\wt-varyag\apps\web\models\varyag.glb`.
- Original art export: `C:\Users\qtas5\Documents\CrownedDogs\3d\03-weapons\web\varyag_p2.glb` (verified byte-identical).
- GLB size: 3,063,420 bytes.
- GLB SHA-256: `821d39993def1567392b719a9ff4bd8352f3f0256f2c415e4d51e044c7c768ba`.
- New runtime file: `apps/web/public/preview/varyag.json`.

The runtime file wraps those exact GLB bytes as `{ "data": "<base64>" }`, matching `apps/web/src/arena/load-model.ts`. Decode the `data` value to verify the GLB hash above. The GLB includes all nine WebP textures; it has no external image or buffer dependencies.

## Approval and production record

The asset belongs to this project and was generated through the user's Tripo Pro account, then assembled and finished in Blender. The existing project handoff records that Art selected P2 on 2026-10-28 and approved further wear detailing with ChatGPT textures:

- External worktree `HANDOFF.md`, line 141: P2 selection, wear textures and completed game integration.
- Art workspace `3d/03-weapons/NOTES.md`, lines 289-304: P2 choice, texture-detail workflow and final 3.06 MB export.
- Source commit `4473d7f` integrates this asset in combat, reflections and the earlier fitting view.

These are existing project provenance records. No separate asset licence file accompanied this export.

## Reuse boundary

Only a new preview wrapper and this provenance note were added. The external source files, the protected `apps/web/models/` files, audio assets and core combat rules were not modified.

The model is in metres with its origin at the grip: +X along the barrel, +Y up, +Z right. Factory parts are children of named socket nodes. The two optional meshes, `optic_sokol` and `muzzle_tishina`, must be placed on their corresponding sockets and shown according to the equipped mod IDs.
