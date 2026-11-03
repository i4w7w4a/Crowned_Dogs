# Grid Combat Sandbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a separate dev-only, playable A/B grid-combat sandbox with deterministic Rust rules, hotseat/bot, replay and a square-cell pseudo-3D presentation.

**Architecture:** Pure versioned rules live in `game-core`; a DOM-independent session lives in `game-client`; `game-wasm` exposes a narrow stateful session and owns the separate browser controller. Vite serves `sandbox.html` only in development, while production keeps the existing single game entry.

**Tech Stack:** Rust 2021/2024 workspace, serde/serde_json, wasm-bindgen, TypeScript 5.9, Vite 8, HTML/CSS, Node test runner, existing WASM build hooks.

**Spec:** `docs/superpowers/specs/2026-11-03-grid-combat-sandbox-design.md`

## Global Constraints

- Rules profile is exactly `mvp-0.1`; every numeric rule and scenario is copied from the spec, not from the old duel catalogue.
- Existing PvP, `distance/lane` duel, saves, production routes and published site remain unchanged.
- No new dependencies, backend, network mode, Telegram, persistence, React, Three.js scene, push, PR or deploy.
- Screen cells are visually square and axis-aligned; no diamond/isometric grid or perspective distortion.
- TypeScript is only a browser adapter; rules, session state, sanitized views and playback records belong to Rust.
- `/sandbox.html` is a Vite development page and must not appear in `apps/web/dist`.
- Existing untracked files outside the new sandbox/spec/plan paths are not modified or committed.
- Run all manual game checks without audio; never treat a resized desktop viewport as a physical-phone test.

## Review Focus

- Non-finite, fractional, duplicate or stale JSON values must be rejected before they can index the board; Task 1 adds malformed-input tests and snapshot immutability assertions.
- Enemy orders must never enter the opposing hotseat view, including accessibility labels and tooltips; Tasks 4 and 6 test serialized views and rendered DOM before reveal.
- Resize and device-pixel scaling must preserve square hit targets and select the same logical cell; Task 7 checks coordinates at desktop and both required mobile viewports.
- A missing fighter image must leave a usable CSS fighter rather than block input; Task 7 checks image error fallback and continued match flow.
- Reset/rematch during autoplay must invalidate late timers and must not mutate HP or append events in the new match; Tasks 4 and 6 test generation invalidation and idempotent playback.

---

### Task 1: Versioned Contracts, Scenarios and Strict Validation

**Files:**
- Modify: `crates/game-core/Cargo.toml`
- Modify: `crates/game-core/src/lib.rs`
- Create: `crates/game-core/src/sandbox/mod.rs`
- Create: `crates/game-core/src/sandbox/model.rs`
- Create: `crates/game-core/src/sandbox/config.rs`
- Create: `crates/game-core/tests/sandbox_contracts.rs`

**Interfaces:**
- Produces: `pub fn scenario(mode: Mode, scenario_id: &str, seed: u32) -> Result<Scenario, String>`.
- Produces: `pub fn validate_round(input: &RoundInput) -> Result<(), ValidationError>`.
- Produces: JSON dispatch methods `sandbox.scenario(mode, scenarioId, seed)` and `sandbox.validateRound(input)`.
- Produces: serde types `Scenario`, `Mode`, `Cell`, `Board`, `Fighter`, `WorldSnapshot`, `RoundInput`, `OrderA`, `OrderB`, `RoundResolution`, `ValidationError` with strict JSON tagging and `deny_unknown_fields`.

- [ ] **Step 1: Write failing contract tests**

Add fixtures for all four scenarios and test exact HP/weapons/armor/walls/positions; test spec cases 1, 9 and 25; add fractional coordinates, negative HP, string/null values in numeric fields, duplicate occupancy, duplicate IDs and stale `roundIndex`. Assert errors and byte-identical input JSON after each rejected call.

- [ ] **Step 2: Run the tests and confirm RED**

