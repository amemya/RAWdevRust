use crate::decode::RawData;

/// RCD (Ratio Corrected Demosaicing) デモザイク
///
/// 手順:
///   1. Hamilton-Adams EdgeDirected 補間で緑チャネル全画素を補完
///   2. 既知 R/B 位置で R/G・B/G の比率マップを生成
///   3. 比率マップを 3×3 平均フィルタで平滑化
///   4. 比率 × 補間済み緑 = R/B チャネルを復元
///
/// output: Vec<f32> RGBRGB... linear Camera RGB（WB・ガンマなし）
pub fn run(raw: &RawData) -> Vec<f32> {
    let w = raw.width;
    let h = raw.height;

    // ─── ブラックレベル減算 & [0,1] 正規化 ───────────────────────
    let norm: Vec<f32> = raw.pixels.iter().enumerate().map(|(i, &p)| {
        let row = i / w;
        let col = i % w;
        let bi = (row % 2) * 2 + (col % 2);
        let bl = raw.black_level[bi];
        let wl = raw.white_level[bi];
        ((p as f32 - bl) / (wl - bl)).clamp(0.0, 1.0)
    }).collect();

    // ─── Step 1: 緑チャネルの Hamilton-Adams 補間 ────────────────
    let green = interp_green(&norm, w, h, raw);

    // ─── Step 2 & 3: R/G・B/G 比率マップ生成 & 平滑化 ────────────
    let (ratio_r, ratio_b) = build_ratio_maps(&norm, &green, w, h, raw);

    // ─── Step 4: R/B 復元（WB・ガンマは color.rs で適用）─────────
    let mut out = Vec::with_capacity(w * h * 3);
    for i in 0..w * h {
        // [0,1] でクランプすると、1.0 を超えたデータを失いハイライトで色がおかしくなる（早期クリップ）
        // そのため、ここではクランプせずに linear 値のまま保持し、カラーマトリクス適用後にクランプする
        out.push(ratio_r[i] * green[i]);
        out.push(green[i]);
        out.push(ratio_b[i] * green[i]);
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 内部関数
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn px(buf: &[f32], w: usize, h: usize, row: i32, col: i32) -> f32 {
    let r = row.clamp(0, h as i32 - 1) as usize;
    let c = col.clamp(0, w as i32 - 1) as usize;
    buf[r * w + c]
}

/// Step 1: Hamilton-Adams EdgeDirected 緑チャネル補間
///
/// Gの既知位置はそのまま使い、R/B位置では水平・垂直方向の
/// エッジ強度を比較して方向を選んで補間する。
fn interp_green(norm: &[f32], w: usize, h: usize, raw: &RawData) -> Vec<f32> {
    let mut green = vec![0.0f32; w * h];

    for row in 0..h {
        for col in 0..w {
            let idx = row * w + col;
            let color = raw.cfa.color_at(row, col);

            if color == 1 {
                // すでに緑
                green[idx] = norm[idx];
            } else {
                // R or B 位置 → エッジ方向推定
                let (r, c) = (row as i32, col as i32);

                // 水平方向の差分・勾配
                // G_h = (G[c-1]+G[c+1])/2 + (2*C[c]-C[c-2]-C[c+2])/4
                let gh = 0.5 * (px(norm, w, h, r, c - 1) + px(norm, w, h, r, c + 1))
                    + 0.25 * (2.0 * px(norm, w, h, r, c    )
                            - px(norm, w, h, r, c - 2)
                            - px(norm, w, h, r, c + 2));
                let dh = (px(norm, w, h, r, c - 1) - px(norm, w, h, r, c + 1)).abs()
                    + (px(norm, w, h, r, c    ) - px(norm, w, h, r, c - 2)).abs()
                    + (px(norm, w, h, r, c    ) - px(norm, w, h, r, c + 2)).abs();

                // 垂直方向の差分・勾配
                // G_v = (G[r-1]+G[r+1])/2 + (2*C[r]-C[r-2]-C[r+2])/4
                let gv = 0.5 * (px(norm, w, h, r - 1, c) + px(norm, w, h, r + 1, c))
                    + 0.25 * (2.0 * px(norm, w, h, r    , c)
                            - px(norm, w, h, r - 2, c)
                            - px(norm, w, h, r + 2, c));
                let dv = (px(norm, w, h, r - 1, c) - px(norm, w, h, r + 1, c)).abs()
                    + (px(norm, w, h, r    , c) - px(norm, w, h, r - 2, c)).abs()
                    + (px(norm, w, h, r    , c) - px(norm, w, h, r + 2, c)).abs();

                green[idx] = if dh < dv {
                    gh
                } else if dv < dh {
                    gv
                } else {
                    0.5 * (gh + gv)
                }
                .clamp(0.0, 1.0);
            }
        }
    }
    green
}

/// Step 2 & 3: R/G・B/G 比率マップを生成し 3×3 平均フィルタで平滑化
fn build_ratio_maps(
    norm: &[f32],
    green: &[f32],
    w: usize,
    h: usize,
    raw: &RawData,
) -> (Vec<f32>, Vec<f32>) {
    // 既知位置のみに比率を入れたスパースマップ
    let mut ratio_r_raw = vec![1.0f32; w * h];
    let mut ratio_b_raw = vec![1.0f32; w * h];

    for row in 0..h {
        for col in 0..w {
            let idx = row * w + col;
            let g = green[idx].max(1e-6); // ゼロ除算防止
            match raw.cfa.color_at(row, col) {
                0 => ratio_r_raw[idx] = norm[idx] / g,
                2 => ratio_b_raw[idx] = norm[idx] / g,
                _ => {}
            }
        }
    }

    // 3×3 近傍平均（同チャネルの既知値だけで平均）
    let ratio_r = smooth_ratio(&ratio_r_raw, w, h, raw, 0);
    let ratio_b = smooth_ratio(&ratio_b_raw, w, h, raw, 2);

    (ratio_r, ratio_b)
}

/// 比率マップを近傍の同チャネル既知値の平均で平滑化
fn smooth_ratio(ratio_raw: &[f32], w: usize, h: usize, raw: &RawData, channel: usize) -> Vec<f32> {
    let mut out = ratio_raw.to_vec();

    for row in 0..h {
        for col in 0..w {
            let mut sum = 0.0f32;
            let mut count = 0u32;
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    let nr = (row as i32 + dr).clamp(0, h as i32 - 1) as usize;
                    let nc = (col as i32 + dc).clamp(0, w as i32 - 1) as usize;
                    if raw.cfa.color_at(nr, nc) == channel {
                        sum += ratio_raw[nr * w + nc];
                        count += 1;
                    }
                }
            }
            if count > 0 {
                out[row * w + col] = sum / count as f32;
            }
        }
    }
    out
}


