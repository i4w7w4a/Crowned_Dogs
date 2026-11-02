# Weapons for 3D: one base model per class, five sockets

Source of truth for names and stats: `packages/core/src/items.ts` (PR #3). Flat silhouettes of every part: `packages/icons/svg/<id>.svg` (same ids as below), and the live gear screen https://claude.ai/artifact/GS1Y2g5bEKbGkUHHb74PFi shows each part on the gun.

## How the gun is built

- One **body** mesh per class (receiver + barrel + grip; no stock, no sight, no magazine, no muzzle device, no handguard accessory).
- Five **sockets** (empties in Blender, named `socket_optic`, `socket_muzzle`, `socket_rail`, `socket_mag`, `socket_stock`). Each part is a separate mesh with its origin at its mounting point, snapped to the socket.
- The factory part (id ends in `-stock`) is the first option in each socket. Decided 2026-10-27 on «Варяг»: the factory iron sights and factory handguard stay IN the body mesh, so the factory optic and rail parts (`*-optic-stock`, `*-rail-stock`) have no mesh of their own and those sockets are empty until a mod is fitted. The factory stock, magazine and muzzle device are separate meshes, cut from the generated gun in Blender (cleaner than Tripo's own segmentation).
- Ammo variants (armour-piercing, hollow-point, slugs, buckshot) **share the magazine mesh** of their class; mark them with a coloured band or decal, not a new model.
- Style: Art's style 2 (2026-10-27, refs in `art/style-2/`): photoreal, cold overcast grey, salt dust and wet grime on everything. Guns are black and dark-grey worn steel and polymer, as in the refs, with no bright colours. Five paint jobs are material swaps, not new meshes: factory, salt, olive, rust, pixel camo. Salt dust in the crevices suits every paint job.

## Sizes and socket positions

Millimetres, side view, gun pointing right. "From rear" = from the back end of the body (without stock). "From top" = down from the top edge of the body. Rifles, shotgun and sniper share one part scale; pistol parts are their own scale.

### Pistol «Пустельга» — body 170 × 125 mm
| socket | from rear | from top | parts |
|---|---|---|---|
| optic | 75 | 5 | pistol-optic-stock (iron sights), optic-tritium, optic-kp1 (mini red dot) |
| muzzle | 170 | 19 | pistol-muzzle-stock (none), muzzle-pb9 (suppressor), muzzle-klyk (compensator) |
| rail (under barrel) | 135 | 43 | pistol-rail-stock (empty rail), rail-luch (flashlight), rail-lcu (laser) |
| mag (in grip) | 27 | 123 | pistol-mag-stock (15 rd), mag-pistol-long (extended); ammo: mag-pistol-hp, mag-pistol-ap |
| stock (grip back) | 11 | 79 | pistol-stock-stock (factory grip), stock-pistol-hvat (grip panels), stock-pistol-upor (holster stock) |

### Shotgun «Таран» — body 690 × 180 mm
| socket | from rear | from top | parts |
|---|---|---|---|
| optic | 128 | 0 | shotgun-optic-stock (bead), optic-blik (red dot), optic-gran (2× prism) |
| muzzle | 690 | 39 | shotgun-muzzle-stock (cylinder), muzzle-choke, muzzle-vata (suppressor) |
| rail (forend) | 375 | 90 | shotgun-rail-stock (forend), rail-luch (flashlight), rail-kulak (vertical grip) |
| mag | 265 | 85 | shotgun-mag-stock (tube, 5), mag-shotgun-box (box, 8); ammo: mag-shotgun-slug, mag-shotgun-buck |
| stock | 0 | 40 | shotgun-stock-stock, stock-shotgun-fold (folding), stock-shotgun-otdacha (recoil pad) |

### Assault rifle «Варяг» — body 620 × 175 mm
| socket | from rear | from top | parts |
|---|---|---|---|
| optic | 150 | 0 | rifle-optic-stock (iron sights), optic-blik, optic-gran, optic-sokol (4× scope), optic-znoy (thermal) |
| muzzle | 620 | 43 | rifle-muzzle-stock (flash hider), muzzle-klyk, muzzle-tishina (suppressor) |
| rail (handguard) | 405 | 75 | rifle-rail-stock, rail-luch, rail-kulak, rail-koster (underbarrel grenade launcher) |
| mag | 310 | 78 | rifle-mag-stock (30), mag-rifle-twin (taped pair), mag-rifle-drum (75 drum); ammo: mag-rifle-ap |
| stock | 0 | 43 | rifle-stock-stock (folding), stock-rifle-opora (fixed), stock-rifle-tesno (short) |

### Sniper rifle «Беркут» — body 805 × 165 mm
| socket | from rear | from top | parts |
|---|---|---|---|
| optic | 150 | 0 | sniper-optic-stock (PSO-style 4×), optic-oryol (12× scope), optic-blik-b (offset red dot), optic-znoy (thermal) |
| muzzle | 805 | 40 | sniper-muzzle-stock (muzzle brake), muzzle-tishina-s (suppressor), muzzle-molot (brake-compensator) |
| rail (stock fore-end) | 450 | 70 | sniper-rail-stock, rail-kulak, rail-upor (bipod) |
| mag | 223 | 70 | sniper-mag-stock (5), mag-sniper-10 (10); ammo: mag-sniper-ap |
| stock | 0 | 40 | sniper-stock-stock, stock-sniper-opora (adjustable), stock-sniper-izgotovka (cheek-rest) |

Shared parts (optic-blik, optic-gran, optic-znoy, muzzle-klyk, rail-luch, rail-kulak) are one mesh used on several guns.

## Mesh count

4 bodies + 59 part ids (20 of them factory parts `<class>-<slot>-stock`). Six are ammo variants without their own mesh, and the 8 factory optic and rail parts live in the body, so the job is 4 bodies + 45 part meshes. Reasonable order: the four bodies with factory parts first (enough for fights), then optics and muzzles (most visible), then the rest.
