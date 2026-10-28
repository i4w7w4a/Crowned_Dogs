---
name: crowned-dogs-concept
description: Core brief for Crowned Dogs game (setting, Persona, combat rules, slots, regions, Telegram) as given by Art on 2026-10-27
metadata:
  type: project
  modified: 2026-10-27T12:20:00.000Z
---

Crowned Dogs: Telegram-based game + rich 3D client for phone and PC, all synced; must also be fully playable through Telegram bot text/buttons. Region/map communities in Telegram where players fight via bot.

Setting: near future, mix 50% military / 25% cyberpunk / 25% post-apocalypse. AI race between states and AI corporations drained minerals and water; water is the most expensive resource (cooling AI datacenters); PMCs fight over reservoirs; heat waves, climate disasters. Battlefields full of wrecked vehicles, drones, destroyed Personas worth salvaging.
Player is an operator under a multi-year contract who neuro-syncs into a humanlike combat robot, the "Persona" (built to look like its operator); feels everything incl. death, then wakes up. Link range is limited, so operators sit near the front.

Contract + player ID (Art 2026-10-27 12:11): the game starts with the player signing a PMC contract; they get a dog tag "K-XXX-XXX" (digits, e.g. K-417-093), which IS the player ID across bot and client. Many operators are convicts who signed from prison and were sent to be operators. Concept doc (Claude Doc "Crowned Dogs — концепт игры", https://claude.ai/artifact/F5ZwmpZkDkQiYptP9DvEgc) has sections Контракт/Жетон/Начало игры.
Flow: sign contract -> get tag -> create character (gender, name, customize face/hair/eyes/tattoos/suit colour) -> 3D main menu: To battle / Gear (fitting room, gear shown on model used in battle) / Sync level (skills, weapon mastery).
Slots: 2 weapon (1h/2h), head, body armour+rig, boots, knife, grenade, suit. Custom chevron on headgear and suit. 4 weapon types (pistol, shotgun, AR, sniper), each with >=5 visible mod slots. All items affect stats.
Battle: pick region -> auto-gather resources -> if other players there, fight. Random 1 of 2 maps, 1 of 4 times of day, 1 of 4 weather (weather odds shifted by region event). Spawn 7 steps apart, ~1 min per turn: 1 attack (head/body/groin/legs) + 2 defence zones + 1 move step; or extra moves instead of attack; knife at 2 cells; grenade instead of shot; retreat too far = flee. Weapon range roles: shotgun close/low crit, pistol close-mid/high crit, AR mid-long/mid crit, sniper long/high crit but must hold still a turn. Weather/time affect hit, turns, crit; events like lightning strike zones.
Start: 2 regions x 2 maps, post-Soviet (ruined panel blocks, factories); first base = ruined salt-mining factory. Visual feel: bodycam in near future; stylish but not realistic.
Combat/damage math to be taken from Art's old project i4w7w4a/4i7-rust (private; cloud session has no access).

Base and battlefield (Art's pick 2026-10-27 18:31): the base is UNDERGROUND, in the old salt mine shafts under the plant (operator sync capsules, Persona hangar); the fights are over the surface halls, which change hands. Why the plant matters: under it are brine wells, and water (the costliest resource) is made from the brine; the Persona link is short-range, so the capsules sit right under the battlefield. Alternatives he did not pick: base in one wing with no-man's halls around; the hall as a training pit.
Base sketch (Art 18:37): salt mine adit with layered salt walls, warm lamps, bunks behind a curtain, workbench, map table with kettle; saved as /mnt/project-files/crowned-dogs/art/style-2/06-base-salt-mine.png and in the concept doc. I suggested adding Personas in cradles (hangar), the operator capsule with the crown and cables, and a brine-water pipe.
