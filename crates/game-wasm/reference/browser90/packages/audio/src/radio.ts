/// <reference lib="dom" />
// The radio in the operator's room: music squeezed through a small speaker
// that sits in a small concrete room. Plain Web Audio nodes, so it runs in the
// Telegram webview on phones.

import { audioSamples, callRust } from "../../runtime/src/index.ts";

export interface RadioChain {
  input: AudioNode;
  output: AudioNode;
  /** Hiss under the music while the radio is on. */
  setHiss(on: boolean): void;
  /** Where the radio stands: pan -1 left … 1 right; far 0 close … 1 across the room (less speaker, more echo). */
  place(pan: number, far: number): void;
}

export const RADIO = callRust<{ lowCutHz: number; presenceHz: number; presenceDb: number; highCutHz: number; drive: number; hissDb: number; roomSeconds: number; roomDb: number }>("audio.dsp.radio", []);

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
  clip.curve = audioSamples("clip", 1024, RADIO.drive, 0);
  clip.oversample = "2x";
  const trim = ctx.createGain();
  trim.gain.value = callRust("audio.dsp.radioTrim", [RADIO.drive]);

  const speaker = ctx.createGain();
  lowCut.connect(presence).connect(highCut).connect(clip).connect(trim).connect(speaker);

  const out = ctx.createGain();
  const panner = ctx.createStereoPanner();
  out.connect(panner);
  const direct = ctx.createGain();
  speaker.connect(direct).connect(out);
  const room = ctx.createConvolver();
  room.buffer = roomImpulse(ctx, RADIO.roomSeconds);
  const roomLevel = ctx.createGain();
  roomLevel.gain.value = callRust("audio.dsp.dbToGain", [RADIO.roomDb]);
  speaker.connect(room).connect(roomLevel).connect(out);

  const hiss = ctx.createBufferSource();
  hiss.buffer = noiseBuffer(ctx, 2);
  hiss.loop = true;
  const hissBand = ctx.createBiquadFilter();
  hissBand.type = "bandpass";
  hissBand.frequency.value = 2500;
  hissBand.Q.value = 0.7;
  const hissGain = ctx.createGain();
  hissGain.gain.value = 0;
  hiss.connect(hissBand).connect(hissGain).connect(speaker);
  hiss.start();

  return {
    input: lowCut,
    output: panner,
    place(pan, far) {
      const t = ctx.currentTime;
      panner.pan.setTargetAtTime(pan, t, 0.3);
      const levels = callRust<{ direct: number; room: number }>("audio.dsp.radioPlace", [far]);
      direct.gain.setTargetAtTime(levels.direct, t, 0.3);
      roomLevel.gain.setTargetAtTime(levels.room, t, 0.3);
    },
    setHiss(on) {
      const target = on ? callRust<number>("audio.dsp.dbToGain", [RADIO.hissDb]) : 0;
      hissGain.gain.setTargetAtTime(target, ctx.currentTime, 0.08);
    },
  };
}

/** Sample arrays are generated in Rust and copied into WebAudio buffers. */
function roomImpulse(ctx: BaseAudioContext, seconds: number): AudioBuffer {
  const rate = ctx.sampleRate;
  const samples = audioSamples("room", rate, seconds, (Math.random() * 0xffffffff) >>> 0);
  const length = samples.length / 2;
  const buffer = ctx.createBuffer(2, length, rate);
  buffer.copyToChannel(samples.subarray(0, length), 0);
  buffer.copyToChannel(samples.subarray(length), 1);
  return buffer;
}
function noiseBuffer(ctx: BaseAudioContext, seconds: number): AudioBuffer {
  const samples = audioSamples("noise", ctx.sampleRate, seconds, (Math.random() * 0xffffffff) >>> 0);
  const buffer = ctx.createBuffer(1, samples.length, ctx.sampleRate);
  buffer.copyToChannel(samples, 0);
  return buffer;
}
