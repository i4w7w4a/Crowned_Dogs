---
name: crowned-dogs-audio
description: Crowned Dogs music and sound rules: fight playlist (Limbo/404 Room/Middle of Nowhere, random), 4-track menu radio quieter, menu pauses fight; Sonniss bundle path + license; audio thread owns it
metadata:
  type: project
  modified: 2026-10-27T13:27:22.435Z
---

Art sent 7 tracks on 2026-10-27 (originals in /mnt/project-files/crowned-dogs/music/, 128k −16 LUFS copies in web-128k/, analysis in ANALYSIS.md).
- **Fight playlist (Art, 2026-10-27 12:08 UTC, replaces the earlier "Limbo only" plan):** The Limbo_3, "demo. 404 Room speed up", "In The Middle of Nowhere", picked at random.
- **Menu / gear radio:** the other 4 (CRAZY_2, FXCK WAG_6, Fog_3, Try To Fake) play as an in-world radio/speaker (diegetic, band-pass + hiss). Art: menu music must be clearly quieter ("сейчас орёт").
- **Opening the menu pauses the fight track**; closing resumes it.
- Earlier (11:59): Limbo_3 is "universal", can play anywhere.

Sounds: Sonniss GDC 2024 Game Audio Bundle (Part 8, 612 WAVs, 29.5 GB) on Blade at C:\Downloads\Sonniss.com - GDC 2024 - Game Audio Bundle (folder access granted; originals read-only). License: royalty-free, commercial use, no attribution, may not sell the sounds as they come, no AI training. It is a sampler (few files per library) with NO real footsteps; good guns (Saiga-12, SVD, SPAS-12, Lynx .50, Glock/AK5/MP5 indoor large hall, AK5 hitting armour plate), melee, explosion, UI, radio interference, ambiences.

Older 20-file arena set (Blade, C:\Users\qtas5\Documents\CrownedDogs\sfx\arena\) is superseded by the full set below. Licence: keep pages holding the files private.

Full set built 2026-10-27 (supersedes the 20-file arena set): repo package packages/audio (branch claude/project-thread-ae1g8u; package is in claude/integration via PR #7; PR #4 now targets claude/integration for follow-ups: MusicDirector.tune, swap cues): 102 SFX + 6 amb loops + 7 tracks as MP3, sound ids in src/bank.ts, cuesForTurn (turn -> timed cues), MusicDirector (rules above), radio filter, Web Audio engine. Rebuild SFX: tools/audio/build_sfx.py with SONNISS_DIR (recipes.py). Mix: fight music -12 dB, radio -17 dB. Private soundboard with every file published (copyable via files {artifact,path}): https://claude.ai/artifact/EVDbTk2jmjGK9QGjT5o7Gi . Footsteps are assembled and still need Art's ear check.

Art's first listen (2026-10-27 12:57 UTC): knife into shield sounded like a heavy sword (now a light tink + scrape + shield crackle); fight music too loud (lowered 4 dB to -12); radio wanted a little reverb (small-room convolver added); salt hall sounded like a cave with water (cave bed and drips removed; now dry hall: wind in broken windows, low hum, roof creaks). Keep these directions; no water/drips in the salt hall.

**Why:** Art's own words; he wants the menu music to feel in-world and not loud.
**How to apply:** the thread "Озвучить бой и меню" owns music and SFX choices and processing; other threads consume its manifest ids. See [[crowned-dogs-arena]], [[art-pc-tools]].
