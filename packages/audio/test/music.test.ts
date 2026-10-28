import assert from "node:assert/strict";
import { test } from "node:test";
import { FIGHT_PLAYLIST, RADIO_PLAYLIST, type SoundId, type TrackId } from "../src/bank.ts";
import { MusicDirector, type Channel, type MusicDeck } from "../src/music.ts";
import { seeded } from "./seeded.ts";

const isStation = (t: TrackId | undefined) => t !== undefined && (RADIO_PLAYLIST as readonly TrackId[]).includes(t);

class FakeDeck implements MusicDeck {
  log: string[] = [];
  playing: Partial<Record<Channel, TrackId>> = {};
  paused: Partial<Record<Channel, boolean>> = {};

  play(ch: Channel, track: TrackId) {
    this.log.push(`play ${ch} ${track}`);
    this.playing[ch] = track;
    this.paused[ch] = false;
  }
  pause(ch: Channel) {
    this.log.push(`pause ${ch}`);
    this.paused[ch] = true;
  }
  resume(ch: Channel) {
    this.log.push(`resume ${ch}`);
    this.paused[ch] = false;
  }
  stop(ch: Channel) {
    this.log.push(`stop ${ch}`);
    delete this.playing[ch];
  }
  cue(sound: SoundId) {
    this.log.push(`cue ${sound}`);
  }
  /** Is this channel audible right now? */
  on(ch: Channel) {
    return this.playing[ch] !== undefined && !this.paused[ch];
  }
}

test("a fight plays one of Art's three fight tracks, at random", () => {
  const deck = new FakeDeck();
  const music = new MusicDirector(deck, seeded("fights"));
  const seen = new Set<TrackId>();
  let last: TrackId | undefined;
  for (let i = 0; i < 60; i++) {
    music.enterFight();
    const t = deck.playing.fight!;
    assert.ok(FIGHT_PLAYLIST.includes(t), t);
    assert.notEqual(t, last);
    seen.add(t);
    last = t;
    music.enterMenu();
  }
  assert.equal(seen.size, 3);
});

test("menus play the other tracks from the radio, never in a fight", () => {
  const deck = new FakeDeck();
  const music = new MusicDirector(deck, seeded("radio"));
  music.enterMenu();
  assert.ok(deck.on("radio"));
  assert.ok(isStation(deck.playing.radio));
  assert.ok(!deck.on("fight"));
  assert.ok(deck.log.includes("cue radio.on"));

  music.enterFight();
  assert.ok(!deck.on("radio"));
  assert.ok(deck.on("fight"));
  assert.ok(deck.log.includes("cue radio.off"));
});

test("opening the menu pauses the fight track; closing it picks up where it was", () => {
  const deck = new FakeDeck();
  const music = new MusicDirector(deck, seeded("pause"));
  music.enterFight();
  const track = deck.playing.fight;

  music.openMenu();
  assert.ok(!deck.on("fight"));
  assert.equal(deck.playing.fight, track, "paused, not stopped");
  assert.ok(deck.on("radio"));
  assert.equal(music.state.fightPaused, true);

  // a song ending behind the menu does not start the next one
  music.ended("fight");
  assert.equal(deck.playing.fight, track);

  music.closeMenu();
  assert.ok(deck.on("fight"));
  assert.equal(deck.playing.fight, track);
  assert.ok(!deck.on("radio"));
  assert.deepEqual(deck.log.slice(-3), ["cue radio.off", "pause radio", "resume fight"]);
});

test("the radio keeps its station between visits and turns the knob when a song ends", () => {
  const deck = new FakeDeck();
  const music = new MusicDirector(deck, seeded("station"));
  music.enterMenu();
  const station = deck.playing.radio;
  music.enterFight();
  music.openMenu();
  assert.equal(deck.playing.radio, station);
  assert.equal(deck.log.at(-1), "resume radio");

  music.ended("radio");
  assert.notEqual(deck.playing.radio, station);
  assert.ok(deck.log.includes("cue radio.tune"));
});

test("when a fight track ends, another one starts", () => {
  const deck = new FakeDeck();
  const music = new MusicDirector(deck, seeded("next"));
  music.enterFight();
  const first = deck.playing.fight;
  music.ended("fight");
  assert.notEqual(deck.playing.fight, first);
  assert.ok(FIGHT_PLAYLIST.includes(deck.playing.fight!));
});

test("menu calls outside a fight do nothing, silence stops everything", () => {
  const deck = new FakeDeck();
  const music = new MusicDirector(deck, seeded("quiet"));
  music.openMenu();
  music.closeMenu();
  assert.deepEqual(deck.log, []);
  music.enterFight();
  music.openMenu();
  music.silence();
  assert.ok(!deck.on("fight") && !deck.on("radio"));
  assert.equal(music.state.scene, "silent");
});

test("a station picked from the list plays at once, or when the radio comes on", () => {
  const deck = new FakeDeck();
  const music = new MusicDirector(deck, seeded("tune"));
  music.enterMenu();
  const other = RADIO_PLAYLIST.find((t) => t !== deck.playing.radio)!;
  music.tune(other);
  assert.equal(deck.playing.radio, other);
  assert.deepEqual(deck.log.slice(-2), ["cue radio.tune", `play radio ${other}`]);

  // the same station again changes nothing
  const n = deck.log.length;
  music.tune(other);
  assert.equal(deck.log.length, n);

  // picked while the radio is off: it starts from the top when the radio comes back
  music.enterFight();
  const next = RADIO_PLAYLIST.find((t) => t !== other)!;
  music.tune(next);
  assert.ok(!deck.on("radio"));
  music.openMenu();
  assert.equal(deck.playing.radio, next);
  assert.ok(deck.on("radio"));
  assert.equal(deck.log.at(-1), `play radio ${next}`);
  assert.ok(deck.on("radio") && !deck.on("fight"));
});
