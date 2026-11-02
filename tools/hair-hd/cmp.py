import sys, os
from PIL import Image
d, g, style, out = sys.argv[1:5]
H = 440
def fit(im): return im.resize((int(im.size[0] * H / im.size[1]), H))
ims = [fit(Image.open(os.path.join(d, f'{style}-{v}.png')).convert('RGB')) for v in ['front', 'back', 'left', 'right']]
gm = [fit(Image.open(os.path.join(g, f'game-{v}.png')).convert('RGB')) for v in ['front', 'back', 'left', 'right']]
s = Image.new('RGB', (max(sum(i.size[0] for i in ims), sum(i.size[0] for i in gm)), H * 2), (200, 200, 200))
x = 0
for i in gm: s.paste(i, (x, 0)); x += i.size[0]
x = 0
for i in ims: s.paste(i, (x, H)); x += i.size[0]
s.save(out, quality=88)
