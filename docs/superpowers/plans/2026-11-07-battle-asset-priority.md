# Battle asset priority implementation plan

Spec: GitHub Issues #15 and #39 (`i4w7w4a/Crowned_Dogs`). Scope: prioritize the current fighters' models and weapons without changing combat rules or presentation.

## Global Constraints

- Work only in `art/battle-asset-priority`, based on `origin/main` commit `a7bae51`.
- Rust/WASM remains the source of truth for loading orchestration; TypeScript only supplies browser capabilities.
- Preserve `arena.load()` compatibility when no priority is supplied: QA and existing callers still wait for every gun.
- Production waits for both Persona models, both fighters' current and secondary weapon classes, knife and grenade. Other classes join the shared gun collection after battle readiness.
- Preserve the existing `saveData` guard in base prefetch. After battle readiness, load the remaining classes needed by later fights; do not claim a measured speedup without comparable browser measurements.
- No merge, deployment, shared `main` push, unrelated assets or game-rule changes.
- Use TDD, run `npm run check` and `npm run web:build`, and record real results.

### Task 1: Priority loading and deferred secondary gun classes

1. Add regression tests for `sceneLoadGuns` selection, production priority arguments and deferred loading after readiness. Run them against the current WASM and observe the intended failures.
   Expected: current code waits for all gun classes and does not pass or schedule a priority list.
2. Add an optional priority argument to gun and arena loading. With no argument preserve existing behavior; with an argument load only selected valid classes and retain the shared collection.
   Expected: targeted unit tests prove selected loads and compatibility.
3. Derive production priorities from both fighters' current and secondary kits, always including knife and grenade. Trigger remaining-class loading only after the battle is ready so later fights retain real models; preserve the existing base-prefetch `saveData` guard.
   Expected: integration tests prove order, timing, deduplication and shared-collection completion.
4. Run targeted tests, `npm run check`, and `npm run web:build`. Review the whole diff, commit source and tests together, and create one Russian PR closing #39 without merging or deploying.
   Expected: all checks pass, only task-related files change, PR remains open for the owner.
