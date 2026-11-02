---
name: crowned-dogs-balance
description: Crowned Dogs combat balance decisions from the duel simulator (HP, movement, shrinking zone, weapon numbers)
metadata:
  type: project
  modified: 2026-10-27T09:09:09.071Z
---

Balance decided 2026-10-27 from a duel simulator (finite-horizon game solver, ~20k duels), recorded in the concept doc https://claude.ai/artifact/F5ZwmpZkDkQiYptP9DvEgc (sections «Правила хода», «Оружие и дистанции»).

- HP 100; damage core = 4i7 formulas (zones, armor curve, shield guard 0.26 on 2 defended zones, knife passes 50%).
- Movement: dash forward up to 3 steps (only 1 defense zone that turn), back up to 2; no dash-back kiting.
- Maps 11–14 steps long; start 7 apart; battlefield shrinks after turn 6 down to 4; turn limit 15 (storm 10); fleeing only via explicit «Уйти из боя» at the edge, keeps 50% loot.
- Weapons: pistol 36 dmg/22% crit; shotgun 42 (58 point-blank)/2% crit; rifle 37/10%; sniper 65/32% crit x2, starts in stance; knife 33 at ≤2 steps; grenade 28 through vest.
- All 4 classes land in a 43–56% average win-rate corridor across weather/maps (41–57% in single conditions); single pairs are lopsided by design: pistol beats shotgun ~70%, rifle beats pistol ~70%, shotgun beats sniper ~70%. Solver output: scratchpad sim/final_results.txt (ephemeral). The heuristic bot in packages/core/src/ai.ts reproduces this pattern (2026-10-27: rifle avg 60%, sniper 39%).

**Why:** Art wanted computed answers for max retreat distance and 1-vs-2 extra moves, and roles by distance.
**How to apply:** Use these numbers as the starting config when implementing the combat server; re-run sims if rules change. See [[crowned-dogs-concept]].

2026-10-27 19:13 Art's rule: after a dash the next turn's attack is 10 points less accurate (DASH_WINDED 0.1 in rules.ts, FighterState.winded, bot counts it; integration 02a5372). Size chosen = dash evade bonus. Bot sims 1000/pair: dashes 1.15 -> 0.87 a fight, post-dash hit 60 -> 51%, averages pistol 52 / shotgun 44 (was 48) / rifle 62 / sniper 45; 0.15 would drop shotgun to 41%. Veteran kits 46-58%. Offered Art a small shotgun damage bump if it feels weak. Quick sim with dash stats: scratchpad dash/sim.ts (ephemeral), or npm run balance / npm run kits.
