# Заметки направлений при передаче

Снимки на 28 октября 2026: так каждое направление описало своё состояние, когда облачные сессии передавали работу в Claude Code. Оригиналы лежат рядом с исходниками в файлах проекта (`project-files\crowned-dogs\...`), и если они там новее, верь им.

| Файл | Направление | Оригинал в файлах проекта |
| --- | --- | --- |
| `core-arena.md` | боевое ядро, бот, арена, 3D в игре | `crowned-dogs/handoff/core-arena.md` |
| `combat-ui.md` | интерфейс экрана боя | (написан при передаче) |
| `weapons.md` | каталог оружия, экран снаряжения, 3D-стволы | `crowned-dogs/weapons/HANDOFF.md` |
| `weapons-3d-spec.md` | размеры стволов и гнёзд в мм | `crowned-dogs/weapons/weapons-3d-spec.md` |
| `armor.md` | подгонка брони, набор A | `crowned-dogs/3d/armor/README.md` |
| `armor-game.md` | броня в игровых моделях | `crowned-dogs/3d/armor-game/README.md` |
| `anim-mocap.md` | анимации боя с видео Арта | `crowned-dogs/3d/anim-mocap/README.md` |
| `persona-male.md`, `persona-female.md` | модели Персон | `crowned-dogs/3d/persona-male-art/README.md`, `crowned-dogs/3d/persona-female/README.md` |
| `logo.md` | логотип и экран загрузки | `crowned-dogs/logo/README.md` |
| `music-analysis.md` | разбор треков Арта | `crowned-dogs/music/ANALYSIS.md` |

Пути вида `/mnt/project-files/...` в заметках означают папку `C:\Users\qtas5\Documents\CrownedDogs\project-files\`. Скрипты из файлов проекта писались под облачный Linux: пути внутри надо поправить под Windows.

В `weapons.md` устарел один абзац: запасной ствол в бою и выбор набора ветерана в боте и арене уже сделаны.
