---
name: persona-mocap-anim
description: Combat animations from Art's 4 reference videos (mocap onto both Personas): state when paused 2026-10-27 20:12, where files are, next steps
metadata:
  type: project
  modified: 2026-10-27T20:12:58.208Z
---

Art asked (18:10/18:13) to adapt the Persona animations using his 4 reference videos. Paused at 20:12 on his order «все треды на паузу» (laptop shutdown).

Done (core thread):
- Mocap pipeline: MediaPipe landmarks → bpy solver.
  - Body and legs come from the video.
  - Both hands hold the gun by IK.
  - Recoil comes at the muzzle-flash frames.
- Game clips made per weapon:
  - ready_* loops.
  - fire_pistol, fire_rifle, fire_shotgun, attack_knife.
  - The sniper uses the shotgun clips.
- New male and female glbs carry these clips. The female keeps Art's 5 Tripo clips (incl. grenade_throw).
- Arena code:
  - arena/motion.ts: legs and body layers; attacks timed to the sound.
  - arena/weapons.ts: stand-in dark box guns in hand_r until the real guns exist.
  - main.ts: armBoth + arena.attack.

MERGED into integration 2026-10-28 07:30 (3089cc5) and published as arena v25. Test with web-test/anim-test3.js (stops the turn clock and freezes the mixer per shot; the old test's 'rest pose' glitch was the turn timer auto-playing a turn).

Files: /mnt/project-files/crowned-dogs/3d/anim-mocap/ holds scripts/, pose/, check/, web-test/, male-anim.blend and a README with the clip table and commands.

Grenade done 10-28: Art's female Tripo grenade_throw retargeted to the male (anim/scripts/retarget_fm.py, world-space like persona-male/retarget.py); motion.ts grenade mark 1.208 s (clip frame 30 = release, hand empties), end 1.875 s (the clip's second swing skipped), stand-in grenade in weapons.ts.
Still open: the viewer page (GeU1c51BY963iEvWFM1SYF) still shows the old female; real gun models.

Morning task DONE 10-28 06:50 (arena v22): camera moved slightly back. Was: try moving the camera slightly back ("возможно отдалить камеру чуть назад"); core thread owns it (frames.ts cal: fov 55, camH 2.0, our figure me {x -0.15, y 0.17, z -0.66}). Hiding the buttons while animations play goes to the UI thread.

UI rounds efc92de + 0d5a149 (controls hide during the turn) merged and published in v22.

See [[persona-female-base]], [[persona-male-art]], [[crowned-dogs-arena]].
