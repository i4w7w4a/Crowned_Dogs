//! Body-camera inverse distortion, render-target budgets and sky-light centroid.
pub fn compute(kind: &str, x: &[f64]) -> Result<Vec<f64>, String> {
    let expected = match kind {
        "arena.lens.screen" | "arena.lens.size" => 9,
        "arena.lens.sun" => {
            if x.len() >= 5 {
                5 + (x[0] * x[1] * 4.0) as usize
            } else {
                5
            }
        }
        _ => return Err(format!("Unknown lens computation: {kind}")),
    };
    if x.len() != expected {
        return Err(format!(
            "{kind} requires {expected} values, got {}",
            x.len()
        ));
    }
    Ok(match kind {
        "arena.lens.screen" => {
            if x[8] == 0.0 {
                return Ok(x[..2].to_vec());
            }
            let cx = x[2] + x[4] / 2.0;
            let cy = x[3] + x[5] / 2.0;
            let hx = x[4] / 2.0;
            let hy = x[5] / 2.0;
            let aspect = x[4] * x[6] / (x[5] * x[7]);
            let n = aspect.hypot(1.0);
            let mut px = x[0];
            let mut py = x[1];
            for _ in 0..4 {
                let ex = (px - cx) / hx * aspect / n;
                let ey = (py - cy) / hy / n;
                let f = 1.0 + x[8] * (ex * ex + ey * ey);
                px = cx + (x[0] - cx) / f;
                py = cy + (x[1] - cy) / f;
            }
            vec![px, py]
        }
        "arena.lens.size" => {
            let max = x[4].min(8192.0);
            let ss = x[5].min(max / x[0].max(x[1]));
            let w = (x[0] * ss).round().max(1.0);
            let h = (x[1] * ss).round().max(1.0);
            let nk = 2.0 / x[6].max(1.0);
            let nw = x[0].min(x[2] * nk).round().max(1.0);
            let nh = x[1].min(x[3] * nk).round().max(1.0);
            let hw = (x[0] * x[7]).round().max(1.0);
            let hh = (x[1] * x[7]).round().max(1.0);
            let rw = (x[0] * x[8]).round().max(1.0);
            let rh = (x[1] * x[8]).round().max(1.0);
            vec![
                w,
                h,
                nw,
                nh,
                hw,
                hh,
                rw,
                rh,
                (w / rw).log2().max(0.0),
                w / x[0],
                (w / 941.0).log2(),
            ]
        }
        "arena.lens.sun" => {
            let w = x[0] as usize;
            let h = x[1] as usize;
            let mut sx = 0.0;
            let mut sy = 0.0;
            let mut sw = 0.0;
            // Pixels are supplied by the browser canvas; weighting stays deterministic in Rust.
            for y in 0..(x[1] * x[2]).round().max(0.0) as usize {
                for col in 0..w {
                    let i = 5 + (y * w + col) * 4;
                    if i + 2 >= x.len() {
                        continue;
                    }
                    let l = (0.2126 * x[i] + 0.7152 * x[i + 1] + 0.0722 * x[i + 2]) / 255.0;
                    let wt = (l - 0.72).max(0.0).powi(2);
                    sx += wt * (col as f64 + 0.5);
                    sy += wt * (y as f64 + 0.5);
                    sw += wt;
                }
            }
            if sw > 0.0 {
                vec![sx / sw / w as f64, 1.0 - sy / sw / h as f64]
            } else {
                vec![x[3], x[4]]
            }
        }
        _ => unreachable!(),
    })
}
