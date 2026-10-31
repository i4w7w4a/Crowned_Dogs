"""Instructor Sedoy's voice lines, spoken by Google Gemini TTS and passed through a field radio.

    python -I tools/audio/build_voice.py            # every line
    python -I tools/audio/build_voice.py 3 5        # only these lines

Needs numpy, scipy and lameenc, and a Google AI key in GOOGLE_AI_KEY or in
~/.crowned-dogs/google-ai.key (never in the repo). Writes
packages/audio/assets/voice/sedoy-N.mp3, assets/voice-sources.json and the voice
rows of packages/audio/src/files.generated.ts. The lines follow BRIEFING in
crates/game-wasm/src/browser/creation.rs; the tag line is spoken without the tag.
"""
import base64
import json
import os
import pathlib
import sys
import time
import urllib.error
import urllib.request

import lameenc
import numpy as np
from scipy import signal

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / "packages" / "audio" / "assets" / "voice"
FILES_TS = ROOT / "packages" / "audio" / "src" / "files.generated.ts"
MODEL = "gemini-3.8-flash-tts"
# A mature, deep, gritty male voice; Gemini reads any style note aloud, so the text goes in bare.
VOICE = "Algenib"
SR = 24000
LINES = [
    "Оператор, слышишь меня? Я Седой, твой инструктор. Говорю коротко, пока канал держится.",
    "Воевать будешь не сам. Ляжешь в капсулу, наденешь корону и поведёшь Персону, боевого робота.",
    "Персона чувствует всё, значит, и ты тоже. Её убьют, ты очнёшься в капсуле. Но больно будет по-настоящему.",
    "Теперь контракт с ЧВК. Служба по контракту, платим водой. Отказ не принимается: умирать-то будешь не ты.",
    "Подписано. Вот твой жетон. По нему тебя найдут и по нему заплатят.",
    "Теперь соберём тебе Персону. Её делают похожей на оператора: так мозг быстрее принимает чужое тело.",
]


def key():
    k = os.environ.get("GOOGLE_AI_KEY") or (pathlib.Path.home() / ".crowned-dogs" / "google-ai.key").read_text()
    return k.strip()


def speak(text):
    body = {"contents": [{"parts": [{"text": text}]}],
            "generationConfig": {"responseModalities": ["AUDIO"],
                                 "speechConfig": {"voiceConfig": {"prebuiltVoiceConfig": {"voiceName": VOICE}}}}}
    req = urllib.request.Request(
        f"https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent",
        data=json.dumps(body).encode(), headers={"Content-Type": "application/json", "x-goog-api-key": key()})
    for attempt in range(8):
        try:
            d = json.load(urllib.request.urlopen(req, timeout=180))
            break
        except urllib.error.HTTPError as e:
            if e.code not in (429, 500, 503) or attempt == 7:
                raise SystemExit(f"{e.code}: {e.read().decode()[:300]}")
            time.sleep(20 + 10 * attempt)
    raw = base64.b64decode(d["candidates"][0]["content"]["parts"][0]["inlineData"]["data"])
    if raw[:4] == b"RIFF":
        raw = raw[raw.index(b"data") + 8:]
    return np.frombuffer(raw, np.int16).astype(np.float64) / 32768


def radio(x):
    """Trim the silence, then a clean field radio: band-limited, a little drive, a breath of hiss."""
    env = np.convolve(np.abs(x), np.ones(480) / 480, "same")
    on = np.flatnonzero(env > 10 ** (-45 / 20))
    x = x[max(0, on[0] - 1200): on[-1] + 2400]
    band = signal.sosfilt(signal.butter(4, [280, 3600], "bandpass", fs=SR, output="sos"), x)
    body = signal.sosfilt(signal.butter(2, 180, "highpass", fs=SR, output="sos"), x)
    y = 0.75 * band + 0.25 * body
    y = np.tanh(2.2 * y / np.abs(y).max()) / np.tanh(2.2)
    hiss = signal.sosfilt(signal.butter(2, [1500, 5000], "bandpass", fs=SR, output="sos"),
                          np.random.default_rng(7).standard_normal(len(y)))
    y = y + hiss / np.abs(hiss).max() * 10 ** (-42 / 20)
    fade = int(0.02 * SR)
    y[:fade] *= np.linspace(0, 1, fade)
    y[-fade:] *= np.linspace(1, 0, fade)
    rms = np.sqrt(np.mean(y[np.abs(y) > 0.02] ** 2))
    return np.clip(y * (10 ** (-17 / 20) / rms), -0.98, 0.98)


def encode(y, out):
    enc = lameenc.Encoder()
    enc.set_bit_rate(64)
    enc.set_in_sample_rate(SR)
    enc.set_channels(1)
    enc.set_quality(2)
    out.write_bytes(enc.encode((y * 32767).astype(np.int16).tobytes()) + enc.flush())


def write_files_ts(report):
    """Replace the voice rows of files.generated.ts; build_sfx.py keeps them when it rewrites the file."""
    lines = [l for l in FILES_TS.read_text(encoding="utf-8").split("\n") if not l.startswith('  "voice/')]
    end = lines.index("};")
    rows = [f'  "{p}": {{ seconds: {info["seconds"]}, trimDb: 0.0 }},' for p, info in sorted(report.items())]
    FILES_TS.write_text("\n".join(lines[:end] + rows + lines[end:]), encoding="utf-8")


def main(only):
    OUT.mkdir(parents=True, exist_ok=True)
    report_path = OUT.parent / "voice-sources.json"
    report = json.loads(report_path.read_text(encoding="utf-8")) if report_path.exists() else {}
    for n, text in enumerate(LINES, 1):
        if only and n not in only:
            continue
        y = radio(speak(text))
        encode(y, OUT / f"sedoy-{n}.mp3")
        report[f"voice/sedoy-{n}.mp3"] = {"seconds": round(len(y) / SR, 3), "text": text,
                                          "source": f"Google {MODEL}, voice {VOICE}"}
        print(f"voice/sedoy-{n}.mp3  {len(y) / SR:5.2f}s")
    report_path.write_text(json.dumps(dict(sorted(report.items())), ensure_ascii=False, indent=1) + "\n",
                           encoding="utf-8")
    write_files_ts(report)


if __name__ == "__main__":
    main({int(a) for a in sys.argv[1:]})
