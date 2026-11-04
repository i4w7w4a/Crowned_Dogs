"""Turn Art's AI sound pack of 2026-11-02 into game files next to the Sonniss ones.

    blender -b --factory-startup -P tools/audio/ai_pack.py -- "/path/to/Dogs-Croft-AI-Sounds"

Runs inside Blender's Python: its audaspace module (aud) decodes, resamples and writes MP3, and
Blender ships numpy, so no ffmpeg is needed. Writes the MP3s to packages/audio/assets, adds them to
sfx-sources.json (each entry has "made_from") and rewrites packages/audio/src/files.generated.ts.

The ElevenLabs clips were made on the free plan: fine for testing, not cleared for a commercial
release. Rebuilding casing/shield-block/amb-base by name with build_sfx.py brings the Sonniss ones back.
"""

import json
import sys
from pathlib import Path

import aud
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from files_ts import write_files_ts  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "packages" / "audio" / "assets"
SR = 44100
EL = "ElevenLabs (free plan: personal non-commercial testing only, not cleared for a commercial release)"
SA = "Stable Audio (commercial use allowed under Stability AI terms)"


def load(path, rate=SR):
    return aud.Sound(str(path)).resample(rate, True).data().astype(np.float64)


def db(x):
    return 20 * np.log10(x + 1e-12)


def frames(m, w=441):
    n = len(m) // w
    return np.sqrt((m[: n * w].reshape(n, w) ** 2).mean(1)), w


def body_rms(m):
    """RMS of the part of the clip within 30 dB of its peak."""
    fr, _ = frames(m)
    on = db(fr) > db(np.abs(m).max()) - 30
    return db(np.sqrt((fr[on] ** 2).mean()))


def trim(m, floor=-50, fade=0.03):
    """Cuts the silence after the last sound 50 dB under the peak, with a short fade."""
    fr, w = frames(m)
    on = np.nonzero(db(fr) > db(np.abs(m).max()) + floor)[0]
    y = m[on[0] * w : min(len(m), (on[-1] + 1) * w + int(fade * SR))].copy()
    f = int(fade * SR)
    y[-f:] *= np.linspace(1, 0, f)
    y[:32] *= np.linspace(0, 1, 32)
    return y


def level(y, rms, peak):
    return y * 10 ** (min(rms - body_rms(y), peak - db(np.abs(y).max())) / 20)


def biquad(x, b, a):
    return aud.Sound.buffer(x.astype(np.float32), 48000).filter(b, a).data().astype(np.float64)


def loudness(x):
    """BS.1770 integrated loudness, as pyloudnorm measures it in dsp.loudness."""
    rate = 48000
    w0 = 2 * np.pi * 1500 / rate
    A, al = 10 ** (4 / 40), np.sin(w0) / (2 / np.sqrt(2))
    c = np.cos(w0)
    a0 = (A + 1) - (A - 1) * c + 2 * np.sqrt(A) * al
    shelf = ([A * ((A + 1) + (A - 1) * c + 2 * np.sqrt(A) * al) / a0, -2 * A * ((A - 1) + (A + 1) * c) / a0,
              A * ((A + 1) + (A - 1) * c - 2 * np.sqrt(A) * al) / a0],
             [1, 2 * ((A - 1) - (A + 1) * c) / a0, ((A + 1) - (A - 1) * c - 2 * np.sqrt(A) * al) / a0])
    w0 = 2 * np.pi * 38 / rate
    al, c = np.sin(w0) / (2 * 0.5), np.cos(w0)
    hp = ([(1 + c) / 2 / (1 + al), -(1 + c) / (1 + al), (1 + c) / 2 / (1 + al)], [1, -2 * c / (1 + al), (1 - al) / (1 + al)])
    if len(x) < int(0.45 * rate):
        x = np.concatenate([x, np.zeros((int(0.45 * rate) - len(x), x.shape[1]))])
    y = biquad(biquad(x, *shelf), *hp)
    block, step = int(0.4 * rate), int(0.1 * rate)
    z = np.array([(y[i : i + block] ** 2).mean(0) for i in range(0, len(y) - block + 1, step)])
    lk = -0.691 + 10 * np.log10(z.sum(1) + 1e-12)
    z = z[lk > -70]
    gate = -0.691 + 10 * np.log10(z.mean(0).sum()) - 10
    z = z[-0.691 + 10 * np.log10(z.sum(1)) > gate]
    return -0.691 + 10 * np.log10(z.mean(0).sum())