Run: `cargo test --locked -p game-core --test sandbox_contracts`  
Expected: compilation failure because `game_core::sandbox` does not exist.

- [ ] **Step 3: Implement models, config and validation**

Add workspace `serde` to `game-core`; implement the exact `mvp-0.1` config, scenario constructors, separate A/B order enums and static route/target validation. Register only `sandbox.*` core dispatch names; do not alter existing combat methods.

- [ ] **Step 4: Run the tests and confirm GREEN**

Run: `cargo test --locked -p game-core --test sandbox_contracts`  
Expected: all contract tests pass.

- [ ] **Step 5: Run existing core regression tests and commit**

Run: `cargo test --locked -p game-core`  
Expected: existing and new tests pass.

Commit only Task 1 files: `git add crates/game-core && git commit -m "Add sandbox combat contracts and scenarios"`.

### Task 2: Movement and Atomic Collision Resolution

**Files:**
- Create: `crates/game-core/src/sandbox/movement.rs`
- Create: `crates/game-core/tests/sandbox_movement.rs`
- Modify: `crates/game-core/src/sandbox/mod.rs`

**Interfaces:**
- Consumes: Task 1 `RoundInput`, `WorldSnapshot`, `Cell`, `ValidationError`.
- Produces: `pub(crate) fn resolve_movement(input: &RoundInput) -> Result<MovementResolution, ValidationError>`.
- Produces: ordered `MovementFrame { microstepIndex, before, after, requests, accepted, stops }` and final `Spos`.

- [ ] **Step 1: Write failing movement tests**

Translate spec cases 2–8 exactly. Add two-step success, diagonal corner symmetry, crossing edges and permutation of fighter/order arrays. Assert positions, stopped remainder, accepted count and byte-identical input.

- [ ] **Step 2: Run movement tests and confirm RED**

Run: `cargo test --locked -p game-core --test sandbox_movement`  
Expected: unresolved `resolve_movement` failure.

- [ ] **Step 3: Implement movement and `max-feasible-steps-v1`**

Enumerate at most 16 subsets against full end positions; maximize accepted transitions, then use direction phase/rank and lexicographically maximal inclusion vector. Stop rejected fighters for the rest of the round and keep all geometry independent of fighter IDs.

- [ ] **Step 4: Run movement and full core tests**

Run: `cargo test --locked -p game-core`  
Expected: all tests pass, including spec cases 1–9 and 26–27 coverage applicable to movement.

- [ ] **Step 5: Commit**

`git add crates/game-core && git commit -m "Resolve sandbox movement collisions atomically"`.

### Task 3: LOS, Attacks, Damage Batch and Outcome

**Files:**
- Create: `crates/game-core/src/sandbox/geometry.rs`
- Create: `crates/game-core/src/sandbox/attack.rs`
- Create: `crates/game-core/src/sandbox/resolve.rs`
- Modify: `crates/game-core/src/sandbox/mod.rs`
- Modify: `crates/game-core/src/lib.rs`
- Create: `crates/game-core/tests/sandbox_attacks.rs`
- Create: `crates/game-core/tests/sandbox_determinism.rs`

**Interfaces:**
- Consumes: Task 2 `resolve_movement` and `Spos`.
- Produces: `pub fn resolve_round(input: &RoundInput) -> Result<RoundResolution, ValidationError>`.
- Produces: `sandbox.resolveRound(input)` JSON dispatch and canonical `MovementFrame`, `HitResult`, `BattleEvent`, `ReplayRecord`, `nextSnapshot` and `outcome` fields from the spec.

- [ ] **Step 1: Write failing geometry and attack tests**

Translate spec cases 10–24 and 28 with exact damage and masks. Add zero armor after penetration, zero damage, self-cell A, direct ally rejection B, independent shotgun LOS rays and dead-cell release only after apply.

- [ ] **Step 2: Run attack tests and confirm RED**

