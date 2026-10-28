"""Small DSP toolbox for building the game's sounds from library recordings.

Everything works on float32 numpy arrays at 48 kHz: mono (n,) or stereo (n, 2).
"""

from fractions import Fraction
from math import gcd

import numpy as np
import soundfile as sf
from scipy.signal import butter, fftconvolve, resample_poly, sosfilt

SR = 48000


def load(path, stereo=False):
    x, fs = sf.read(path, always_2d=True, dtype="float32")
    if x.shape[1] == 1:
        x = np.repeat(x, 2, axis=1)
    x = x[:, :2]
    if fs != SR:
        g = gcd(fs, SR)
        x = resample_poly(x, SR // g, fs // g, axis=0).astype("float32")
    return x if stereo else x.mean(axis=1)


def db(v):
    return 20 * np.log10(np.maximum(np.abs(v), 1e-12))


def gain(x, g_db):
    return x * np.float32(10 ** (g_db / 20))


def envelope_db(m, win=0.005):
    n = max(1, int(SR * win))
    k = len(m) // n
    e = np.sqrt((m[: k * n].reshape(k, n) ** 2).mean(axis=1) + 1e-12)
    return 20 * np.log10(e), n / SR


def find_events(m, thresh_rel=-30, gap=0.25, tail_rel=-45):
    """Separate hits in a take: [(start, peak_time, end)] in seconds."""
    e, hop = envelope_db(m)
    peak = e.max()
    floor = np.percentile(e, 10)
    above = e > max(peak + thresh_rel, floor + 15)
    out, i = [], 0
    while i < len(e):
        if not above[i]:
            i += 1
            continue
        j = last = i
        while j < len(e) and (j - last) * hop < gap:
            if above[j]:
                last = j
            j += 1
        pk = i + int(e[i : last + 1].argmax())
        lim = max(e[pk] + tail_rel, floor + 6)
        t = pk
        while t < len(e) and e[t] > lim:
            t += 1
        out.append((i * hop, pk * hop, t * hop))
        i = max(last + 1, t)
    return out


def ring_out(m, start, max_len, rel=-55):
    """Length from start until the sound decays `rel` dB under its peak."""
    seg = m[int(start * SR) : int((start + max_len) * SR)]
    if len(seg) == 0:
        return 0.0
    e, hop = envelope_db(seg)
    lim = e.max() + rel
    above = np.nonzero(e > lim)[0]
    return min(max_len, (above[-1] + 1) * hop + 0.02) if len(above) else max_len


def pitch(x, semitones):
    """Resample-based shift: higher is also shorter, like a tape."""
    if not semitones:
        return x
    f = Fraction(2 ** (-semitones / 12)).limit_denominator(240)
    return resample_poly(x, f.numerator, f.denominator, axis=0).astype("float32")


def _sos(kind, hz, order=2):
    return butter(order, hz, btype=kind, fs=SR, output="sos")


def highpass(x, hz, order=2):
    return sosfilt(_sos("highpass", hz, order), x, axis=0).astype("float32")


def lowpass(x, hz, order=2):
    return sosfilt(_sos("lowpass", hz, order), x, axis=0).astype("float32")


def bandpass(x, lo, hi, order=2):
    return sosfilt(butter(order, [lo, hi], btype="bandpass", fs=SR, output="sos"), x, axis=0).astype("float32")


def fade(x, fade_in=0.002, fade_out=0.0, curve=2.0):
    x = x.copy()
    n_in = min(len(x), int(fade_in * SR))
    if n_in:
        ramp = np.linspace(0, 1, n_in, dtype="float32")
        x[:n_in] *= ramp if x.ndim == 1 else ramp[:, None]
    n_out = min(len(x), int(fade_out * SR))
    if n_out:
        ramp = np.linspace(1, 0, n_out, dtype="float32") ** curve
        x[-n_out:] *= ramp if x.ndim == 1 else ramp[:, None]
    return x


def place(dst, src, at):
    """Add src into dst at `at` seconds, growing dst if needed."""
    i = int(round(at * SR))
    need = i + len(src)
    if need > len(dst):
        pad = np.zeros((need - len(dst),) + dst.shape[1:], dtype="float32")
        dst = np.concatenate([dst, pad])
    dst[i:need] += src
    return dst


def hall_ir(rt60=1.4, predelay=0.018, bright=5500, seed=7, stereo=False):
    """A synthetic concrete-hall impulse response: early taps plus a decaying tail."""
    rng = np.random.default_rng(seed)
    n = int((rt60 * 1.1 + predelay) * SR)
    t = np.arange(n) / SR
    ch = 2 if stereo else 1
    noise = rng.standard_normal((n, ch)).astype("float32")
    tail = noise * np.exp(-6.9 * t / rt60)[:, None]
    tail[: int(predelay * SR)] = 0
    tail = lowpass(tail, bright)
    for k, (dt, g) in enumerate([(0.011, 0.5), (0.023, 0.35), (0.037, 0.3), (0.052, 0.22)]):
        tail[int(dt * SR), :] += g * (1 if k % 2 == 0 else -1)
    tail /= np.sqrt((tail**2).sum(axis=0, keepdims=True)) + 1e-9
    return tail if stereo else tail[:, 0]


def reverb(x, wet, **kw):
    if wet <= 0:
        return x
    ir = hall_ir(stereo=x.ndim == 2, **kw)
    if x.ndim == 1:
        y = fftconvolve(x, ir)[: len(x) + len(ir) - 1]
    else:
        y = np.stack([fftconvolve(x[:, c], ir[:, c]) for c in range(2)], axis=1)
    dry = np.zeros_like(y)
    dry[: len(x)] = x
    return (dry * (1 - wet * 0.5) + y * wet * 0.9).astype("float32")


def limit(x, ceiling_db=-1.0, release=0.06, lookahead=0.0015):
    """Peak limiter: keeps sample peaks under the ceiling without hard clipping."""
    c = 10 ** (ceiling_db / 20)
    a = np.abs(x) if x.ndim == 1 else np.abs(x).max(axis=1)
    need = np.minimum(1.0, c / np.maximum(a, 1e-9))
    la = max(1, int(lookahead * SR))
    # look ahead: each sample takes the minimum gain of the next `la` samples
    padded = np.concatenate([need, np.ones(la)])
    from numpy.lib.stride_tricks import sliding_window_view

    g = sliding_window_view(padded, la + 1).min(axis=1)[: len(need)]
    # release: gain recovers slowly
    r = np.exp(-1 / (release * SR))
    out = np.empty_like(g)
    cur = 1.0
    for i, v in enumerate(g):
        cur = v if v < cur else v + (cur - v) * r
        out[i] = cur
    return (x * (out if x.ndim == 1 else out[:, None])).astype("float32")


def trim_tail(x, rel=-60, keep=0.03):
    m = x if x.ndim == 1 else np.abs(x).max(axis=1)
    e, hop = envelope_db(m)
    above = np.nonzero(e > e.max() + rel)[0]
    if not len(above):
        return x
    end = min(len(x), int(((above[-1] + 1) * hop + keep) * SR))
    return fade(x[:end], 0, min(0.04, end / SR / 4))


def loudness(x):
    import pyloudnorm as pyln

    y = x if x.ndim == 2 else x[:, None]
    need = int(0.45 * SR)
    if len(y) < need:
        y = np.concatenate([y, np.zeros((need - len(y), y.shape[1]), dtype="float32")])
    return pyln.Meter(SR).integrated_loudness(y)


def punch(x, win=0.05):
    """Loudest 50 ms of the clip (RMS, dBFS, lows cut): how hard a hit lands.
    Integrated loudness misjudges gunshots, whose tails dilute the number."""
    m = x if x.ndim == 1 else x.mean(axis=1)
    m = highpass(m, 150).astype("float64")
    n = max(1, int(win * SR))
    c = np.concatenate([[0.0], np.cumsum(m * m)])
    if len(m) <= n:
        return float(10 * np.log10(c[-1] / max(1, len(m)) + 1e-12))
    rms = (c[n:] - c[:-n]) / n
    return float(10 * np.log10(rms.max() + 1e-12))


def loop_crossfade(x, xfade=3.0):
    """Fold the end over the start so the clip loops without a seam."""
    n = int(xfade * SR)
    body, tail = x[:-n].copy(), x[-n:]
    t = np.linspace(0, np.pi / 2, n, dtype="float32")
    fin, fout = np.sin(t), np.cos(t)
    if x.ndim == 2:
        fin, fout = fin[:, None], fout[:, None]
    body[:n] = body[:n] * fin + tail * fout
    return body
