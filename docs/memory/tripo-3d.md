---
name: tripo-3d
description: How to use Tripo for Crowned Dogs 3D: API key location (never copy), cloud blocks API, website route via Remote Control on Blade, costs and quirks
metadata:
  type: reference
  modified: 2026-10-27T10:31:15.377Z
---

2026-10-27: Art gave a Tripo3D API key (tripo3d.ai, API base https://api.tripo3d.ai/v2/openapi) to generate 3D models. The key is the root message of the thread "Подключить Tripo для 3D-моделей" (cmsg_017bZjvwrJsD5nWZ88aiYvFS5jMRg877GEyQEAepHPhnTg). NEVER copy the key into memory, project files or any repo.

- Cloud sessions: api.tripo3d.ai is denied by the network policy (CONNECT 403). The project runs on the built-in cloud environment with no settings; fixing it would need Art to add a cloud environment with that host allowed (Project settings → environment).
- Working route: Remote Control session on Art's PC Blade (folder C:\Quantum River\Kriss) runs the API calls with the key inline (no setx, no files).
- 2026-10-27: key verified from Blade (balance endpoint code 0), but API balance was 0 credits; Art must top up at https://developers.tripo3d.ai/en/pricing ("Buy credits", from $1, 1 credit = $0.01). API credits are separate from the Tripo web app plan.
- Web route (no API): the Remote Control session on Blade has the Claude app's built-in browser (Claude in Chrome not connected). studio.tripo3d.ai works there; Art must sign in himself once, then Claude can generate with his web-plan credits. 2026-10-27: Art signed in there; web account shows 6715 credits, no paid subscription visible; an HD model costs ~55 credits. Art gave standing approval on 2026-10-27 for any needed Tripo credit (and ChatGPT limit) spending, so no per-generation yes is needed.
- The built-in browser cannot upload local files (file picker and clipboard paste fail), so ChatGPT reference images cannot be put into Tripo by Claude; workaround: Tripo's own image generator (GPT Image 2.5) + multiview inside Tripo, or Art drags the files into Tripo's upload window himself.
- Website costs seen 2026-10-27: 3D generation (H3.1 max, ultra mesh, 4K PBR, ~78k polys) 55; Smart UV 20; auto-rig 20 (UE5 Mannequin bone names); one free retry per generation. Tripo Smart UV can return overlapping UV islands (fix by repacking in Blender, geometry untouched) and can hang at "99%" for 40+ min with credits frozen; don't block on it. The rigged GLB download from the website came out broken; use the rigged FBX.
- 2026-10-27 18:35: the web account is on the Pro plan (12000 credits, valid to 2026-11-22), so models are private with commercial rights; balance 5650. Tripo's image generator has a free monthly allowance (100 images), so reference images cost no credits.
- Art prefers Smart Mesh P2 over HD generation (2026-10-27 20:00: «а почему ты не делаешь Smart Mesh P2? да он дороже но там сетка лучше же»). Use it by default for game assets.
- Web plan rights: Free = 200 credits/mo, IP stays with Tripo, attribution needed, limited export; paid (Professional $13 annual / $20 monthly, 3000+ credits) = private models, full commercial use. Use paid or API for real game assets.
- Blender on Blade: C:\Program Files\Blender Foundation\Blender 5.2\blender.exe (not on PATH). CrownedDogs folder has art, concept, research.
- This thread's folder tools (device_stage_files etc.) are unavailable; to show PC images, have the Remote Control session publish a private Artifact page with them.
- Blender MCP on Blade: interactive server (localhost:9876) only works when Blender is open with the addon; the CLI variant fails until BLENDER_PATH is set.

Finished assets: see [[persona-male-base]].

Character pipeline (body and head separately, join in Blender, rig + Smart UV in Tripo, male and female skeletons): see [[crowned-dogs-3d-pipeline]].

See [[art-pc-tools]], [[crowned-dogs-visual]].
