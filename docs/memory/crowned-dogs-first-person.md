---
name: crowned-dogs-first-person
description: First-person bodycam view on trial in the arena (#fp): how it works, tuning knobs, open issues, plus the run-steadiness and ring fixes of v27
metadata:
  type: project
---

First person (Art 07:30-07:36 asks). Since v30 (integration 32f6b0f, Art 09:02 «добавь в настройки выбор») the menu has «Камера»: «Из-за плеча» / «От первого лица», saved in localStorage cd-camera (main.ts setCamera); #fp still forces it. Default stays over the shoulder. Eyes at 1.65 m under the same painted horizon, enemy depth ×0.88 (frames.ts cal.fp). Only our arms render (rig.ts armsOnly). Since v29 (integration fb16b33, after Art's 08:26 refs of bodycam grips: gun low in the middle aimed at the target, two hands on it) arena/viewmodel.ts places the gun in camera space per stance (HOLD), IK-reaches both arms to it (ik.ts reach), closes fingers (ik.ts Grip), kicks on each shot (Motion.onMark); our figure's materials squeeze clip space towards the lens axis (WIDE 0.62, a wider lens for the arms only). The body sits as a bodycam between the shoulders (VIEW gun spots); knife/grenade still use VIEW spots + clips. Guns are stand-in boxes, so the pistol reads as a dark slab: real models will fix most of it. Knobs for local builds: ?fpview=x,y,z,tilt, ?fpkey=, ?wide=.

Stances (v29, Art 08:26 «криво стоит», «стойку мягче»): ik.ts Feet plants both feet flat at each ready clip's average footprint (toe turn clamped to 40°), Motion.soften keeps 55% of the ready loops' sway, smoothed σ 2.5 frames round the loop. Rings: main.ts placeRings keeps rings above .ctl and caps hit areas. Run: Motion.settle keeps the upper body on its own clip's pelvis turn.

Art 07:32: first person also cuts load; his own fighter is to be seen in the fitting room (gear screen) instead. If #fp becomes the default, tell the UI thread: showOnMe floats over empty ground and our sync-connector light is hidden.

See [[crowned-dogs-arena]], [[persona-mocap-anim]], [[crowned-dogs-combat-ui]].
