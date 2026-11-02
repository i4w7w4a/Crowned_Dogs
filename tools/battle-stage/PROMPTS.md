# Промпты материалов проекционной сцены


## Продолжение пола и стен вне исходного кадра

Встроенный imagegen, новые текстуры материала для мировой геометрии. Не зеркальные повторы фотографии комнаты. Геометрия уходит на 40 м позади исходной камеры.

### hall-floor.png

Use case photorealistic-natural, production game material. Generate ONE seamless square 1024x1024 tileable texture of the damp gritty salt-encrusted dark concrete factory floor from the attached image. Reference is material appearance only. Orthographic straight overhead surface scan, no perspective, no room, no walls, no equipment, no people, no text. Cold desaturated charcoal-grey concrete aggregate, irregular salt dust and small scattered broken salt/concrete chips, hairline cracks, dark wet patches. Subtle neutral diffuse illumination, no recognizable reflections of windows or architecture, no cast shadows, no bright highlights, no large dominant shapes. Low-to-medium contrast, mostly dark grey so it matches the photographed floor, enough crisp fine grit. Make all four edges seamless for repeating across a broad real 3D ground plane, no border. Output only the texture.

### hall-wall.png

Generate ONE seamless square 1024x1024 game material texture, orthographic straight-on neutral diffuse scan of an old cold grey industrial factory brick wall: small desaturated grey-brown bricks, eroded dark thin mortar, salt efflorescence, subtle grime, minor chipped edges, realistic weathered surface. Overall dark medium grey (around RGB80) with restrained contrast, no bright white areas. Uniform scale and illumination, no directional cast shadows, no perspective, no scene, no objects, no openings or windows, no text, no frame. All four edges seamless when tiled. Gritty photographic abandoned factory material, NOT stylized.

`hall-roof.png` — ранее сгенерированный `textures-v2/worn-steel.png` из этой же сцены.


## hall-clean.png — чистая архитектурная подложка

Встроенный imagegen, редактирование battle-source.jpg. Исходный фон оборудования сохранён отдельно; изображения не смешиваются на одних поверхностях.

Use case: precise-object-edit. Asset type: clean background projection plate for a 3D game environment, portrait 9:16, retaining EXACT camera viewpoint, vanishing point, geometry and color grading of the attached abandoned salt-processing factory. This is a VFX clean plate reconstruction, NOT a redesigned room. Remove ALL foreground and midground machinery and loose equipment: entire big hopper/crusher/electric motor and concrete equipment plinths on left, ALL conveyor belts and their rails, support frames and posts, central machinery hoppers, right electrical cabinets, workbenches, ALL sacks and pallets. Remove their attached shadows. Reconstruct the floor and walls that were hidden behind them. Keep the brick side walls, high factory windows, structural building columns, corrugated roof and original roof trusses, broken roof opening, distant ruined wall and atmospheric depth exactly registered to the original picture. Floor should be a continuous believable flat dirty concrete/salt floor, preserve the existing visible foreground floor/reflections/rubble pattern and continue it naturally into removed areas. Do not add any people, robots, props, new openings, new lights, captions, interface or borders. Preserve cold desaturated grey/charcoal photographic style, exposure, light direction and source texture sharpness. Do not brighten or stylize. Maintain exact original framing and portrait ratio. Output only the cleaned empty building image, ready for projection on physical floor and wall planes.
