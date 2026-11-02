// Rust owns clip scheduling, mark/release timing and pose filtering.
// This adapter uploads the resulting commands to Three's AnimationMixer.
import * as THREE from "three";
import type { Held } from "./weapons.ts";
import { callRust, motionCompute } from "../../../../packages/runtime/src/index.ts";
export type Stance = "pistol" | "rifle" | "shotgun" | "sniper";
export type Attack = Stance | "knife" | "grenade";
type Layer = "body" | "legs";
const LEGS = /^(root|pelvis|thigh_|calf_|foot_|ball_|mixamorigHips|mixamorig(Left|Right)(UpLeg|Leg|Foot|ToeBase|Toe_End))/;
const PELVIS = /^(pelvis|mixamorigHips)\.quaternion$/;
interface Timeline { stance: Stance; walking: boolean; kind: Attack | null; planted: string | null; body: string | null; legs: string | null }
interface Command { op: string; layer?: Layer; name?: string; fade?: number; speed?: number; once?: boolean; reset?: boolean; value?: number | Held | null }

export class Motion {
  private layers: Record<Layer, Map<string, THREE.AnimationAction>> = { body: new Map(), legs: new Map() };
  private cur: Record<Layer, THREE.AnimationAction | null> = { body: null, legs: null };
  private state!: Timeline;
  private pelvisOf = new Map<string, THREE.Interpolant>();
  private pelvis: THREE.Object3D | null;
  private spine: THREE.Object3D | null;
  onMark: ((kind: Attack) => void) | null = null;
  onHold: ((what: Held | null) => void) | null = null;
  readonly stances = ["ready_pistol", "ready_rifle", "ready_shotgun"];

  constructor(mixer: THREE.AnimationMixer, clips: THREE.AnimationClip[]) {
    for (const c of clips) if (/^ready_/.test(c.name)) soften(c);
    for (const c of clips) {
      this.layers.legs.set(c.name, mixer.clipAction(new THREE.AnimationClip(c.name + ".legs", c.duration, c.tracks.filter(t => LEGS.test(t.name)))));
      this.layers.body.set(c.name, mixer.clipAction(new THREE.AnimationClip(c.name + ".body", c.duration, c.tracks.filter(t => !LEGS.test(t.name)))));
      const turn = c.tracks.find(t => PELVIS.test(t.name));
      if (turn) this.pelvisOf.set(c.name, new THREE.QuaternionLinearInterpolant(turn.times, turn.values, 4, new Float32Array(4)));
    }
    const root = mixer.getRoot() as THREE.Object3D;
    this.pelvis = root.getObjectByName("pelvis") ?? root.getObjectByName("mixamorigHips") ?? null;
    this.spine = root.getObjectByName("spine_01") ?? root.getObjectByName("mixamorigSpine") ?? null;
    this.send("create", { durations: Object.fromEntries(clips.map(c => [c.name, c.duration])) });
    mixer.addEventListener("finished", e => { if (this.state.kind && e.action === this.cur.body) this.send("done"); });
  }
  settle(): void {
    if (!this.pelvis || !this.spine) return;
    const input = [...this.pelvis.quaternion.toArray(), ...this.spine.quaternion.toArray()];
    for (const [name, act] of this.layers.body) {
      const weight = act.isRunning() ? act.getEffectiveWeight() : 0, turn = this.pelvisOf.get(name);
      if (weight > 0 && turn) input.push(...Array.from(turn.evaluate(act.time)), weight);
    }
    this.spine.quaternion.fromArray(motionCompute("settle", new Float64Array(input)));
  }
  arm(stance: Stance): void { this.send("arm", { stance }); }
  get attackKind(): Attack | null { return this.state.kind; }
  get pistolReady(): number { const a = this.layers.body.get("ready_pistol"); return a?.isRunning() ? a.getEffectiveWeight() : 0; }
  get planted(): string | null { return this.state.planted; }
  walk(clip: "walk" | "run", at: number): void { this.send("walk", { clip, at }); }
  get gait(): "walk" | "run" | null { return this.state.walking ? this.state.legs as "walk" | "run" : null; }
  pace(speed: number): void { this.send("pace", { speed }); }
  stand(fade = .35): void { this.send("stand", { fade }); }
  attack(kind: Attack, inSeconds: number): void { this.send("attack", { kind, inSeconds }); }
  fall(): void { this.send("fall"); }
  reset(phase: number): void { this.send("reset", { phase }); }
  update(dt: number): void { this.send("update", { dt, time: this.cur.body?.time ?? 0 }); }

  private send(event: string, data: Record<string, unknown> = {}): void {
    const result = callRust<{ state: Timeline; commands: Command[] }>("motion.timeline", { state: this.state, event, ...data });
    this.state = result.state;
    for (const c of result.commands) {
      if (c.op === "hold") { this.onHold?.(c.value as Held | null); continue; }
      if (c.op === "mark") { this.onMark?.(c.value as Attack); continue; }
      if (c.op === "sync") { if (this.cur.body && this.cur.legs) this.cur.legs.time = this.cur.body.time; continue; }
      if (c.op === "stopAll") { for (const layer of ["body", "legs"] as const) { for (const a of this.layers[layer].values()) a.stop(); this.cur[layer] = null; } continue; }
      const layer = c.layer!;
      if (c.op === "speed") { if (this.cur[layer]) this.cur[layer]!.timeScale = c.value as number; continue; }
      if (c.op === "time") { if (this.cur[layer]) this.cur[layer]!.time = c.value as number; continue; }
      const action = this.layers[layer].get(c.name!);
      if (!action) continue;
      action.timeScale = c.speed!;
      if (c.reset) {
        const previous = this.cur[layer];
        action.reset().setLoop(c.once ? THREE.LoopOnce : THREE.LoopRepeat, Infinity);
        action.clampWhenFinished = !!c.once;
        action.setEffectiveWeight(1).fadeIn(c.fade!).play();
        if (previous && previous !== action) previous.fadeOut(c.fade!);
        this.cur[layer] = action;
      }
    }
  }
}

/** The pose the mixer gave the bones, put back before the mixer runs again. three.js only writes a bone whose
 *  value changed (a clip held on its last frame, a frame with no time), so whatever was turned on top of the
 *  animation last frame (settle(), the feet, a hit, the fall) would otherwise be turned once more. */
export class Pose {
  private bones: { o: THREE.Object3D; p: THREE.Vector3; q: THREE.Quaternion }[] = [];

  constructor(model: THREE.Object3D) {
    model.traverse((o) => { if ((o as THREE.Bone).isBone) this.bones.push({ o, p: o.position.clone(), q: o.quaternion.clone() }); });
  }

  /** Right after the mixer. */
  save(): void { for (const b of this.bones) { b.p.copy(b.o.position); b.q.copy(b.o.quaternion); } }

  /** Right before the mixer. */
  restore(): void { for (const b of this.bones) { b.o.position.copy(b.p); b.o.quaternion.copy(b.q); } }
}

/** Each track is filtered as one packed buffer in Rust. */
function soften(clip: THREE.AnimationClip, keep = .3, sigma = 6): void {
  for (const track of clip.tracks) {
    const input = new Float64Array(4 + track.values.length);
    input.set([track.getValueSize(), keep, sigma, track instanceof THREE.QuaternionKeyframeTrack ? 1 : 0]);
    input.set(track.values, 4);
    track.values.set(motionCompute("soften", input));
  }
}
