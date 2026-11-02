/// <reference lib="dom" />
import { RustRadio, callRust } from "../../runtime/src/index.ts";
export interface RadioChain {
  input: AudioNode;
  output: AudioNode;
  setHiss(on: boolean): void;
  place(pan: number, far: number): void;
}
export const RADIO = callRust<{ lowCutHz: number; presenceHz: number; presenceDb: number; highCutHz: number; drive: number; hissDb: number; roomSeconds: number; roomDb: number }>("audio.dsp.radio", []);
export function createRadio(ctx: BaseAudioContext): RadioChain { return new RustRadio(ctx); }
