# Четыре времени суток Соляного цеха

Ночь дополнительно затемнена по замечанию Арта; начата отдельная карта «Пятиэтажки». Актуальные промпты, измерения яркости и источники — [dark-night-blocks.md](battle-times-review/dark-night-blocks.md). Ночной промпт ниже сохранён как история первого, более яркого варианта.

Дневная карта сохранена. Добавлены утро, вечер и ночь той же карты. Рабочие изображения: `apps/web/public/img/battle-times/{morning,evening,night}/`. `hall` — оборудование и общий вид; `clean` — архитектура за ним, которая открывается при движении камеры. Геометрия, калибровка и карта глубины общие с дневным цехом.

## Изображения и качество

Использован встроенный `image_gen` в режиме `lighting-weather`. Сначала получены три версии общего вида, затем три соответствующих пустых архитектурных подложки. Исходные дневные файлы не заменялись. Сгенерированные PNG имеют **941 × 1672**: запрос большего размера не дал дополнительных пикселей. PNG сохраняются без изменений как старший вариант фона, WebP 94 — обычный сжатый вариант; выбор следует общей политике фоновых изображений для устройства. Искусственного увеличения, дополнительного цветового фильтра и резкости после генерации нет.

`tools/battle-stage/pack_times.py` воспроизводит только WebP-кодирование, проверяет размер и записывает SHA256 в `assets.json`. Он не генерирует свет и не меняет геометрию. При новой генерации требуется повторная визуальная проверка совпадения контуров.

Проекционная реконструкция остаётся приближённой: скрытые стороны оборудования не становятся полноценной круговой моделью. Цвет и интенсивность живых источников подстроены к соответствующей карте; материал за исходным полем зрения использует отдельное освещение продолжения поверхности.

## Промпты общего вида

Общая часть каждого из трёх запросов, edit target — `img/grey-hall-1@2x.jpg`:

```text
Use case: lighting-weather. EDIT TARGET is the supplied image, a production game environment texture. Produce ONE full-bleed portrait 9:16 image at 1882x3344 if possible. Camera and EVERY object's silhouette and location MUST stay pixel registered to the input: same broken roof beams, girders, machinery, conveyor belts, left motor, brick wall, white salt sacks on right, tiny red indicator on right, rubble, floor cracks, distant building and reflections. This is the EXACT same map under different lighting, not a new composition. Do not move/rebuild/add/delete geometry or zoom/crop. Keep high quality cold desaturated industrial photorealism, sharp material textures, realistic cinematic exposure and broad dynamic range. Empty environment only, NO humans, NO robots, NO weapons, NO UI, NO text, NO logos. Preserve original salt/dust distribution and current floor condition. Change ONLY physical light, shadows, reflections and visible sky to:
```

Утро — добавление:

```text
Early morning blue hour just before sunrise. Pale blue-grey sky in the roof opening, cold diffuse dawn light through the left windows, a very faint first pale sunlight edge entering far left. Darker interior than daytime but clearly readable fine salt and oxidized metal, soft long cool shadows, luminous pale salt piles. Natural mist in the distance, NOT white fog filling the hall. Palette graphite, steel grey, restrained slate blue; no orange/cyan blockbuster grade, no artificial light strips. Visibly morning, not identical daytime.
```

Вечер — добавление:

```text
Late evening with a LOW setting sun outside the damaged roof at rear right. A small bright warm-white setting-sun glow in the existing distant opening, warm raking natural light across the right edge of the conveyors and salt sacks, long diagonal architectural shadows across floor, cooler dark graphite shadow areas. Sky muted dusty grey-peach, highlights pale warm bone and steel, with only restrained desaturated warm colour. The same cold salt-factory style, NOT a room flooded with orange. Photoreal atmosphere and light direction, visibly different from daytime.
```

Ночь — добавление:

```text
True NIGHT. The roof opening shows a dark blue-black cloudy night sky and a little cold moonlit cloud glow. All previous bright daylight windows now dark with faint cold grey moonlight. Very low key but carefully exposed readable interior: quiet cool moonlight grazing salt and upper beams, deep neutral graphite machinery, a FEW faint existing industrial emergency amber light sources deep in the distance and the original small red indicator on the right. Do NOT add neon strips, sci-fi panels, dramatic spotlights or many lamps. No daylight white sky, no sunlight. Salt and floor glints should help orientation, center corridor visible. Night atmosphere cinematic yet playable, not a pure black image.
```

## Промпт архитектурных подложек

Image1 — `img/battle-stage/hall-clean@2x.webp`, Image2 — соответствующий новый `hall.png`. Три отдельных вызова, вместо `<time>` подставлены `morning`, `evening`, `night`:

```text
Use case: lighting-weather. Production game CLEAN ARCHITECTURE PLATE. Image1 is the EDIT TARGET: an empty salt-factory hall with NO machinery. Image2 is LIGHTING REFERENCE ONLY, showing the same room with equipment at a different time. Edit ONLY IMAGE1. Keep ALL image1 architecture, perspective, exact camera, framing, floor, roof beams, brick patterns and silhouette pixel registered. Do NOT import ANY machinery, conveyor, sacks, control cabinets or lights from image2; those are separate 3D objects and must be absent from this CLEAN PLATE. Return ONE portrait 9:16 image, 1882x3344 if possible. Match the light/sky/exposure, shadow colour and reflection colour of Image2 precisely on Image1. This plate will appear in small gaps behind moving foreground objects, so coherent matching illumination is mandatory. Preserve material texture and detail. No people, robots, UI, text, weapons, neon, extra structures. Time of day: <time>. Keep the empty plate EMPTY. The only change from Image1 is the lighting/weather.
```

## Просмотр

В игре: «В бой» → меню боя → «Время суток». Выбор начинает новый бой с тем же комплектом. Прямые ссылки поддерживают `?time=morning|day|evening|night`; `?muted=1` отключает звук для проверки. Правила боя описаны в `battle-times-mechanics.md`.
