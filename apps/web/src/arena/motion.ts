// What a figure's body does: stands ready with its weapon, walks, and attacks on time. The clips come
// with the Persona (mocap of Art's reference videos, anim/scripts/apply_mocap.py): a ready stance that
// loops and an attack for each weapon. They play on two layers of one mixer, the legs (the pelvis and
// down) and the rest of the body, so a figure can walk while it raises its gun.

import * as THREE from "three";
import type { Held } from "./weapons.ts";

export type Stance = "pistol" | "rifle" | "shotgun" | "sniper";
export type Attack = Stance | "knife" | "grenade";

const READY: Record<Stance, string> = { pistol: "ready_pistol", rifle: "ready_rifle", shotgun: "ready_shotgun", sniper: "ready_shotgun" };
/** Seen from our own eyes the pistol's low ready is under the chin, out of the picture: in first person the
 *  pistol is held out the way it is at the shot (fire_pistol at its mark), and a shot starts from there. */
const AIMED: Partial<Record<Stance, { clip: string; at: number; name: string }>> = { pistol: { clip: "fire_pistol", at: 0.625, name: "aim_pistol" } };

/** Each attack's clip, its speed, and the moment in it (seconds at normal speed) that must meet the sound:
 *  the shot, the knife reaching the enemy, the grenade leaving the hand. `hold` is what the hand shows
 *  during it, `release` empties the hand at the mark (the grenade flies), `end` cuts a long clip short
 *  (seconds at normal speed) and goes back to the stance. */
const ATTACKS: Record<Attack, { clip: string; speed: number; mark: number; hold?: Held; release?: boolean; end?: number; aimFrom?: number }> = {
  pistol: { clip: "fire_pistol", speed: 1.15, mark: 0.625, aimFrom: 0.5 },
  rifle: { clip: "fire_rifle", speed: 1, mark: 0.333 },
  shotgun: { clip: "fire_shotgun", speed: 1.25, mark: 0.333 },
  sniper: { clip: "fire_shotgun", speed: 1, mark: 0.333 }, // the bolt works like the pump, slower
  knife: { clip: "attack_knife", speed: 1.25, mark: 0.958, hold: "knife" },
  // Art's Tripo throw (24 fps): the hand leaves the grenade at its frame 30, back by frame 46, then a second swing we skip
  grenade: { clip: "grenade_throw", speed: 1.4, mark: 1.208, hold: "grenade", release: true, end: 1.875 },
};

/** Tracks that move the legs: the pelvis and everything below it (UE Mannequin and Mixamo names). */
const LEGS = /^(root|pelvis|thigh_|calf_|foot_|ball_|mixamorigHips|mixamorig(Left|Right)(UpLeg|Leg|Foot|ToeBase|Toe_End))/;
/** The pelvis turn and the first spine bone, which carries the whole upper body. */
const PELVIS = /^(pelvis|mixamorigHips)\.quaternion$/;
const SPINE = ["spine_01", "mixamorigSpine"];
const qa = new THREE.Quaternion(), qb = new THREE.Quaternion();

type Layer = "body" | "legs";

export class Motion {
  private layers: Record<Layer, Map<string, THREE.AnimationAction>> = { body: new Map(), legs: new Map() };
  private cur: Record<Layer, THREE.AnimationAction | null> = { body: null, legs: null };
  private stance: Stance = "pistol";
  private walking = false;
  private pending: { in: number; kind: Attack; from: number } | null = null;
  /** First person: the stances in AIMED are held out in front. */
  private aimed = false;
  private attacking: THREE.AnimationAction | null = null;
  private kind: Attack | null = null;
  private released = false;
  /** Each clip's pelvis turn, to keep the upper body on its own clip while the legs play another. */
  private pelvisOf = new Map<string, THREE.Interpolant>();
  private pelvis: THREE.Object3D | null;
  private spine: THREE.Object3D | null;
  /** Called when the hand should show something else (the knife for a stab, then the gun again; nothing once a grenade is thrown). */
  onHold: ((what: Held | null) => void) | null = null;