def punch(m, win=0.05):
    """Loudest 50 ms above 150 Hz, as dsp.punch."""
    m = aud.Sound.buffer(m.reshape(-1, 1).astype(np.float32), 48000).highpass(150, 0.7071).data()[:, 0].astype(np.float64)
    n = int(win * 48000)
    c = np.concatenate([[0.0], np.cumsum(m * m)])
    return float(10 * np.log10(((c[n:] - c[:-n]) / n).max() + 1e-12)) if len(m) > n else float(10 * np.log10(c[-1] / len(m)))


report = {}


def write(rel, y, made_from, source, stereo=False, loop=False):
    out = ASSETS / rel
    data = (y if stereo else y.reshape(-1, 1)).astype(np.float32)
    aud.Sound.buffer(data, SR).write(str(out), SR, aud.CHANNELS_STEREO if stereo else aud.CHANNELS_MONO,
                                    aud.FORMAT_S16, aud.CONTAINER_MP3, aud.CODEC_MP3, (112 if stereo else 96) * 1000)
    x = load(out, 48000)
    m = x.mean(1)
    report[rel] = {
        "seconds": round(len(y) / SR, 3),
        "lufs": round(loudness(x if stereo else m.reshape(-1, 1)), 1),
        "punch_db": round(punch(m), 1),
        "peak_db": round(float(db(np.abs(m).max())), 1),
        "loop": loop,
        "sources": [source],
        "made_from": made_from,
    }
    print(rel, report[rel])


def main(pack):
    pack = Path(pack)
    i = 0
    for kind in ("01_Electric_Snap", "02_Magnetic_Ripple"):
        for v in range(1, 5):
            i += 1
            m = load(pack / f"ElevenLabs/Shield/{kind}_v{v}.mp3").mean(1)
            write(f"sfx/shield-block-{i}.mp3", level(trim(m), -15.0, -1.0), f"ElevenLabs/Shield/{kind}_v{v}.mp3", EL)
    for kind, name in (("01_Concrete", "casing"), ("02_Steel_Deck", "casing-steel")):
        for v in range(1, 5):
            m = load(pack / f"ElevenLabs/Casings/{kind}_v{v}.mp3").mean(1)
            write(f"sfx/{name}-{v}.mp3", level(trim(m), -27.0, -3.0), f"ElevenLabs/Casings/{kind}_v{v}.mp3", EL)
    # The generated "approach" clips start loud and die away. A shell coming down should grow, so each
    # keeps its first 1.8 s, has its own decay levelled out and then rises 18 dB into the blast.
    length = 1.8
    i = 0
    for kind in ("01_Heavy_Approach", "02_Whistling_Approach"):
        for v in range(1, 5):
            i += 1
            m = load(pack / f"ElevenLabs/Artillery_Flight/{kind}_v{v}.mp3").mean(1)
            fr, w = frames(m)
            start = np.nonzero(db(fr) > db(np.abs(m).max()) - 40)[0][0] * w
            y = m[start : start + int(length * SR)].copy()
            fr, w = frames(y, 2205)
            slope = np.polyfit((np.arange(len(fr)) + 0.5) * w / SR, db(fr), 1)[0]
            t = np.arange(len(y)) / SR
            y *= 10 ** ((-slope * t - 18 * (1 - t / length)) / 20)
            f = int(0.06 * SR)
            y[:f] *= np.linspace(0, 1, f)
            f = int(0.02 * SR)
            y[-f:] *= np.linspace(1, 0, f)
            g = min(-14.0 - db(np.sqrt((y[-int(0.4 * SR) :] ** 2).mean())), -2.0 - db(np.abs(y).max()))
            write(f"sfx/artillery-incoming-{i}.mp3", y * 10 ** (g / 20),
                  f"ElevenLabs/Artillery_Flight/{kind}_v{v}.mp3, rising envelope", EL)
    # 30 s ambiences: the last 3 s fade over the first 3 s, so the 27 s file loops without a seam.
    for rel, path, rms in (("amb/amb-base.mp3", "StableAudio/Base/01_Utility_Corridor.wav", -33.0),
                           ("amb/amb-workshop.mp3", "StableAudio/Workshop/01_Machined_Air_Hum.wav", -33.5)):
        x = load(pack / path)
        X = 3 * SR
        n = len(x) - X
        y = x[:n].copy()
        k = np.linspace(0, np.pi / 2, X).reshape(-1, 1)
        y[:X] = x[:X] * np.sin(k) + x[n:] * np.cos(k)
        write(rel, y * 10 ** ((rms - body_rms(y.mean(1))) / 20), f"{path}, 3 s crossfade loop", SA, stereo=True, loop=True)
    report_path = ASSETS / "sfx-sources.json"
    full = json.loads(report_path.read_text(encoding="utf-8"))
    full.update(report)
    report_path.write_text(json.dumps(dict(sorted(full.items())), ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    write_files_ts(full)


main(sys.argv[sys.argv.index("--") + 1])
