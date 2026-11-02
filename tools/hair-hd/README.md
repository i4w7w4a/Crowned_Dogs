# HD-причёски

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
- Игровая «Длинные» (`shag.py`), v2 от 2026-11-02 по замечаниям Арта: рваные пряди с острыми кончиками
  (края каждой пряди короче середины), затылок клином без ступеньки (сзади пряди сходятся к середине,
  верхний слой сзади не короче нижнего), тонкая рваная чёлка, торчащие волоски. HD `3d\hair-hd\shag.blend`,
  сравнение `previews/shag-v2.jpg` (сверху игра, снизу новая; старый черновик `previews/shag-wip.jpg`). Арт отклонил
  v2 (2026-11-02): в игре остаётся старая «Длинные», переделка закрыта.
- Пикси, боб и коса в игре с 2026-11-01 (см. ниже). Игровая «Длинные» осталась старой.

## Мужские (2026-11-02)

Референс `research/hair-ref/hair-male-ref.webp`, сцена `3d\hair-hd\base_male.blend` (головы `Head_m1`–`Head_m4`),
запуск с `PERSONA=male`. Скрипты: `male.py` (линия роста, кадры референса), `male_short.py` (m1 короткая),
`male_slick.py` (m2 зачёс назад), `male_medium.py` (m3 средняя). HD: `3d\hair-hd\male_*.blend`, лист для Арта
`research/hair-ref/male-hd-v1.jpg`. Дреды (m4) Арт отменил; вместо них низкий хвост по его эскизу: `male_ponytail.py`
(хвост с резинкой на затылке, свисает до лопаток), HD `3d\hair-hd\male_ponytail.blend`, лист
`research/hair-ref/male-ponytail-v1.jpg`, Арт одобрил 2026-11-02.

В игре они называются `crop` «Короткая», `slick` «Зачёс назад», `medium` «Средняя», `ponytail` «Низкий хвост»
(пакет `hair-male.glb`), «Ёжик» остаётся. У короткой бока «под машинку» — не полоски, а слой `Hair_crop_fade` с текстурой щетины
`hair-fade.png` (`fade_texture.py`): к уху и шее реже, наверху сплошная.
Хвост: полоски по голове `Hair_ponytail_cards`, сам хвост `Hair_ponytail_tail`, чёрная резинка `Hair_ponytail_tie`.
В HD есть пряди сквозь череп и свободные у висков: в игровой версии их нет (полосками они висели на лбу).
У лба в центре HD-волосы расходятся «звездой».

Линия роста у лба зачёса и хвоста (2 ноября 2026, раньше там был слой `_fade` из штрихов, и концы полосок
стояли ступеньками):
- полоски, чей корень лежит открыто спереди (никакая другая полоска не проходит над ним ближе 6 мм), а у хвоста
  все полоски передней половины головы, проявляются от корня: альфа в цвете вершин (атрибут `Color`), 0.1 у корня
  и полная через 14 мм, на этом участке ряды вершин через 2 мм. Текстура прядей почти без полутонов, поэтому шейдер
  полосок в `look.rs` (`hair_hook`) начинает каждую прядь текстуры (столбец u) в своей точке этого участка, и край
  рассыпается на волоски, а не обрезается одной линией;
- подложка `_cap` не заходит под первые 12 мм таких полосок (ближе 7 мм), иначе её плоский край просвечивал;
- у хвоста полоски передней половины, идущие поперёк соседей больше чем на 14°, выброшены: у висков и у пробора
  они лежали рублеными кусками поперёк остальных;
- на голове под ними нарисована сама линия роста, маска `hairline-<style>.png` (`scalp_mask.py`, по корням из
  `game_<style>_roots.npz`, которые пишет `game_cards.py`): 0.5 — ничего, выше — цвет волос, сплошной под
  полосками и редеющий штрихами по направлению зачёса чуть дальше последних корней; ниже — запечённая щетина скана
  на лбу перед линией, она возвращается к коже лица. Нарисованные волосы матовые (блик кожи на них гасится),
  иначе тёмно-коричневое светилось серым пятном.

## Игровые версии (`apps/web/models/hair-female.glb`, `hair-male.glb`)

