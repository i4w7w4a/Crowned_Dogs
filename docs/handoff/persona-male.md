# Persona, male base (from Art's Tripo parts, 2026-10-27 18:45)

Art's male body and bald head, joined the same way as the female (../persona-female/). Replaces the earlier male model (../persona-male/) in the game under the same file name.

- `persona-male.glb` (1.47 MB): the game file. Body 30k + head 15k triangles on the 61-bone UE skeleton, +Y up, faces +Z, feet at y = 0, skull top 1.80 m. 1K WebP textures, meshopt. In the arena as `apps/web/models/persona-male.glb` (our fighter).
- Clips: Art's body came without clips; `idle`, `walk`, `run` are baked from three.js X Bot (../xbot/Xbot.glb), 24 fps, until adapted clips replace them.
- `male.blend`: the assembled full-detail scene (body 55k + head 52k triangles, 4K textures packed in), the source for the game file. Objects `Armature`, `MaleBody`, `MaleHead`.
- Rest pose: Tripo's, upper arms 50° below horizontal (the female's are 61.5°, the earlier male standard was 65.6°). Clips are retargeted in world space, so this does not matter for them; armour fitted on this file must use this rest pose.
- `assemble.py`: `python -I assemble.py -- MaleBody.fbx malehead.fbx OUT.blend [S_B S_H SINK]` with Blender 5.2 as a Python module (`pip install bpy==5.2.3`). Body 1.5618x, head 0.2632x (its collar just wider than the neck), collar slid 25 mm down over the neck (body neck plus collar read too long otherwise), the neck below the collar bent to its outline, head skinned neck_01 to head over the lowest 3 cm.
- `export_game.py`: `python -I export_game.py -- male.blend Xbot.glb raw.glb` (decimate 0.55/0.3, bake idle/walk/run, export), then `gltf-transform resample, dedup, prune, resize 1024, webp --quality 88, meshopt --level medium` (@gltf-transform/cli 4.5.1).
- `pack.py`: packs the textures into a .blend. `render.py`, `look.py`: Cycles check renders; `head-check.jpg` (assembled), `parts.jpg` (Art's two parts as sent).
- Art's originals: the two zips he uploaded (MaleBody, malehead). Never modify them.