Run: `cargo test --locked -p game-core --test sandbox_attacks`  
Expected: unresolved `resolve_round` failure.

- [ ] **Step 3: Implement rational LOS and all attacks**

Use integer/rational segment comparisons without floating-point rounding; walls block corner touch, fighters block only interior intersection. Resolve every attack against one `Spos`, aggregate damage and apply HP once.

- [ ] **Step 4: Implement canonical trace and outcome**

Emit stable stage/event ordering, full damage explanations, win/draw precedence and round-30 limit. Add JSON serialization with deterministic arrays independent of input order and ID names.

- [ ] **Step 5: Run all new and existing core tests**

Run: `cargo test --locked -p game-core`  
Expected: all 28 spec cases and determinism tests pass.

- [ ] **Step 6: Commit**

`git add crates/game-core && git commit -m "Resolve sandbox attacks and simultaneous damage"`.

### Task 4: DOM-Independent Match Session, Bot and Replay

**Files:**
- Modify: `crates/game-client/src/lib.rs`
- Create: `crates/game-client/src/sandbox/mod.rs`
- Create: `crates/game-client/src/sandbox/session.rs`
- Create: `crates/game-client/src/sandbox/bot.rs`
- Create: `crates/game-client/tests/sandbox_session.rs`

**Interfaces:**
- Consumes: `game_core::sandbox::{scenario, resolve_round, RoundInput, RoundResolution}`.
- Produces: `SandboxSession::start(Setup) -> Result<Self, String>`.
- Produces: `view(&self) -> SandboxView`, `edit(&mut self, Edit) -> Result<(), String>`, `ready(&mut self, TeamId) -> Result<(), String>`, `playback(&mut self, PlaybackCommand) -> Result<(), String>`, `rematch(&mut self)`.
- Produces: `generation`, `matchId`, private drafts, public/side-sanitized views, precomputed round resolution, replay records and deterministic bot orders.

- [ ] **Step 1: Write failing session tests**

Cover complete scripted A/B matches, hotseat handoff, Pass, idempotent Ready, edit-after-lock rejection, next-round flow and rematch. Assert enemy orders/guard/trace are absent from serialized pre-reveal views. Assert bot order is computed once, uses the spec zone table and ignores later human edits.

- [ ] **Step 2: Add failing replay and generation tests**

Advance auto/step/skip/repeat without changing HP twice; corrupt replay config/snapshot and assert a clean error with unchanged active match; reset during pending playback and assert stale generation commands are ignored.

- [ ] **Step 3: Run session tests and confirm RED**

Run: `cargo test --locked -p game-client --test sandbox_session`  
Expected: missing `game_client::sandbox` failure.

- [ ] **Step 4: Implement session, bot and replay**

Store all private state in Rust, calculate each round once at common lock and make playback cursor-only. Implement bot shortest-path tie-breaking and the exact zone/guard table from the spec. Preserve complete replay inputs rather than reusing mutable current config.

- [ ] **Step 5: Run client and core regression tests**

Run: `cargo test --locked -p game-client -p game-core`  
Expected: all tests pass.

- [ ] **Step 6: Commit**

`git add crates/game-client && git commit -m "Add private sandbox match sessions and replay"`.

### Task 5: Narrow WASM Session API

**Files:**
- Create: `crates/game-wasm/src/sandbox_session.rs`
- Modify: `crates/game-wasm/src/lib.rs`
- Modify: `packages/runtime/src/index.ts`
- Create: `apps/web/test/sandbox-session-rust.test.ts`

**Interfaces:**
- Produces: exported `RustSandboxSession` with `start(setupJson)`, `view()`, `edit(editJson)`, `ready(team)`, `playback(commandJson)`, `rematch()`; JSON strings cross the boundary and errors throw.
- Consumes: Task 4 `SandboxSession` and its exact view/edit/playback types.

- [ ] **Step 1: Write failing Node/WASM tests**

