/// <reference lib="dom" />
// The radio in the operator's room: music squeezed through a small speaker.
// Plain Web Audio nodes, so it runs in the Telegram webview on phones.

export interface RadioChain {
  input: AudioNode;
  output: AudioNode;
  /** Hiss under the music while the radio is on. */
  setHiss(on: boolean): void;
}

export const RADIO = {
  lowCutHz: 380,
  presenceHz: 1600,
  presenceDb: 5,
  highCutHz: 3400,
  drive: 2.2,
  hissDb: -40,
} as const;

export function createRadio(ctx: BaseAudioContext): RadioChain {
  const lowCut = ctx.createBiquadFilter();
  lowCut.type = "highpass";
  lowCut.frequency.value = RADIO.lowCutHz;
  lowCut.Q.value = 0.8;

  const presence = ctx.createBiquadFilter();
  presence.type = "peaking";
  presence.frequency.value = RADIO.presenceHz;
  presence.gain.value = RADIO.presenceDb;
  presence.Q.value = 0.9;

  const highCut = ctx.createBiquadFilter();
  highCut.type = "lowpass";
  highCut.frequency.value = RADIO.highCutHz;
  highCut.Q.value = 0.8;

  // a small speaker breaking up a little; the trim keeps the level where it was
  const clip = ctx.createWaveShaper();
  clip.curve = softClipCurve(RADIO.drive);
  clip.oversample = "2x";
  const trim = ctx.createGain();
  trim.gain.value = 1 / (1 + RADIO.drive * 0.35);

  const out = ctx.createGain();
  lowCut.connect(presence).connect(highCut).connect(clip).connect(trim).connect(out);

  const hiss = ctx.createBufferSource();
  hiss.buffer = noiseBuffer(ctx, 2);
  hiss.loop = true;
  const hissBand = ctx.createBiquadFilter();
  hissBand.type = "bandpass";
  hissBand.frequency.value = 2500;
  hissBand.Q.value = 0.7;
  const hissGain = ctx.createGain();
  hissGain.gain.value = 0;
  hiss.connect(hissBand).connect(hissGain).connect(out);
  hiss.start();

  return {
    input: lowCut,
    output: out,
    setHiss(on) {
      const target = on ? 10 ** (RADIO.hissDb / 20) : 0;
      hissGain.gain.setTargetAtTime(target, ctx.currentTime, 0.08);
    },
  };
}

function softClipCurve(drive: number, n = 1024): Float32Array<ArrayBuffer> {
  const curve = new Float32Array(n);
  const norm = Math.tanh(drive);
  for (let i = 0; i < n; i++) {
    const x = (i / (n - 1)) * 2 - 1;
    curve[i] = Math.tanh(drive * x) / norm;
  }
  return curve;
}

function noiseBuffer(ctx: BaseAudioContext, seconds: number): AudioBuffer {
  const buf = ctx.createBuffer(1, Math.floor(ctx.sampleRate * seconds), ctx.sampleRate);
  const data = buf.getChannelData(0);
  for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
  return buf;
}
