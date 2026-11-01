# HD-причёски (женские)

Процедурный генератор причёсок прядями (Blender 5.2, Hair Curves, Cycles). Делает HD-версии по референсу
`research/hair-ref/hair-concepts-ref.webp` (Арт). Игровые LOD-версии делаются только после того, как Арт одобрит HD.

## Где что лежит (на ПК Арта, не в git)

- `Documents\CrownedDogs\3d\hair-hd\base.blend`: сцена-основа (голова `Head_f1`, тело, баф, свет, камера `Review_camera`).
- `3d\hair-hd\pixie.blend`, `bob.blend`, `braid.blend`: готовые HD-причёски.
- `3d\hair-hd\renders\`: HD-рендеры и листы «референс | рендер»; `renders\wip-shag\` — черновик игровой причёски.
- `3d\hair-hd\scripts\`: копия этих скриптов.

## Скрипты

- `hairgen.py`: библиотека (голова, рост пряди с путевыми точками, дети, пряди-пучки, фризз, рендер видов, лист).
- `pixie2.py` (01 пикси), `bob3.py` (02 боб), `braid.py` (03 коса сбоку у левого уха), `shag.py` (04 игровая «Длинные», черновик).
- `render_fem.py`: рендер текущей игровой причёски Арта из `project-files/crowned-dogs/3d/persona-female/fem.blend` (только чтение).
- `cmp.py`: лист «игровая сверху | новая снизу». `saveonly.py`: сохранить без рендера.
- `game_cards.py`, `pack.py`, `finish.py`: игровые версии (ниже).

Запуск (пример, превью):

```bash
"/c/Program Files/Blender Foundation/Blender 5.2/blender.exe" -b base.blend --python shag.py -- <out_dir> 1500 40000
```

Аргументы: папка, число гайдов, число прядей, виды через запятую, путь для сохранения .blend.
Переменные: `SPP` (сэмплы), `GUIDES_ONLY=1` (только гайды), `HAIRCOL`, `PIECES`, `RESMUL`.

## Состояние на 2026-11-01

- Пикси: Арт одобрил.
- Боб: левая сторона исправлена (за ухом, до бафа). Ждёт ответа Арта.
- Коса: зад переделан, коса сбоку у левого уха (генератор не нужен, придумал сам). Ждёт ответа Арта.
- Игровая «Длинные» (`shag.py`): черновик, Арт попросил отложить. Сейчас форма — гладкий «шлем»;
  нужно: рваные пряди по всей голове, конический затылок без ступеньки, острые кончики, торчащие волоски,
  тонкая рваная чёлка. Сравнение: `previews/shag-wip.jpg` (сверху игра, снизу новая).
- Пикси, боб и коса в игре с 2026-11-01 (см. ниже). Игровая «Длинные» осталась старой.

## Игровые версии (`apps/web/models/hair-female.glb`)

Каждая причёска: пряди-полоски `Hair_<id>_cards` (коса ещё `Hair_braid_plait` и резинка `Hair_braid_tie`)
и тёмная подложка `Hair_<id>_cap`. Пакет висит на кости `head`, как бороды; показ, цвет, скрытие от первого
лица и под закрытым шлемом: `crates/game-wasm/src/browser/look.rs`, список причёсок: `crates/game-client/src/look.rs`.
Одна причёска на все 4 лица: где череп f2–f4 выходит из подложки, подложка и пряди отодвигаются (до 11 мм).

```bash
B="/c/Program Files/Blender Foundation/Blender 5.2/blender.exe"
for s in pixie bob braid; do "$B" -b 3d/hair-hd/$s.blend --python tools/hair-hd/game_cards.py -- $s <tmp>/game_$s.blend x; done
"$B" -b --factory-startup --python tools/hair-hd/pack.py -- <tmp> <полный путь>/tools/hair-hd/hair-strands.png
python -I tools/hair-hd/finish.py output/hair-hd/raw/hair-female.glb
npx @gltf-transform/cli@4.5.1 meshopt output/hair-hd/raw/hair-female.glb apps/web/models/hair-female.glb --quantize-position 16 --quantize-normal 12 --quantize-texcoord 14
```

`hair-strands.png`: текстура прядей старых женских волос (та же, что в persona-female.glb).

`previews/`: уменьшенные листы для быстрого просмотра.
