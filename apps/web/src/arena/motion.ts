// What a figure's body does: stands ready with its weapon, walks, and attacks on time. The clips come
// with the Persona (mocap of Art's reference videos, anim/scripts/apply_mocap.py): a ready stance that
// loops and an attack for each weapon. They play on two layers of one mixer, the legs (the pelvis and
// down) and the rest of the body, so a figure can walk while it raises its gun.

import * as THREE from "three";
import type { Held } from "./weapons.ts";

export type Stance = "pistol" | "rifle" | "shotgun" | "sniper";
export type Attack = Stance | "knife" | "grenade";

const READY: Record<Stance, string> = { pistol: "ready_pistol", rifle: "ready_rifle", shotgun: "ready_shotgun", sniper: "ready_shotgun" };

/** Each attack's clip, its speed, and the moment in it (seconds at normal speed) that must meet the sound:
 *  the shot, the knife reaching the enemy, the grenade leaving the hand. */
const ATTACKS: Record<Attack, { clip: string; speed: number; mark: number; hold?: Held }> = {
  pistol: { clip: "fire_pistol", speed: 1.15, mark: 0.625 },
  rifle: { clip: "fire_rifle", speed: 1, mark: 0.333 },
  shotgun: { clip: "fire_shotgun", speed: 1.25, mark: 0.333 },
  sniper: { clip: "fire_shotgun", speed: 1, mark: 0.333 }, // the bolt works like the pump, slower
  knife: { clip: "attack_knife", speed: 1.25, mark: 0.958, hold: "knife" },
  grenade: { clip: "grenade_throw", speed: 1.4, mark: 1.25 },
};

/** Tracks that move the legs: the pelvis and everything below it (UE Mannequin and Mixamo names). */
const LEGS = /^(root|pelvis|thigh_|calf_|foot_|ball_|mixamorigHips|mixamorig(Left|Right)(UpLeg|Leg|Foot|ToeBase|Toe_End))/;

type Layer = "body" | "legs";

export class Motion {
  private layers: Record<Layer, Map<string, THREE.AnimationAction>> = { body: new Map(), legs: new Map() };
  private cur: Record<Layer, THREE.AnimationAction | null> = { body: null, legs: null };
  private stance: Stance = "pistol";
  private walking = false;
  private pending: { in: number; kind: Attack } | null = null;
  private attacking: THREE.AnimationAction | null = null;
  /** Called when the hand should show something else (the knife for a stab, then the gun again). */
  onHold: ((what: Held) => void) | null = null;

  constructor(mixer: THREE.AnimationMixer, clips: THREE.AnimationClip[]) {
    for (const c of clips) {
      const legs = c.tracks.filter((t) => LEGS.test(t.name)), body = c.tracks.filter((t) => !LEGS.test(t.name));
      this.layers.legs.set(c.name, mixer.clipAction(new THREE.AnimationClip(`${c.name}.legs`, c.duration, legs)));
      this.layers.body.set(c.name, mixer.clipAction(new THREE.AnimationClip(`${c.name}.body`, c.duration, body)));
    }
    mixer.addEventListener("finished", (e) => { if (e.action === this.attacking) this.done(); });
  }

  /** The weapon in hand: its ready stance, unless the figure is busy attacking. */
  arm(stance: Stance): void {
    if (stance === this.stance) return;
    this.stance = stance;
    if (!this.attacking) this.to("body", this.ready(), 0.3);
    if (!this.walking && !this.attacking) this.to("legs", this.ready(), 0.3);
    this.onHold?.(stance);
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
    const wait = inSeconds - a.mark / a.speed;
    if (wait > 0) this.pending = { in: wait, kind };
    else this.start(kind, -wait * a.speed);
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
    this.pending = null; this.attacking = null; this.walking = false;
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
    if (!p) return;
    p.in -= dt;
    if (p.in <= 0) { this.pending = null; this.start(p.kind, -p.in * ATTACKS[p.kind].speed); }
  }

  private ready(): string {
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
    this.attacking = act;
    if (!this.walking) { this.to("legs", a.clip, 0.12, a.speed, true); this.sync("legs", "body"); }
    if (a.hold) this.onHold?.(a.hold);
  }

  private done(): void {
    this.attacking = null;
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
