# Clean foundations for modular facial hair

The current `Head_m2` and `Head_m4` contain facial hair in their original colour,
normal, ORM maps and small scan seams. This companion removes the baked hairstyle
without modifying `persona-male.glb` or the authored fitting model.

## Rebuild

Run both scripts with Blender 5.2 in background mode, in this order:

1. `bake_masks.py` writes a numerical coverage mask from the original head UVs and
   anatomical coordinates. It does not paint or alter source colour images.
2. `build_clean_skin.py` composes the supplied generated clean-skin candidates
   with the original material in Blender and bakes new maps. It also writes
   `apps/web/models/beards-skin.glb` and neutral front/profile diagnostics under
   the workspace's `output/beards-production/clean-heads` directory.

From the repository root, in PowerShell:

```powershell
& 'C:\Program Files\Blender Foundation\Blender 5.2\blender.exe' -b --python tools/beards/clean-heads/bake_masks.py
& 'C:\Program Files\Blender Foundation\Blender 5.2\blender.exe' -b --python tools/beards/clean-heads/build_clean_skin.py
```

The two `sources/*-shaved-candidate.png` files were created with image generation
from the corresponding original UV albedos. They are inputs, not the final maps.
Blender material composition retains original skin outside the authored beard
mask. Normal data is non-colour: flat tangent normal is `(0.5, 0.5, 1)`, not an
sRGB-encoded equivalent. Generated colour is matched through a smooth spatial
skin-colour field, sampled from untouched skin on the same head.

Generation provenance (the copied inputs above are the reproducible sources):

- m2: `exec-69da9ce7-39e8-4492-bb51-e86d5b44045b.png`
- m4: `exec-b85ee6a2-686a-4667-85f6-30e2dea426f1.png`

Both were produced in image-generation session
`01a188cb-5f95-7600-a44d-f818d2fc32b9`. The earlier m4 candidate with grey patches
was rejected and is not used.

## Companion contract

The GLB is uncompressed and has six mesh nodes:

- `CleanHead_m2`, `CleanHead_m4`: tiny hidden material carriers with the finished
  base-colour, normal and ORM maps.
- `CleanNormals_m2`, `CleanNormals_m4`: original bind-space position keys and
  corrected shading normals.
- `CleanNormalsAuthored_m2`, `CleanNormalsAuthored_m4`: the same lookup in the
  authored static fitting coordinates.

Each normal carrier has `POSITION` containing original physical float32 coordinates,
`NORMAL` containing the corrected normal, and `_CLEAN_POSITION` containing the
bounded target position. All three are in glTF local Y-up coordinates. The
runtime transforms the source's normalized raw vertex positions through its
actual skin bind/rest frame, then matches the physical position keys and copies
target positions and normals back into that original local space. Original
triangle indices, UVs and skinning attributes remain intact.
No carrier is intended to be rendered.

Blender's roundtrip introduced up to 0.746 micrometres of key drift relative to
the source weighted-rest positions. Runtime matching allows 2 micrometres; this
is separate from the 2 millimetre maximum repair displacement.

The scan has disconnected UV patches and some noncoincident patch boundaries.
Mutual nearest boundary pairs avoid collapsing consecutive vertices into one
group. Fine vertices at a coarsely tessellated neighbouring boundary use segment
constraints instead of merging both edge endpoints. Geometry correction is
restricted to beard-bearing skin; the collar, nose and lip edge are protected.
Every final position displacement is bounded below 2 mm, and measured results
are written to `clean-skin-stats.json` beside the diagnostic renders.

The final stitch-only correction measures 1.117 mm maximum for m2 and 0.738 mm
for m4. Protected collar, lip and nose vertices move exactly zero. No positional
Laplacian smoothing is applied. Original Persona model files are unchanged.

The finished maps are 1024 square, matching the source material resolution.
Small pre-existing scan marks outside the corrected beard region remain; this
does not replace or resculpt the character's entire head.

Inspect the neutral material, emission-only albedo and uniform-grey renders
separately. A valid export does not establish visual acceptance.