Каждая причёска: пряди-полоски `Hair_<id>_cards` (коса ещё `Hair_braid_plait` и резинка `Hair_braid_tie`)
и тёмная подложка `Hair_<id>_cap`. Пакет висит на кости `head`, как бороды; показ, цвет, скрытие от первого
лица и под закрытым шлемом: `crates/game-wasm/src/browser/look.rs`, список причёсок: `crates/game-client/src/look.rs`.
Одна причёска на все 4 лица: где череп f2–f4 выходит из подложки, подложка и пряди отодвигаются (до 11 мм).

Мужские (2 ноября 2026): основная форма отодвинута под самый крупный череп, а для каждого лица есть своя посадка,
morph target `Head_m1…m4` (выходит там, где череп этого лица торчит, и прижимается до 1 мм над ним, не глубже 3 мм).
`look.rs` включает морф выбранного лица. Без этого на m1 хвост и зачёс висели до 6,6 мм над головой. Головы берутся
из `output/beards/heads-rest.blend` (`tools/beards/prepare.py`), четвёртым аргументом `game_cards.py`.

Под причёсками у сканов m1–m4 своя запечённая короткая стрижка (светлая у m1, седая у m2/m3). Маска `scalp-male.png`
(`scalp_mask.py`, канал на лицо: R m1 … A m4, в UV головы) показывает шейдеру головы, где она: там стрижка
перекрашивается в цвет причёски с сохранением рисунка щетины (`shaders/look-scalp.glsl`). Маска, средняя яркость
каждой стрижки, цвет кожи лба и щетины каждого лица (`scalp-male.json`: `luma`, `skin`, `buzz`) едут в пакете на
скрытом квадрате `Hair_scalp`, маски линии роста — на `Hair_line_slick` и `Hair_line_ponytail` (квадраты делает
`pack.py`, PNG вешает `finish.py`). В сжатой копии для телефона (`npm run models:ktx2`) эти маски остаются PNG
(`apps/web/scripts/compress-models.ts`): сжатие смешало бы каналы разных лиц.

```bash
B="/c/Program Files/Blender Foundation/Blender 5.2/blender.exe"
for s in pixie bob braid; do "$B" -b 3d/hair-hd/$s.blend --python tools/hair-hd/game_cards.py -- $s <tmp>/game_$s.blend x; done
"$B" -b --factory-startup --python tools/hair-hd/pack.py -- <tmp> <полный путь>/tools/hair-hd/hair-strands.png
python -I tools/hair-hd/finish.py output/hair-hd/raw/hair-female.glb
npx @gltf-transform/cli@4.5.1 meshopt output/hair-hd/raw/hair-female.glb apps/web/models/hair-female.glb --quantize-position 16 --quantize-normal 12 --quantize-texcoord 14
```

Мужские: то же с `male_short.blend → crop`, `male_slick.blend → slick`, `male_medium.blend → medium`,
`male_ponytail.blend → ponytail` и головами для посадок, затем маска, `pack.py … male`, `finish.py` и meshopt:

```bash
"$B" -b 3d/hair-hd/male_short.blend --python tools/hair-hd/game_cards.py -- crop <tmp>/game_crop.blend x "$PWD/output/beards/heads-rest.blend"
"$B" -b --factory-startup --python tools/hair-hd/scalp_mask.py -- "$PWD/apps/web/models/persona-male.glb" "$PWD/tools/hair-hd/scalp-male.png" slick=<tmp>/game_slick_roots.npz ponytail=<tmp>/game_ponytail_roots.npz
"$B" -b --factory-startup --python tools/hair-hd/pack.py -- <tmp> <полный путь>/tools/hair-hd/hair-strands.png male
python -I tools/hair-hd/finish.py output/hair-hd/raw/hair-male.glb
npx @gltf-transform/cli@4.5.1 meshopt output/hair-hd/raw/hair-male.glb apps/web/models/hair-male.glb --quantize-position 16 --quantize-normal 12 --quantize-texcoord 14
```

Маски пересобирать после замены голов и после новых полосок зачёса или хвоста (корни меняются).

`hair-strands.png`: текстура прядей старых женских волос (та же, что в persona-female.glb).

`previews/`: уменьшенные листы для быстрого просмотра.
