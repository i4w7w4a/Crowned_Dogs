---
name: crowned-dogs-handoff
description: 2026-10-28 move of Crowned Dogs to a local Claude Code session on Blade: HANDOFF.md/CLAUDE.md, merge-to-main approval, project files copied to the PC
metadata:
  type: project
---

Art 2026-10-28 09:17: «как треды закончат свои задачи - подготовь всё для переноса. я продолжу в сессии Claude Code». Thread «Подготовить перенос в Claude Code» owns it.

- Art 09:25 (decision card) approved merging everything into main. Done 10:09: PR #7 (claude/integration → main) merged as b63b849, logo PR #8 shows merged with it. main now holds all work; every claude/* branch is inside it (not deleted: ask Art first).
- Handoff docs on branch claude/project-thread-r0cjks (off main): HANDOFF.md (Russian, state + files + open tasks), CLAUDE.md (rules), docs/handoff/ (per-area notes), docs/memory/ (snapshot of this memory), docs/concept.md (export of the concept Claude Doc).
- Project files copied to Blade at C:\Users\qtas5\Documents\CrownedDogs\project-files\ (same layout as /mnt/project-files/), verified by size 10:10 (only web-test/fonts/*.woff2 skipped). Files over 24 MB went as 23 MiB .partNN chunks; Art double-clicks project-files\join-big-files.cmd to join them (checks size, deletes parts). Transfer gotchas: device_commit_files reads at most 25 MB per file and 100 MB per call; /mnt/project-files/uploads/ is not served (404), so stage copies elsewhere under /mnt/project-files/ first. The staging folder _handoff-transfer was deleted after the copy.
- Armour README copied next to Blade 04-armor NOTES.md as README-cloud.md.

See [[crowned-dogs-repo]], [[art-pc-tools]], [[art-preferences]].
