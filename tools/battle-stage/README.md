# Battle camera projection stage

The `hall1` battle background is now projected onto a reconstructed stage. The architecture has its own clean plate; machinery has closed traced volumes, conveyors have depth along their length, and sacks have curved fronts. The camera projector stays fixed while our player's travel moves the environment. Enemy-only movement never changes the camera position.

The reconstruction is approximate and designed for longitudinal battle movement, not a free 360-degree camera. The source photograph cannot supply unseen equipment details; narrow side surfaces use edge projection. `hall2` retains its own painted background.

## Inspect

- Unified local battle: `http://127.0.0.1:5197/?muted=1#battle` after starting `release-local/Crowned-Dogs/CrownedDogs-Web.exe --no-open`.
- The standalone preview remains in the earlier `codex/battle-camera-stage` development branch; the combined release uses the actual battle renderer.
- In the default first-person battle, the existing calibration gives a step of 0.469491 m and a three-step rush of 1.318338 m from range 7. The camera follows that real displacement; focal length stays constant.
- `?muted=1&motionDebug=1` exposes read-only `data-stage-travel`, `data-enemy-depth`, `data-projected-stage` and `data-walking` on the battle canvas for QA. No audio context is started in muted mode.

## Rebuild the Blender source

Run from this worktree with Blender 5.2:

```powershell
& 'C:\Program Files\Blender Foundation\Blender 5.2\blender.exe' --background --factory-startup --python tools/battle-stage/build_stage.py
```

Outputs are written to ignored `release-local/battle-stage/`, including the packed `.blend` and three validation renders. The shared geometry is exported to `apps/web/public/img/battle-stage/manifest.json`. Use `-- --no-render` after the Python path to export only. Optional `--source-root`, `--output-root`, and `--web-root` arguments override paths.

In Blender, scene 1 has the fixed `PROJECTOR` camera and animated `VIEW` camera. Frame 1 is the original view; frame 97 is 2 m forward; frame 193 returns to the origin. Scene 2 shows the equipment volumes. The image plates and generated floor/wall/roof materials are packed into the file. Outside the original photographic frustum, these world-space materials cover the extended geometry, including 40 m behind the source camera. No mirrored room-photo tiles or stretched screen-space border are used. No depth-map displacement is used. Runtime inverse-depth rendering is used only for occlusion, haze and depth of field. The floor is excluded from the figure occlusion pass so the existing below-floor reflection remains visible; it still participates in color and depth-of-field rendering.

## Parallel work

The original projection prototype is preserved at `eefe46d` on `codex/battle-camera-stage`. The unified implementation is based on the completed Rust migration `d6ea6da` and lives on `codex/unified-camera-stage`. Stage orchestration, movement and render-graph hooks run through Rust/WASM; the old TypeScript arena and lens are not restored. Both Windows launchers and the browser use the same packaged game directory.
