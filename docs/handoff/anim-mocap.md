# Persona animations from Art's reference videos

How it works: MediaPipe Pose (heavy model, python 3.12 venv, mediapipe 0.10.14) turns Art's four videos into landmarks (`pose/*.json`, via `scripts/extract.py`). `scripts/mocap.py` solves those landmarks onto the 61-bone UE skeleton:

- The body, legs and head come from the video.
- For the pistol and the long guns, both arms are re-placed by IK so that both hands hold the gun (`Rig.hold`).
- For the knife, the video's arms are kept and the hand is turned so the blade points at the enemy.
- Recoil kicks are added at the muzzle flashes found by `flash2.py`.

`scripts/apply_mocap.py` writes the whole takes as `ref_*` actions and cuts the game clips:

| Weapon | Ready loop | Attack | Shot or contact at |
|---|---|---|---|
| Pistol | ready_pistol | fire_pistol | shot 0.625 s |
| Rifle | ready_rifle | fire_rifle | shot 0.333 s |
| Shotgun | ready_shotgun | fire_shotgun | shot 0.333 s, then the pump |
| Knife | ready_knife | attack_knife (replaced 2026-11-02, see below) | contact 0.3 s |

The sniper uses the shotgun clips.

## How to run

`python3 -I scripts/apply_mocap.py -- IN.blend OUT.blend pose pistol:pistol rifle:long shotgun:long knife:knife`

- Male input: `persona-male-art/male.blend`.
- Female input: `persona-female/fem-v3.blend` (the joined neck, head 2 cm lower).

The grenade throw is Art's female Tripo clip; `scripts/retarget_fm.py -- male-anim.blend fem-v3.blend OUT.blend grenade_throw` gives it to the male. In the arena the grenade leaves the hand at the clip's frame 30 (1.208 s). Since 2026-11-02 the arena plays only 0.6–1.45 s of it (`from`/`end` in `crates/game-motion/src/timeline.rs`): the T-pose-like stand before and the arms-out stand after looked silly. The knife plays 0.6–1.15 s of `attack_knife` for the same reason (half a second of crouched shuffling before the stab). The gun comes back 0.25 s into the blend to the ready stance.

The current game sources are `male-anim-v2.blend` and `fem-anim-v3.blend` here (with the grenade throw; images packed). Since arena v31 the game models are these files dressed in the armour: see ../armor-game/README.md.

Then export the game file:

- `export_male.py` or `export_fem.py` (both drop `ref_*`).
- The female also needs `hairmask.mjs`.
- Then gltf-transform: resample, dedup, prune, resize 1024, webp q88, meshopt medium.

Check sheets live in `check/`. `male-anim.blend` is the male with every take and clip.

## Knife and grenade from Quaternius (2026-11-02)

Art found the mocap knife stab (a low crouched lunge) and the Tripo grenade throw (a sidearm swing) silly. Both clips in both Persona GLBs now come from Quaternius' **Universal Animation Library 2, Standard** (CC0, quaternius.itch.io, the free 17 MB zip; file `Unreal-Godot/UAL2_Standard.glb`, UE Mannequin bone names):

- `attack_knife` = `Melee_Hook` + `Melee_Hook_Rec`, hips, trunk and legs damped to 0.35: a guard, a short wind-up, a hooking slash at face height (forward at 0.3 s), back to guard. Game cut: 0–0.55 s, then the ready stance blends in.
- `grenade_throw` = `OverhandThrow`, damped to 0.45: the off arm points, the hand goes over the head and lets go at 0.35 s, a follow-through lean. Game cut: 0.03–0.6 s. The library throws with an open hand, so the right fingers are bent 0.7 of the way to `Melee_Hook`'s fist (wrist to middle fingertip 0.13–0.14 m instead of 0.18–0.19), and `HOLDS` in `crates/game-wasm/src/browser/scene_assets.rs` moves the grenade 4.5 cm to the palm side, where those fingers close round it (before, it sat in the plane of the hand and showed through its back).

Recipe, from the repo root with the original GLBs from git:

```
git show HEAD:apps/web/models/persona-male.glb > male-orig.glb
node tools/anim/retarget_ual.mjs male-orig.glb UAL2_Standard.glb apps/web/models/persona-male.glb "attack_knife=Melee_Hook+Melee_Hook_Rec~0.35" "grenade_throw=OverhandThrow~0.45^Melee_Hook:0.7"
```

The same for the female. `name=ClipA+ClipB@from-to~k^Grip:g` joins clips, cuts seconds, keeps `k` of the hips', trunk's and legs' travel (undamped, the library's strikes lunge to the floor and the throw bows to the knees) and bends the right fingers `g` of the way to their pose in `Grip`'s first frame. The tool is described in its header. After a change: `spec` in `crates/game-motion/src/timeline.rs` and the frozen `crates/game-motion/reference/motion.ts` take the new `from`/`mark`/`end`, then `npm run models:ktx2 -w @crowned-dogs/web -- persona-male persona-female`. Rejected from the same pack: `Sword_Regular_A`/`B` (too elaborate, leans far back), undamped `Melee_Hook` (falls to the floor).

## In the game

Merged into `claude/integration` on 2026-10-28 and live in the arena since v25. It contains:

- `arena/motion.ts`: two layers, legs and body, and attacks timed so the shot meets the sound.
- `arena/weapons.ts`: dark stand-in guns in the right hand until the real guns exist.
- `main.ts`: `armBoth()` and `arena.attack(...)`.
- Both new glb files.

`web-test/anim-test3.js` shoots close-ups of the enemy's stances and attacks at set moments of each clip (it stops the turn clock, which otherwise plays a turn by itself, and freezes the mixer for each shot).

## Next

1. Real gun models in place of the stand-in blocks.
2. The viewer page still shows the old female.
