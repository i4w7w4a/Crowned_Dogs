use crate::{data, num, text, truth};
use serde_json::{json, Value};

fn pick_other(n: usize, last: Option<usize>, random: f64) -> usize {
    if n <= 1 {
        return 0;
    }
    match last.filter(|x| *x < n) {
        None => (random * n as f64).floor() as usize,
        Some(last) => {
            let i = (random * (n - 1) as f64).floor() as usize;
            if i >= last {
                i + 1
            } else {
                i
            }
        }
    }
}
struct Script {
    cues: Vec<Value>,
    viewer: usize,
    far: f64,
}
impl Script {
    fn add(&mut self, sound: impl Into<String>, at: f64, side: usize, extra: f64) {
        let round = |v: f64| (v * 1000.0 + 0.5).floor() / 1000.0;
        self.cues.push(json!({"sound":sound.into(),"at":round(at),"gainDb":round(if side==self.viewer{extra}else{self.far+extra}),"pan":if side==self.viewer{-0.3}else{0.15}}));
    }
    fn steps(&mut self, n: usize, gap: f64, side: usize) {
        for i in 0..n {
            self.add("step", i as f64 * gap, side, 0.0);
        }
        self.add("gear.rustle", 0.0, side, 0.0);
    }
    fn movement(&mut self, a: &Value, side: usize) {
        match text(&a["type"]) {
            "run" => self.steps(2, 0.27, side),
            "dash" => {
                self.add("dash.whoosh", 0.0, side, 0.0);
                self.steps(3, 0.19, side);
            }
            "flee" => {
                self.add("dash.whoosh", 0.0, side, 0.0);
                self.steps(3, 0.27, side);
            }
            "shoot" | "knife" | "grenade" | "swap" => {
                if num(&a["move"]) != 0.0 {
                    self.steps(1, 0.0, side);
                }
            }
            _ => {}
        }
    }
    fn hit(&mut self, a: &Value, target: usize, at: f64, ctx: &Value) {
        let zone = if a["zone"].is_null() {
            "body"
        } else {
            text(&a["zone"])
        };
        if truth(&a["blocked"]) {
            self.add(
                if truth(&a["shieldBroken"]) {
                    "shield.break"
                } else {
                    "shield.block"
                },
                at,
                target,
                0.0,
            );
            if !truth(&a["shieldBroken"]) {
                return;
            }
        }
        let armor = ctx["armor"][target]
            .get(zone)
            .cloned()
            .unwrap_or_else(|| game_core::constants()["DEFAULT_ARMOR"][zone].clone());
        let layer = if truth(&a["blocked"]) { -6.0 } else { 0.0 };
        self.add(
            if num(&armor) > 0.0 {
                "hit.armor"
            } else {
                "hit.suit"
            },
            at,
            target,
            layer,
        );
        if zone == "head" {
            self.add("hit.head", at, target, layer);
        }
        if truth(&a["crit"]) {
            self.add("hit.crit", at + 0.02, target, 0.0);
        }
    }
    fn attack(&mut self, a: &Value, side: usize, t: f64, ctx: &Value) -> f64 {
        let ours = side == self.viewer;
        let target = 1 - side;
        match text(&a["weapon"]) {
            "grenade" => {
                self.add("grenade.pin", t - 0.3, side, -2.0);
                self.add("grenade.throw", t, side, 0.0);
                self.add("grenade.bounce", t + 0.45, target, 0.0);
                self.add("grenade.blast", t + 0.9, target, 0.0);
                if truth(&a["hit"]) {
                    self.add("hit.suit", t + 0.95, target, -4.0);
                }
                t + 0.9
            }
            "knife" => {
                self.add("knife.swing", t, side, 0.0);
                if !truth(&a["hit"]) {
                    return t;
                }
                let at = t + 0.18;
                self.add(
                    if truth(&a["blocked"]) {
                        "knife.blocked"
                    } else {
                        "knife.hit"
                    },
                    at,
                    target,
                    0.0,
                );
                if truth(&a["crit"]) {
                    self.add("hit.crit", at, target, 0.0);
                }
                at
            }
            weapon => {
                self.add(
                    format!("shot.{weapon}.{}", if ours { "near" } else { "far" }),
                    t,
                    side,
                    0.0,
                );
                if ours && weapon != "shotgun" {
                    self.add("casing", t + 0.55, side, 0.0);
                }
                let at = t + if ours { 0.06 } else { 0.02 };
                if !truth(&a["hit"]) {
                    self.add(
                        if ours { "miss.far" } else { "miss.near" },
                        at + 0.04,
                        target,
                        0.0,
                    );
                } else {
                    self.hit(a, target, at, ctx);
                }
                at
            }
        }
    }
}
fn cues_for_turn(report: &Value, ctx: &Value) -> Value {
    let me = ctx["viewer"].as_u64().unwrap_or(0) as usize;
    let sides = [me, 1 - me];
    let far = -6.0f64.min(0.0f64.max(num(&report["distanceAfter"]) - 1.0) * 0.6);
    let mut s = Script {
        cues: vec![],
        viewer: me,
        far,
    };
    for side in sides {
        s.movement(&report["actions"][side], side);
    }
    let mut last = 0.85f64;
    for side in sides {
        let t = if side == me { 0.85 } else { 1.45 };
        let drawn = text(&ctx["weapons"][side]);
        if report["actions"][side]["type"] == "swap" && !drawn.is_empty() {
            s.add(format!("equip.{drawn}"), t, side, 0.0);
        }
        if !report["attacks"][side].is_null() {
            last = last.max(s.attack(&report["attacks"][side], side, t, ctx));
        }
    }
    for side in sides {
        if num(&report["hp"][side]) <= 0.0 {
            let t = last + 0.45;
            s.add("body.fall", t, side, 0.0);
            s.add("persona.powerdown", t + 0.25, side, 0.0);
            if side == me {
                s.add("sync.lost", t + 0.6, side, 0.0);
            }
        }
    }
    s.cues
        .sort_by(|a, b| num(&a["at"]).total_cmp(&num(&b["at"])));
    json!(s.cues)
}
fn fight_sounds(ours: &Value, theirs: &Value) -> Value {
    let list = |v: &Value| {
        if v.is_string() {
            vec![v.clone()]
        } else {
            v.as_array().cloned().unwrap_or_default()
        }
    };
    let a = list(ours);
    let b = list(theirs);
    let mut out = Vec::<String>::new();
    for w in &a {
        out.push(format!("shot.{}.near", text(w)));
    }
    for w in &b {
        out.push(format!("shot.{}.far", text(w)));
    }
    out.push("casing".into());
    for guns in [&a, &b] {
        if guns.len() > 1 {
            for w in guns {
                out.push(format!("equip.{}", text(w)));
            }
        }
    }
    out.extend(
        [
            "knife.swing",
            "knife.hit",
            "knife.blocked",
            "hit.armor",
            "hit.suit",
            "hit.head",
            "hit.crit",
            "shield.block",
            "shield.break",
            "miss.near",
            "miss.far",
            "grenade.pin",
            "grenade.throw",
            "grenade.bounce",
            "grenade.blast",
            "step",
            "dash.whoosh",
            "gear.rustle",
            "body.fall",
            "persona.powerdown",
            "sync.lost",
        ]
        .map(String::from),
    );
    let mut unique = vec![];
    for item in out {
        if !unique.contains(&item) {
            unique.push(item);
        }
    }
    json!(unique)
}

