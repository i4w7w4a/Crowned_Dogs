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
| Knife | ready_knife | attack_knife | contact 0.958 s |

The sniper uses the shotgun clips.

## How to run

`python3 -I scripts/apply_mocap.py -- IN.blend OUT.blend pose pistol:pistol rifle:long shotgun:long knife:knife`

- Male input: `persona-male-art/male.blend`.
- Female input: `persona-female/fem-v3.blend` (the joined neck, head 2 cm lower).

The grenade throw is Art's female Tripo clip; `scripts/retarget_fm.py -- male-anim.blend fem-v3.blend OUT.blend grenade_throw` gives it to the male. In the arena the grenade leaves the hand at the clip's frame 30 (1.208 s) and the clip ends at 1.875 s.

The current game sources are `male-anim-v2.blend` and `fem-anim-v3.blend` here (with the grenade throw; images packed). Since arena v31 the game models are these files dressed in the armour: see ../armor-game/README.md.

Then export the game file:

- `export_male.py` or `export_fem.py` (both drop `ref_*`).
- The female also needs `hairmask.mjs`.
- Then gltf-transform: resample, dedup, prune, resize 1024, webp q88, meshopt medium.

Check sheets live in `check/`. `male-anim.blend` is the male with every take and clip.

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
