# Combat UI: handoff (thread «Переделать интерфейс экрана боя»)

State on 2026-10-28 09:20. All HUD work is merged (last UI commit 069fd2f, in `main` through PR #7). Nothing uncommitted.

## Code

- `apps/web/index.html`, `apps/web/src/style.css`, `apps/web/src/main.ts`.
- `apps/web/src/hud/liquid.ts` (glass that bends the picture in Chromium, CSS var `--lens`), `hud/log.ts` (fight log), `hud/words.ts` (Russian labels).
- Layout follows Art's mock-up of 2026-10-27 18:21: defence bottom left («Защита 2/2»), attack by tapping a ring on the enemy (no «Огонь» button), knife and grenade side by side over «Подтвердить ход», the aimed ring labelled on a short amber line («Корпус · 73%»), log line and menu button under the controls.

## Look (Art's rules)

- Apple-style liquid glass on every button, in the fight and in the menu.
- The same black 60% tint in every state, disabled included. Light 2px blur (a strong blur was «слишком сильный»). Rim lit top-left and bottom-right.
- No outlines. A picked button gets a brighter edge and an inner glow, bold white text.
- Words and icons in buttons are white.
- Defence is 4 glass buttons, each with a small figure lighting its zone (Art disliked a list with ticks or radio circles).
- The menu is a floating glass sheet with glass controls, switches, round slider knobs, «Звуки боя» and «Музыка» sliders, and «Камера».
- Fonts: Inter Tight, Inter, IBM Plex Mono (Barlow and Saira lack Cyrillic).
- While a turn plays out (2.5 s after «Подтвердить ход») the controls and rings hide; the log line and menu stay. The enemy's hit on us floats over our fighter (`showOnMe`, at 30%/60% of the scene).

## Open

- In first person (menu «Камера» → «От первого лица») the `showOnMe` numbers float over empty ground. They need a new spot if Art makes first person the default.
- Movement numbers show the distance after the move. Art said keep them and explain them in the tutorial.

## Checks

- `npm run check` and `npm run web:build`.
- Screenshot harness: project files `crowned-dogs/arena-ui/tools/shot.js` (Playwright, `node shot.js out.png "js" waitMs`, env `VP`, `DSF`, `CLIP`, `SITE=apps/web/dist`). Reference screenshots of every round are in `crowned-dogs/arena-ui/`.
- Never publish the old prototype `crowned-dogs/arena/index.html`.