Start real A/B sessions through the exported WASM class, play to terminal outcomes, inspect privacy, repeat Ready/apply/replay and rematch. Assert exceptions for malformed JSON and unsupported B+shotgun.

- [ ] **Step 2: Build WASM and confirm RED**

Run: `npm run rust:build && node --test apps/web/test/sandbox-session-rust.test.ts`  
Expected: missing export `RustSandboxSession`.

- [ ] **Step 3: Implement the thin wasm-bindgen wrapper**

Delegate all behavior to `SandboxSession`; add no rules in TypeScript or `game-wasm`. Export the class through the existing runtime barrel.

- [ ] **Step 4: Rebuild and confirm GREEN**

Run: `npm run rust:build && node --test apps/web/test/sandbox-session-rust.test.ts`  
Expected: all sandbox WASM tests pass.

- [ ] **Step 5: Commit**

`git add crates/game-wasm packages/runtime/src/index.ts apps/web/test/sandbox-session-rust.test.ts && git commit -m "Expose sandbox sessions through WASM"`.

### Task 6: Dev Page, Planning, Hotseat and Playback Controller

**Files:**
- Create: `apps/web/sandbox.html`
- Create: `apps/web/src/sandbox/main.ts`
- Create: `apps/web/src/sandbox/sandbox.css`
- Create: `crates/game-wasm/src/browser/sandbox.rs`
- Modify: `crates/game-wasm/src/browser/mod.rs`
- Modify: `packages/runtime/src/index.ts`
- Create: `apps/web/test/browser-sandbox-rust.test.ts`

**Interfaces:**
- Consumes: Task 5 `RustSandboxSession` and sanitized `SandboxView` JSON.
- Produces: exported `RustSandbox` browser controller with `mount()` and `unmount()`; DOM IDs prefixed `sb-`; commands only call session APIs.
- Produces: setup, square board, fighter inspector, Inspect/Move/Attack modes, B guard, Ready/handoff, reveal, playback, log, outcome and rematch views.

- [ ] **Step 1: Write failing browser-controller tests**

Use a DOM/fake-clock harness like `browser-battle-rust.test.ts`. Test setup reset, route editing, separate attack clearing, A/B controls, squad Ready, opaque handoff, no enemy data in DOM/accessibility attributes, stage controls, single HP apply and stale timer cancellation.

- [ ] **Step 2: Build and confirm RED**

Run: `npm run rust:build && node --test apps/web/test/browser-sandbox-rust.test.ts`  
Expected: missing `RustSandbox` export.

- [ ] **Step 3: Implement standalone HTML and Rust DOM controller**

Do not import main `app.ts`, old battle controller or old storage. Render all data from the latest sanitized view, use event delegation with cell coordinates and disable editing outside planning. Timer callbacks capture and verify `generation`.

- [ ] **Step 4: Implement readable logs and playback controls**

Render movement/attack/damage events from Rust trace, reveal all orders only after lock, animate positions without deciding outcomes, and keep next-round/rematch transitions idempotent.

- [ ] **Step 5: Run controller and session tests**

Run: `npm run rust:build && node --test apps/web/test/sandbox-session-rust.test.ts apps/web/test/browser-sandbox-rust.test.ts`  
Expected: all tests pass.

- [ ] **Step 6: Commit**

`git add apps/web/sandbox.html apps/web/src/sandbox crates/game-wasm/src/browser/sandbox.rs crates/game-wasm/src/browser/mod.rs packages/runtime/src/index.ts apps/web/test/browser-sandbox-rust.test.ts && git commit -m "Add playable dev-only combat sandbox"`.

### Task 7: Square Pseudo-3D Rendering and Responsive Interaction

**Files:**
- Modify: `apps/web/src/sandbox/sandbox.css`
- Modify: `crates/game-wasm/src/browser/sandbox.rs`
- Create: `apps/web/public/sandbox/fighter.png`
- Modify: `apps/web/test/browser-sandbox-rust.test.ts`

