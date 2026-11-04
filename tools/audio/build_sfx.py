#!/usr/bin/env python3
"""Build the game's sound effects and ambience loops from the Sonniss GDC 2024 bundle.

    SONNISS_DIR="/path/to/Sonniss.com - GDC 2024 - Game Audio Bundle" \
        python3 tools/audio/build_sfx.py [name ...]

Needs ffmpeg plus numpy, scipy, soundfile and pyloudnorm. Writes MP3s to
packages/audio/assets/sfx and packages/audio/assets/amb, and a report of what
each file is made of to packages/audio/assets/sfx-sources.json.
"""

import json
import os
import subprocess
import sys
import tempfile
from functools import lru_cache
from pathlib import Path

import numpy as np
import soundfile as sf

sys.path.insert(0, str(Path(__file__).resolve().parent))
import dsp  # noqa: E402
from files_ts import write_files_ts  # noqa: E402
from recipes import AMB, SFX, SRC  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "packages" / "audio" / "assets"


def bundle_path(rel):
    base = Path(os.environ["SONNISS_DIR"])
    p = base / rel
    if p.exists():
        return p
    # copies staged from Windows sometimes carry "&amp;" in folder names
    alt = base / rel.replace("&", "&amp;")
    if alt.exists():
        return alt
    raise FileNotFoundError(rel)


@lru_cache(maxsize=None)
def source(key, stereo=False):
    return dsp.load(bundle_path(SRC[key]), stereo=stereo)


@lru_cache(maxsize=None)
def events(key):
    return dsp.find_events(source(key))


def cut(layer, stereo):
    x = source(layer["src"], stereo)
    mono = source(layer["src"]) if stereo else x
    if layer["segs"]:
        parts = [x[int(a * dsp.SR) : int(b * dsp.SR)] for a, b in layer["segs"]]
        out = parts[0]
        xf = int(0.015 * dsp.SR)
        for p in parts[1:]:
            ramp = np.linspace(0, 1, xf, dtype="float32")
            if out.ndim == 2:
                ramp = ramp[:, None]
            out[-xf:] = out[-xf:] * (1 - ramp) + p[:xf] * ramp
            out = np.concatenate([out, p[xf:]])
        seg = dsp.fade(out, 0.002, min(0.2, len(out) / dsp.SR / 3))
        return seg, 0.0
    if layer["t"] is not None:
        start = layer["t"]
        nxt = None
    else:
        evs = events(layer["src"])
        start = max(0.0, evs[layer["ev"]][0] - 0.004)
        nxt = evs[layer["ev"] + 1][0] - 0.01 if layer["ev"] + 1 < len(evs) else None
    max_len = layer["dur"] or 2.5
    if nxt is not None:
        max_len = min(max_len, nxt - start)
    length = dsp.ring_out(mono, start, max_len, layer["rel"]) if layer["t"] is None else max_len
    seg = x[int(start * dsp.SR) : int((start + length) * dsp.SR)].copy()
    if layer["rev"]:
        seg = seg[::-1].copy()
    seg = dsp.pitch(seg, layer["st"])
    if layer["hp"]:
        seg = dsp.highpass(seg, layer["hp"])
    if layer["lp"]:
        seg = dsp.lowpass(seg, layer["lp"])
    fo = layer["fo"] if layer["fo"] is not None else min(0.15, len(seg) / dsp.SR / 4)
    seg = dsp.fade(seg, layer["fi"], fo)
    seg = dsp.gain(seg, layer["g"])
    m = seg if seg.ndim == 1 else np.abs(seg).max(axis=1)
    peak_t = float(np.argmax(np.abs(m))) / dsp.SR
    return seg, peak_t


def render(name, r):
    stereo = r["stereo"]
    shape = (0, 2) if stereo else (0,)
    mix = np.zeros(shape, dtype="float32")
    anchor = None
    for layer in r["layers"]:
        seg, peak_t = cut(layer, stereo)
        at = layer["at"]
        if layer["align_peak"]:
            at = max(0.0, (anchor or 0.0) - peak_t)
        elif anchor is None:
            anchor = at + peak_t
        mix = dsp.place(mix, seg, at)
    if r["room"]:
        mix = dsp.reverb(mix, r["room"], rt60=r["rt60"])
    if r["hp"]:
        mix = dsp.highpass(mix, r["hp"])
    if r["lp"]:
        mix = dsp.lowpass(mix, r["lp"])
    if r["loop"]:
        n = int((r["loop"] + 3.0) * dsp.SR)
        mix = dsp.loop_crossfade(mix[:n], 3.0)
    else:
        mix = dsp.trim_tail(mix, r["tail"])
    if r["punch"] is not None:
        want = r["punch"] - dsp.punch(mix)
    else:
        want = r["loud"] - dsp.loudness(mix)
    peak = float(dsp.db(np.abs(mix).max()))
    mix = dsp.gain(mix, min(want, r["ceiling"] - peak + r["max_gr"]))
    mix = dsp.limit(mix, r["ceiling"])
    return mix


def encode(x, out, stereo, kbps):
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(suffix=".wav") as tmp:
        sf.write(tmp.name, x, dsp.SR, subtype="FLOAT")
        subprocess.run(
            ["ffmpeg", "-v", "error", "-y", "-i", tmp.name, "-ar", "44100", "-ac", "2" if stereo else "1",
             "-c:a", "libmp3lame", "-b:a", f"{kbps}k", "-map_metadata", "-1", "-id3v2_version", "0", str(out)],
            check=True,
        )


def main(only):
    report_path = ASSETS / "sfx-sources.json"
    report = json.loads(report_path.read_text()) if report_path.exists() else {}
    jobs = [("sfx", k, v) for k, v in SFX.items()] + [("amb", k, v) for k, v in AMB.items()]
    known = {f"{folder}/{name}.mp3" for folder, name, _ in jobs}
    # Files made by ai_pack.py carry "made_from"; they stay unless their recipe is rebuilt by name.
    report = {k: v for k, v in report.items() if k in known or "made_from" in v}
    for folder, name, r in jobs:
        if only and name not in only:
            continue
        if not only and "made_from" in report.get(f"{folder}/{name}.mp3", {}):
            continue
        x = render(name, r)
        stereo = r["stereo"]
        kbps = 112 if folder == "amb" else (128 if stereo else 96)
        out = ASSETS / folder / f"{name}.mp3"
        encode(x, out, stereo, kbps)
        report[f"{folder}/{name}.mp3"] = {
            "seconds": round(len(x) / dsp.SR, 3),
            "lufs": round(dsp.loudness(x), 1),
            "punch_db": round(dsp.punch(x), 1),
            "peak_db": round(float(dsp.db(np.abs(x).max())), 1),
            "loop": bool(r["loop"]),
            "sources": sorted({SRC[l["src"]].split("/")[0] for l in r["layers"]}),
        }
        print(f"{folder}/{name}.mp3  {report[f'{folder}/{name}.mp3']['seconds']:6.2f}s  "
              f"{report[f'{folder}/{name}.mp3']['lufs']:6.1f} LUFS  punch {report[f'{folder}/{name}.mp3']['punch_db']:6.1f}  "
              f"peak {report[f'{folder}/{name}.mp3']['peak_db']:5.1f}")
    report_path.write_text(json.dumps(dict(sorted(report.items())), ensure_ascii=False, indent=1) + "\n")
    write_files_ts(report)


if __name__ == "__main__":
    main(set(sys.argv[1:]))