  constructor(mixer: THREE.AnimationMixer, clips: THREE.AnimationClip[]) {
    for (const c of clips) {
      const legs = c.tracks.filter((t) => LEGS.test(t.name)), body = c.tracks.filter((t) => !LEGS.test(t.name));
      this.layers.legs.set(c.name, mixer.clipAction(new THREE.AnimationClip(`${c.name}.legs`, c.duration, legs)));
      this.layers.body.set(c.name, mixer.clipAction(new THREE.AnimationClip(`${c.name}.body`, c.duration, body)));
      const turn = c.tracks.find((t) => PELVIS.test(t.name));
      if (turn) this.pelvisOf.set(c.name, new THREE.QuaternionLinearInterpolant(turn.times, turn.values, 4, new Float32Array(4)));
    }
    for (const a of Object.values(AIMED)) {
      const c = clips.find((c) => c.name === a.clip);
      if (!c) continue;
      const held = poseAt(c, a.at, a.name);
      this.layers.legs.set(a.name, mixer.clipAction(new THREE.AnimationClip(`${a.name}.legs`, 1, held.tracks.filter((t) => LEGS.test(t.name)))));
      this.layers.body.set(a.name, mixer.clipAction(new THREE.AnimationClip(`${a.name}.body`, 1, held.tracks.filter((t) => !LEGS.test(t.name)))));
      const turn = held.tracks.find((t) => PELVIS.test(t.name));
      if (turn) this.pelvisOf.set(a.name, new THREE.QuaternionLinearInterpolant(turn.times, turn.values, 4, new Float32Array(4)));
    }
    const root = mixer.getRoot() as THREE.Object3D;
    this.pelvis = root.getObjectByName("pelvis") ?? root.getObjectByName("mixamorigHips") ?? null;
    this.spine = SPINE.map((n) => root.getObjectByName(n)).find((o) => o) ?? null;
    mixer.addEventListener("finished", (e) => { if (e.action === this.attacking) this.done(); });
  }

  /** After the mixer: the upper body keeps the turn its own clip gives the pelvis. The legs layer turns the
   *  pelvis with every stride of a run, and a stance stands half turned (a rifle is held bladed), so without
   *  this the spine, the arms and the gun swing with the hips. */
  settle(): void {
    const pelvis = this.pelvis, spine = this.spine;
    if (!pelvis || !spine) return;
    qa.set(0, 0, 0, 0);
    for (const [name, act] of this.layers.body) {
      const w = act.isRunning() ? act.getEffectiveWeight() : 0, turn = this.pelvisOf.get(name);
      if (w <= 0 || !turn) continue;
      qb.fromArray(turn.evaluate(act.time) as unknown as number[]);
      if (qa.dot(qb) < 0) qb.set(-qb.x, -qb.y, -qb.z, -qb.w);
      qa.set(qa.x + qb.x * w, qa.y + qb.y * w, qa.z + qb.z * w, qa.w + qb.w * w);
    }
    if (qa.lengthSq() < 1e-8) return;
    qa.normalize();
    // spine' = pelvis⁻¹ · (the body clip's pelvis) · spine
    spine.quaternion.premultiply(qb.copy(pelvis.quaternion).invert().multiply(qa));
  }

  /** The weapon in hand: its ready stance, unless the figure is busy attacking. */
  arm(stance: Stance): void {
    if (stance === this.stance) return;
    this.stance = stance;
    if (!this.attacking) this.to("body", this.ready(), 0.3);
    if (!this.walking && !this.attacking) this.to("legs", this.ready(), 0.3);
    this.onHold?.(stance);
  }

  /** The attack under way, if any. */
  get attackKind(): Attack | null { return this.kind; }

  /** First person: holds the pistol out in front instead of low, where our own eyes could not see it. */
  aim(on: boolean): void {
    if (on === this.aimed) return;
    this.aimed = on;
    if (!this.attacking) this.to("body", this.ready(), 0.3);
    if (!this.walking && !this.attacking) this.to("legs", this.ready(), 0.3);
  }

  /** Walks (or runs) on the spot while the arena carries the figure; a negative speed walks backwards. */
  walk(clip: "walk" | "run", speed: number): void {
    this.walking = true;
    this.to("legs", clip, 0.15, speed);
  }

  /** Stops walking: the legs go back to the stance, in step with the body. */
  stand(fade = 0.35): void {
    this.walking = false;
    if (this.attacking) { this.to("legs", this.attackClip(), fade); this.sync("legs", "body"); return; }
    this.to("legs", this.ready(), fade);
    this.sync("legs", "body");
  }

  /** Attacks so that the clip's moment (the shot, the stab, the throw) comes `inSeconds` from now. */
  attack(kind: Attack, inSeconds: number): void {
    const a = ATTACKS[kind];
    if (!this.layers.body.has(a.clip)) return; // this figure has no such clip: it stays in its stance
    // held out already, a shot starts near its mark instead of raising the gun from low
    const from = this.aimed && a.aimFrom !== undefined ? a.aimFrom : 0;
    const wait = inSeconds - (a.mark - from) / a.speed;
    if (wait > 0) this.pending = { in: wait, kind, from };
    else this.start(kind, from - wait * a.speed);
  }

