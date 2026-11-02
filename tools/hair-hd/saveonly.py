"""Run a groom script without rendering, only to save its .blend: blender -b base.blend --python saveonly.py -- script.py args..."""
import sys, os, runpy
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import hairgen as hg
hg.render_views = lambda *a, **k: None
hg.sheet = lambda *a, **k: None
hg.save_grid = lambda *a, **k: None
i = sys.argv.index("--")
script = sys.argv[i + 1]
sys.argv = sys.argv[:i + 1] + sys.argv[i + 2:]
runpy.run_path(script, run_name="__main__")
