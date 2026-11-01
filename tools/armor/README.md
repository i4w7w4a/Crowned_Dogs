# Броня, вторая партия: подгонка и сборка в игру

Вещи из Tripo (кепка, «Сфера», «Плита-4», «Рывок», берцы) подгоняются на обе Персоны в Blender и вливаются в
`apps/web/models/persona-{male,female}.glb`. Рабочие файлы на ПК: `Documents\CrownedDogs\3d\04-armor`
(`fit2\` подгонка, `game2\` выгрузка, `recolour\` перекраски). Blender 5.2, `gltf-transform` 4.5.1.

## Шаги

1. `run_batch2.sh` — подгонка каждой вещи (`fit_garment.py`) на обе Персоны, `fit2/{m,f}-<вещь>.blend`.
   Полевой костюм обрезается над берцами (`cut_suit.py`). Берцы подгоняются без костюма: мягкое выталкивание
   через штанину давало лохмотья на голенище.
2. `dress_gear.py -- ANIM_ARMOR.blend FITDIR m|f CUT OUT.glb` — все вещи на игровом скелете, маски тела
   (`_suit`, `_vest`, `_ryvok`, `_plita4`, `_boots`), прореживание. Перед прореживанием вершины сшиваются
   (Tripo режет сетку по швам UV, без сшивки она рвётся).
3. `gltf-transform resize --width 1024 --height 1024`, затем `webp --quality 88`.
4. `recolour_suit.py SUIT_COLOR.webp OUT...` — «Солончак» и «Тень» из цвета полевого костюма.
5. `node merge_gear.mjs GAME.glb GEAR.glb OUT.glb RECOLOUR_DIR` — вливает вещи в игровую модель. GAME берите из
   git (`git show HEAD:apps/web/models/persona-male.glb`), а не уже слитый файл.
6. Превью для Снаряжения: `preview_gear.py -- GEAR.glb RECOLOUR_DIR OUT.glb`, затем meshopt, resize 512,
   webp 85 и base64 в `apps/web/public/preview/gear.json`. Предмет показывает свою часть через `gear#Имя` в
   `models()` (`crates/game-client/src/workshop.rs`).
7. Текстуры 2K/4K: `apps/web/scripts/texture-tiers.py`, записи в `public/tex/{armor,persona-*}.json`. Запись
   материала — строка (одна основа на все карты) или `{color, normal, orm}` (перекраска берёт чужие нормали и ORM).

Связь предмета каталога с мешем — `GEAR` и `SLOTS` в `crates/game-wasm/src/browser/scene_assets.rs`.