  /** Falls: whatever it was doing slows to a stop while the arena tips it over. */
  fall(): void {
    this.pending = null;
    for (const l of ["body", "legs"] as Layer[]) {
      const c = this.cur[l];
      if (c) c.timeScale = 0.2;
    }
  }

  /** Back to the stance at once; `phase` (0..1) staggers figures so they never breathe in step. */
  reset(phase: number): void {
    this.pending = null; this.attacking = null; this.kind = null; this.walking = false;
    for (const l of ["body", "legs"] as Layer[]) {
      for (const a of this.layers[l].values()) a.stop();
      this.cur[l] = null;
      const a = this.to(l, this.ready(), 0);
      if (a) a.time = phase * a.getClip().duration;
    }
    this.onHold?.(this.stance);
  }

  update(dt: number): void {
    const p = this.pending;
    if (p) {
      p.in -= dt;
      if (p.in <= 0) { this.pending = null; this.start(p.kind, p.from - p.in * ATTACKS[p.kind].speed); }
    }
    const act = this.attacking, a = this.kind && ATTACKS[this.kind];
    if (!act || !a) return;
    if (a.release && !this.released && act.time >= a.mark) { this.released = true; this.onHold?.(null); }
    if (a.end !== undefined && act.time >= a.end) this.done();
  }

  private ready(): string {
    const aim = this.aimed && AIMED[this.stance];
    if (aim && this.layers.body.has(aim.name)) return aim.name;
    const r = READY[this.stance];
    return this.layers.body.has(r) ? r : "idle";
  }

  private attackClip(): string {
    return this.attacking!.getClip().name.replace(/\.body$/, "");
  }

  private start(kind: Attack, at: number): void {
    const a = ATTACKS[kind];
    const act = this.to("body", a.clip, 0.12, a.speed, true);
    if (!act) return;
    act.time = Math.min(at, act.getClip().duration);
    this.attacking = act; this.kind = kind; this.released = false;
    if (!this.walking) { this.to("legs", a.clip, 0.12, a.speed, true); this.sync("legs", "body"); }
    if (a.hold) this.onHold?.(a.hold);
  }

  private done(): void {
    this.attacking = null; this.kind = null;
    this.to("body", this.ready(), 0.35);
    if (!this.walking) { this.to("legs", this.ready(), 0.35); this.sync("legs", "body"); }
    this.onHold?.(this.stance);
  }

  /** Crossfades a layer to a clip; a looping clip already playing just takes the new speed. */
  private to(layer: Layer, name: string, fade: number, speed = 1, once = false): THREE.AnimationAction | null {
    const a = this.layers[layer].get(name);
    if (!a) return null;
    const cur = this.cur[layer];
    a.timeScale = speed;
    if (cur === a && !once) return a;
    a.reset();
    a.setLoop(once ? THREE.LoopOnce : THREE.LoopRepeat, Infinity);
    a.clampWhenFinished = once;
    a.setEffectiveWeight(1).fadeIn(fade).play();
    if (cur && cur !== a) cur.fadeOut(fade);
    this.cur[layer] = a;
    return a;
  }

  /** Puts one layer at the same moment of the same clip as the other, so legs and body move as one. */
  private sync(layer: Layer, from: Layer): void {
    const a = this.cur[layer], b = this.cur[from];
    if (a && b && a.getClip().name.split(".")[0] === b.getClip().name.split(".")[0]) a.time = b.time;
  }
}

/** A still clip of one moment of a clip: every track holds its value at `t` (seconds). */
function poseAt(clip: THREE.AnimationClip, t: number, name: string): THREE.AnimationClip {
  const tracks = clip.tracks.map((tr) => {
    const n = tr.getValueSize(), at = new Float32Array(n);
    const interp = tr instanceof THREE.QuaternionKeyframeTrack
      ? new THREE.QuaternionLinearInterpolant(tr.times, tr.values, n, at)
      : new THREE.LinearInterpolant(tr.times, tr.values, n, at);
    const v = Array.from(interp.evaluate(t) as ArrayLike<number>);
    const Track = tr.constructor as new (name: string, times: number[], values: number[]) => THREE.KeyframeTrack;
    return new Track(tr.name, [0, 1], [...v, ...v]);
  });
  return new THREE.AnimationClip(name, 1, tracks);
}