fn music_new() -> Value {
    json!({"scene":"silent","menuOpen":false,"radioOn":false,"retuned":false,"fightPaused":false,"fightIndex":-1,"radioIndex":-1,"radioPlaylist":data()["RADIO_PLAYLIST"]})
}
fn music_random(s: &Value, method: &str, arg: &Value) -> bool {
    let radio_random = s["radioPlaylist"].as_array().is_some_and(|p| p.len() > 1);
    match method {
        "enterFight" => true,
        "enterMenu" => !truth(&s["radioOn"]) && num(&s["radioIndex"]) < 0.0 && radio_random,
        "openMenu" => {
            s["scene"] == "fight"
                && !truth(&s["menuOpen"])
                && !truth(&s["radioOn"])
                && num(&s["radioIndex"]) < 0.0
                && radio_random
        }
        "ended" => {
            if arg == "fight" {
                s["scene"] == "fight" && !truth(&s["fightPaused"])
            } else {
                arg == "radio" && truth(&s["radioOn"]) && radio_random
            }
        }
        "nextStation" => {
            truth(&s["radioOn"]) && s["radioPlaylist"].as_array().is_some_and(|p| p.len() > 1)
        }
        "playlist" => {
            truth(&s["radioOn"])
                && arg.as_array().is_some_and(|p| {
                    p.len() > 1
                        && !p.contains(
                            &s["radioPlaylist"]
                                [s["radioIndex"].as_u64().unwrap_or(u64::MAX) as usize],
                        )
                })
        }
        _ => false,
    }
}
fn music_state(s: &Value) -> Value {
    let mut out =
        json!({"scene":s["scene"],"menuOpen":s["menuOpen"],"fightPaused":s["fightPaused"]});
    if let Some(i) = s["fightIndex"].as_u64() {
        out["fightTrack"] = data()["FIGHT_PLAYLIST"][i as usize].clone();
    }
    if truth(&s["radioOn"]) {
        if let Some(i) = s["radioIndex"].as_u64() {
            out["radioTrack"] = s["radioPlaylist"][i as usize].clone();
        }
    }
    out
}
fn music_step(mut s: Value, method: &str, arg: &Value, random: f64) -> Result<Value, String> {
    let mut ops = Vec::<Value>::new();
    let op = |ops: &mut Vec<Value>, method: &str, arg: &str| {
        ops.push(json!({"method":method,"args":[arg]}))
    };
    let play = |ops: &mut Vec<Value>, channel: &str, track: &Value| {
        ops.push(json!({"method":"play","args":[channel,track]}))
    };
    let next = |s: &mut Value, channel: &str| -> Value {
        let key = if channel == "fight" {
            "fightIndex"
        } else {
            "radioIndex"
        };
        let list = if channel == "fight" {
            data()["FIGHT_PLAYLIST"].clone()
        } else {
            s["radioPlaylist"].clone()
        };
        let i = pick_other(
            list.as_array().unwrap().len(),
            s[key].as_u64().map(|i| i as usize),
            random,
        );
        s[key] = json!(i);
        list[i].clone()
    };
    let down = |s: &mut Value, ops: &mut Vec<Value>| {
        if truth(&s["radioOn"]) {
            s["radioOn"] = json!(false);
            op(ops, "cue", "radio.off");
            op(ops, "pause", "radio");
        }
    };
    let up = |s: &mut Value, ops: &mut Vec<Value>| {
        if truth(&s["radioOn"]) {
            return;
        }
        s["radioOn"] = json!(true);
        op(ops, "cue", "radio.on");
        if num(&s["radioIndex"]) < 0.0 {
            let track = next(s, "radio");
            play(ops, "radio", &track);
        } else if truth(&s["retuned"]) {
            play(
                ops,
                "radio",
                &s["radioPlaylist"][s["radioIndex"].as_u64().unwrap() as usize],
            );
        } else {
            op(ops, "resume", "radio");
        }
        s["retuned"] = json!(false);
    };
    match method {
        "enterMenu" => {
            if s["scene"] == "fight" {
                op(&mut ops, "stop", "fight");
            }
            s["scene"] = json!("menu");
            s["menuOpen"] = json!(false);
            s["fightPaused"] = json!(false);
            up(&mut s, &mut ops);
        }
        "enterFight" => {
            down(&mut s, &mut ops);
            s["scene"] = json!("fight");
            s["menuOpen"] = json!(false);
            s["fightPaused"] = json!(false);
            let track = next(&mut s, "fight");
            play(&mut ops, "fight", &track);
        }
        "openMenu" => {
            if s["scene"] == "fight" && !truth(&s["menuOpen"]) {
                s["menuOpen"] = json!(true);
                s["fightPaused"] = json!(true);
                op(&mut ops, "pause", "fight");
                up(&mut s, &mut ops);
            }
        }
        "closeMenu" => {
            if s["scene"] == "fight" && truth(&s["menuOpen"]) {
                s["menuOpen"] = json!(false);
                down(&mut s, &mut ops);
                s["fightPaused"] = json!(false);
                op(&mut ops, "resume", "fight");
            }
        }
        "silence" => {
            op(&mut ops, "stop", "fight");
            down(&mut s, &mut ops);
            s["scene"] = json!("silent");
            s["menuOpen"] = json!(false);
            s["fightPaused"] = json!(false);
        }
        "ended" => {
            if arg == "fight" && s["scene"] == "fight" && !truth(&s["fightPaused"]) {
                let track = next(&mut s, "fight");
                play(&mut ops, "fight", &track);
            } else if arg == "radio" && truth(&s["radioOn"]) {
                op(&mut ops, "cue", "radio.tune");
                let track = next(&mut s, "radio");
                play(&mut ops, "radio", &track);
            }
        }
        "nextStation" => {
            if truth(&s["radioOn"]) {
                op(&mut ops, "cue", "radio.tune");
                let track = next(&mut s, "radio");
                play(&mut ops, "radio", &track);
            }
        }
        "playlist" => {
            let list = arg
                .as_array()
                .filter(|p| !p.is_empty())
                .ok_or("Empty playlist")?;
            let current = s["radioIndex"]
                .as_u64()
                .map(|i| s["radioPlaylist"][i as usize].clone());
            s["radioPlaylist"] = arg.clone();
            s["radioIndex"] = json!(-1);
            if let Some(index) = current
                .as_ref()
                .and_then(|current| list.iter().position(|t| t == current))
            {
                s["radioIndex"] = json!(index);
            } else if truth(&s["radioOn"]) {
                op(&mut ops, "cue", "radio.tune");
                let track = next(&mut s, "radio");
                play(&mut ops, "radio", &track);
            } else {
                s["retuned"] = json!(false);
            }
        }
        "tune" => {
            let list = s["radioPlaylist"].as_array().unwrap();
            let index = list
                .iter()
                .position(|t| t == arg)
                .ok_or("Not in the playlist")?;
            if s["radioIndex"] != index {
                s["radioIndex"] = json!(index);
                if truth(&s["radioOn"]) {
                    op(&mut ops, "cue", "radio.tune");
                    play(&mut ops, "radio", arg);
                } else {
                    s["retuned"] = json!(true);
                }
            }
        }
        _ => return Err(format!("Unknown music operation: {method}")),
    }
    Ok(json!({"state":s,"operations":ops}))
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "audio.pickOther" => json!(pick_other(
            num(&args[0]) as usize,
            args[1].as_u64().map(|i| i as usize),
            num(&args[2])
        )),
        "audio.jitter" => json!((num(&args[1]) * 2.0 - 1.0) * num(&args[0])),
        "audio.fightSounds" => fight_sounds(&args[0], &args[1]),
        "audio.cuesForTurn" => cues_for_turn(&args[0], &args[1]),
        "audio.music.new" => music_new(),
        "audio.music.state" => music_state(&args[0]),
        "audio.music.needsRandom" => json!(music_random(&args[0], text(&args[1]), &args[2])),
        "audio.music.step" => {
            return music_step(args[0].clone(), text(&args[1]), &args[2], num(&args[3]))
        }
        _ => return Err(format!("Unknown audio operation: {method}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn next_choice_never_repeats() {
        for n in 2..10 {
            for last in 0..n {
                for sample in [0.0, 0.2, 0.9, 0.999999] {
                    assert_ne!(pick_other(n, Some(last), sample), last);
                }
            }
        }
    }
    #[test]
    fn menu_resume_does_not_restart_fight() {
        let s = music_step(music_new(), "enterFight", &Value::Null, 0.3).unwrap()["state"].clone();
        let s = music_step(s, "openMenu", &Value::Null, 0.7).unwrap()["state"].clone();
        let result = music_step(s, "closeMenu", &Value::Null, 0.0).unwrap();
        assert_eq!(
            result["operations"],
            json!([{"method":"cue","args":["radio.off"]},{"method":"pause","args":["radio"]},{"method":"resume","args":["fight"]}])
        );
    }
}
