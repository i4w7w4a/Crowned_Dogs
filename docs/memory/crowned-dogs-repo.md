---
name: crowned-dogs-repo
description: Crowned Dogs GitHub repo facts and merge gotchas (stacked PRs landed in an old branch; prefer base main)
metadata:
  type: project
  modified: 2026-10-27T13:57:04.935Z
---

Repo code-of-stalker/Crowned-Dogs (private; GraphQL is blocked here, so use `gh api` REST). Default branch main, `delete_branch_on_merge` is false. Art merges PRs himself, fast, in the order he sees them.

2026-10-27: PRs #2, #3 and #4 were stacked on PR #1's branch. Art merged #1 and, ten seconds later, #2 without deleting #1's branch, so #2 landed in claude/combat-core-telegram-bot instead of main. PR #5 was opened to carry that branch into main.

Merge plan (owned by the core/bot/3D thread, 2026-10-27 13:10). Art said he can't do merges or git for now, so don't ask; threads keep their PRs open and conflict-free, and progress is shown through artifacts. Stack: main ← #5 (branch claude/combat-core-telegram-bot = #1 + #2) ← #3 weapons (claude/project-thread-3rmjpo) ← #6 sidearm swap/veteran kits (claude/sidearm-swap); #4 audio (claude/project-thread-ae1g8u) also sits on #5's branch. Order later: #5 (then delete its branch), #3, #4, #6 (retarget to main after #3).

Update 13:30: one-button plan. Branch claude/integration (PR #7 → main) merges #5, #3, #4 and #6. Threads push to their own branches and ping the core thread, which merges them into claude/integration. New cross-thread work branches from claude/integration, with PRs against it. When Art is back: merge #7 only, and the others show as merged.

State 17:41: claude/integration is 09e174c (adds the UI thread's second HUD round; before that ecbfd94 added claude/style2-hall: grey hall + lens, and the UI thread's HUD branch claude/project-thread-kxnjvc). Earlier at 14:40 it held #5, #3, #4, #6, #8, #9 (web client) and #10 (male Persona model, merged). Nothing else is open into integration. 16:22 Art merged #3 into claude/combat-core-telegram-bot himself (so #5 now carries it); integration already had it. Merge a thread's branch into integration only after typecheck, tests and web:build pass locally.

2026-10-28 06:59: Art merged #5 into main and #6 into claude/combat-core-telegram-bot (old base again; harmless, integration already has #6). #7 (claude/integration → main) is the one PR left, clean and mergeable at 0c2e97d.

2026-10-28 10:09: PR #7 merged into main (b63b849) with Art's approval; #8 shows merged too. main holds everything, no open PRs. From now on: branch off main, PR into main, ask Art before merging.

**Why:** stacked PRs only retarget to main when the base branch is deleted after its merge, and Art doesn't do that.
**How to apply:** open new PRs against main whenever possible. If stacking can't be avoided, retarget the PR to main yourself (REST `PATCH /pulls/N` with `base=main`) as soon as its base merges, before Art gets to it. Package.json scripts, .gitignore, README and package-lock are the usual conflict spots between threads. See [[crowned-dogs-arena]].
