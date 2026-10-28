---
name: tripo-skeleton
description: Facts about Tripo auto-rig output (61 UE-Mannequin bones, 1.0 m normalised height, arms-down A rest pose) from Art's old Guardian/Heavy Guard models
metadata:
  type: reference
---

Read on 2026-10-27 from Art's old Tripo models, originals in C:\Users\qtas5\Downloads\Guardian.zip and Heavy+Guard.zip (never modify; work on copies). Each holds a binary FBX plus base colour, normal, metallic and roughness JPGs; one "Armature" plus one skinned mesh (Guardian 48,342 tris, Heavy 46,195).

- **Bones:** 61, identical names and hierarchy in both, UE Mannequin style, no face bones: root, pelvis, spine_01..03, neck_01, head; per side clavicle, upperarm, upperarm_twist_01, lowerarm, lowerarm_twist_01, hand, index/middle/pinky/ring/thumb _01..03; per leg thigh, thigh_twist_01, calf, calf_twist_01, foot, ball.
- **Scale:** Tripo normalises every model to exactly 1.0 unit tall (mesh Z 0..1, armature scale 1). Scale to ~1.8 m and apply before binding; exported root must have scale 1 (a 100x armature scale once stretched bodies after retargeting in Unreal).
- **Rest pose:** relaxed arms-down A (upper arms 56-58 deg below horizontal), follows the generated pose, so all parts sharing a skeleton must be generated in the same pose.
- Bone names match across models but positions differ: animations transfer by name, swappable parts need one shared rest skeleton per sex.

See [[crowned-dogs-3d-pipeline]], [[tripo-3d]].
