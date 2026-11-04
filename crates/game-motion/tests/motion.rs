use game_motion::{compute, dispatch};
use glam::{DQuat, DVec3};
use serde_json::{Value, json};
#[test]
fn ik_reaches_target_and_preserves_segment_lengths() {
    let a = DVec3::ZERO;
    let b = DVec3::new(0.0, -0.3, 0.0);
    let c = DVec3::new(0.05, -0.55, 0.0);
    let target = DVec3::new(0.25, -0.3, 0.12);
    let pole = DVec3::new(0.4, -0.1, 0.3);
    let mut input = vec![];
    for v in [a, b, c, target, pole] {
        input.extend(v.to_array());
    }
    for _ in 0..4 {
        input.extend(DQuat::IDENTITY.to_array());
    }
    let result = compute("reach", &input).unwrap();
    assert_eq!(result[0], 1.0);
    let q1 = DQuat::from_slice(&result[1..5]);
    let q2 = DQuat::from_slice(&result[5..9]);
    let elbow = q1 * b;
    let end = elbow + q1 * q2 * (c - b);
    assert!(end.distance(target) < 1e-10);
    assert!((elbow.length() - b.length()).abs() < 1e-12);
    assert!((end.distance(elbow) - c.distance(b)).abs() < 1e-12);
}
#[test]
fn degenerate_ik_does_not_invent_a_pose() {
    let mut v = vec![0.0; 31];
    for offset in [15, 19, 23, 27] {
        v[offset + 3] = 1.0;
    }
    assert_eq!(compute("reach", &v).unwrap(), vec![0.0]);
    assert!(compute("reach", &[0.0; 8]).is_err());
}
#[test]
fn knife_mark_is_emitted_once_and_grenade_releases_at_throw() {
    let created = dispatch(
        "motion.timeline",
        json!({"event":"create","durations":{"idle":2.0,"attack_knife":2.0,"grenade_throw":2.4}}),
    )
    .unwrap();
    let started = dispatch(
        "motion.timeline",
        json!({"state":created["state"],"event":"attack","kind":"grenade","inSeconds":0.0}),
    )
    .unwrap();
    let tick = dispatch(
        "motion.timeline",
        json!({"state":started["state"],"event":"update","dt":0.0,"time":1.21}),
    )
    .unwrap();
    let commands = tick["commands"].as_array().unwrap();
    assert_eq!(commands.iter().filter(|v| v["op"] == "mark").count(), 1);
    assert!(
        commands
            .iter()
            .any(|v| v["op"] == "hold" && v["value"] == Value::Null)
    );
    let again = dispatch(
        "motion.timeline",
        json!({"state":tick["state"],"event":"update","dt":0.0,"time":1.3}),
    )
    .unwrap();
    assert!(again["commands"].as_array().unwrap().is_empty());
}
#[test]
fn dust_remains_inside_the_original_salt_hall_volume() {
    let output = compute("dust", &[1024.0, 417093.0]).unwrap();
    assert_eq!(output.len(), 1024 * 7);
    for point in output[..1024 * 3].chunks_exact(3) {
        assert!((-4.0..4.0).contains(&point[0]));
        assert!((0.1..4.7).contains(&point[1]));
        assert!((-16.0..-1.0).contains(&point[2]));
    }
    assert!(output[1024 * 3..].iter().all(|n| (0.0..1.0).contains(n)));
}
#[test]
fn look_turns_the_face_to_the_target_across_neck_and_head() {
    // A head turned 40° to its right (the rifle stance) and canted 25° onto the stock; the target is straight ahead.
    let neck = DQuat::from_rotation_y(-0.3);
    let head = DQuat::from_rotation_y(-40f64.to_radians()) * DQuat::from_rotation_z(0.44);
    let mut input = neck.to_array().to_vec();
    input.extend(head.to_array());
    input.extend(DVec3::X.to_array()); // ear-to-ear axis in the head's frame (its rest is the identity)
    input.extend([0.0, 1.6, 0.0, 0.3, 1.5, 8.0, 1.0, 0.4, 1.3]);
    let out = compute("look", &input).unwrap();
    let to = (0.3f64).atan2(8.0);
    assert!(
        (out[8] - (40f64.to_radians() + to)).abs() < 1e-9,
        "the cant does not change the heading"
    );
    let (dn, dh) = (DQuat::from_slice(&out[0..4]), DQuat::from_slice(&out[4..8]));
    // The neck carries 40% of the turn; the head ends up facing the target with its cant kept.
    let turned_neck = neck * dn;
    assert!(
        ((turned_neck * DVec3::Z).x.atan2((turned_neck * DVec3::Z).z) - (-0.3 + 0.4 * out[8]))
            .abs()
            < 1e-9
    );
    let turned_head = DQuat::from_rotation_y(0.4 * out[8]) * head * dh;
    let face = (turned_head * DVec3::X).cross(DVec3::Y);
    assert!((face.x.atan2(face.z) - to).abs() < 1e-9);
    assert!(((turned_head * DVec3::X).y - (head * DVec3::X).y).abs() < 1e-9);
    // Half weight turns half way; a turn past the limit stops at it.
    input[17] = 0.5;
    let half = compute("look", &input).unwrap();
    assert!((DQuat::from_slice(&half[4..8]).to_axis_angle().1 - 0.5 * 0.6 * out[8]).abs() < 1e-9);
    input[17] = 1.0;
    input[19] = 0.2;
    let capped = compute("look", &input).unwrap();
    assert!((DQuat::from_slice(&capped[4..8]).to_axis_angle().1 - 0.6 * 0.2).abs() < 1e-9);
}

#[test]
fn a_spent_flash_stays_dark_when_time_steps_back() {
    let f = compute("flash", &[0.0, -8.0, 0.1]).unwrap();
    assert_eq!(f[0], 0.0);
    assert_eq!(f[1], 0.0);
    let lit = compute("flash", &[0.07, 0.0, 0.1]).unwrap();
    assert!((lit[1] - 1.0).abs() < 1e-9);
}