**Interfaces:**
- Consumes: Task 6 board DOM and movement/attack frames.
- Produces: axis-aligned square tiles, elevated CSS walls, fighter sprite/CSS fallback, y-based stacking, path/LOS/shotgun overlays and viewport-independent cell picking.

- [ ] **Step 1: Add failing presentation and coordinate tests**

Assert equal tile width/height and no grid rotation transforms, correct x/y from `getBoundingClientRect` after resize, y-order stacking and CSS fallback after sprite error. Cover desktop, 360×640 and 390×844 harness sizes.

- [ ] **Step 2: Run presentation tests and confirm RED**

Run: `node --test apps/web/test/browser-sandbox-rust.test.ts`  
Expected: missing square/pseudo-3D classes and fallback behavior.

- [ ] **Step 3: Add the supplied fighter and scoped presentation**

Copy the provided transparent fighter as `fighter.png`; draw team rings/HP/IDs independently. Add wall top/side faces, contact shadows, short movement/fire/hit transitions and reduced-motion handling without rotating or skewing the board.

- [ ] **Step 4: Run presentation tests and confirm GREEN**

Run: `node --test apps/web/test/browser-sandbox-rust.test.ts`  
Expected: all controller and presentation tests pass.

- [ ] **Step 5: Commit**

`git add apps/web/src/sandbox apps/web/public/sandbox crates/game-wasm/src/browser/sandbox.rs apps/web/test/browser-sandbox-rust.test.ts && git commit -m "Style sandbox with square pseudo-3D battlefield"`.

### Task 8: Production Isolation, Full Match Acceptance and Documentation

**Files:**
- Modify: `LOCAL_BUILD.md`
- Create: `apps/web/test/sandbox-production-isolation.test.ts`
- Modify: `apps/web/test/browser-sandbox-rust.test.ts`

**Interfaces:**
- Consumes: all previous public APIs and the Vite development page.
- Produces: documented `npm run web -- --host 127.0.0.1` plus `/sandbox.html`, explicit non-production scope and verification evidence.

- [ ] **Step 1: Add failing isolation test**

Inspect Vite config inputs and build output to assert the existing production entry remains unchanged and no `sandbox.html` or sandbox entry appears. Test that the dev HTML never imports `mount_application`, old battle or old storage.

- [ ] **Step 2: Run isolation test and confirm RED/GREEN by current behavior**

Run: `node --test apps/web/test/sandbox-production-isolation.test.ts`  
Expected: initial failure if the dev page was accidentally added to production; otherwise preserve a passing guard test.

- [ ] **Step 3: Run all standard automated checks**

Run: `cargo test --locked --release -p game-core -p game-client && npm run check && npm run web:build`  
Expected: all commands exit 0; inspect `apps/web/dist` for sandbox absence.

- [ ] **Step 4: Exercise real browser matches**

Start Vite on `127.0.0.1` using its printed port. In Kimi Code Desktop browser, play full A/B `compare-duel` and `compare-squad`, shotgun A, bot, hotseat, collision/LOS/guard/mutual-kill cases, pause/repeat/skip/reset/rematch and image fallback. Check Network, console, square cells and desktop/360×640/390×844 layouts.

- [ ] **Step 5: Smoke the existing game and check workspace hygiene**

Open existing `/?muted=1`, start the old battle and verify its menu/round still work. Run `git status --short`; remove only generated logs/screenshots not needed for the report, leaving pre-existing untracked files untouched.

- [ ] **Step 6: Document launch and commit final verified changes**

Add the exact dev-only launch/URL and verification boundary to `LOCAL_BUILD.md`. Commit only relevant files: `git add LOCAL_BUILD.md apps/web/test && git commit -m "Document and verify isolated combat sandbox"`.

- [ ] **Step 7: Report the ready prototype**

Return the actual `/sandbox.html` URL, implemented A/B scenarios, checks with results and any remaining limitations. Do not push, create a PR, deploy or claim physical-device coverage.
