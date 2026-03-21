use super::apply_srgb_transfer_curve;
use crate::decode::RawData;

/// Bilinear デモザイク
/// output: Vec<u8> RGBRGB... (8bit sRGB)
pub fn run(raw: &RawData) -> Vec<u8> {
    let w = raw.width;
    let h = raw.height;

    // ブラックレベル減算 & [0.0, 1.0] 正規化
    // ベイヤー配列の各位置に対応するブラック/ホワイトレベルを使う
    let normalized: Vec<f32> = raw
        .pixels
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let row = i / w;
            let col = i % w;
            // ベイヤー2x2ブロック内の位置でbl/wlを選択
            let bayer_idx = (row % 2) * 2 + (col % 2);
            let bl = raw.black_level[bayer_idx];
            let wl = raw.white_level[bayer_idx];
            let v = p as f32 - bl;
            (v / (wl - bl)).clamp(0.0, 1.0)
        })
        .collect();

    // R G B チャネルの既知値マスク
    let mut r = vec![0.0f32; w * h];
    let mut g = vec![0.0f32; w * h];
    let mut b = vec![0.0f32; w * h];

    for row in 0..h {
        for col in 0..w {
            let idx = row * w + col;
            match raw.cfa.color_at(row, col) {
                0 => r[idx] = normalized[idx], // R
                1 => g[idx] = normalized[idx], // G
                2 => b[idx] = normalized[idx], // B
                _ => {}
            }
        }
    }

    // Bilinear 補間
    let r_interp = interpolate(&r, w, h, &raw, 0);
    let g_interp = interpolate(&g, w, h, &raw, 1);
    let b_interp = interpolate(&b, w, h, &raw, 2);

    // ホワイトバランス係数を適用（G基準に正規化）
    let wb = &raw.wb_coeffs;
    let g_ref = wb[1];
    let wb_r = wb[0] / g_ref;
    let wb_b = wb[2] / g_ref;

    let mut out = Vec::with_capacity(w * h * 3);
    for i in 0..w * h {
        let rv = (r_interp[i] * wb_r).clamp(0.0, 1.0);
        let gv = g_interp[i].clamp(0.0, 1.0);
        let bv = (b_interp[i] * wb_b).clamp(0.0, 1.0);
        out.push(apply_srgb_transfer_curve(rv));
        out.push(apply_srgb_transfer_curve(gv));
        out.push(apply_srgb_transfer_curve(bv));
    }
    out
}

/// 特定チャネルの Bilinear 補間（近傍の同チャネル画素の平均）
fn interpolate(src: &[f32], w: usize, h: usize, raw: &RawData, channel: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; w * h];
    for row in 0..h {
        for col in 0..w {
            let idx = row * w + col;
            if raw.cfa.color_at(row, col) == channel {
                out[idx] = src[idx];
            } else {
                let mut sum = 0.0f32;
                let mut count = 0u32;
                for dr in -1i32..=1 {
                    for dc in -1i32..=1 {
                        let nr = row as i32 + dr;
                        let nc = col as i32 + dc;
                        if nr >= 0 && nr < h as i32 && nc >= 0 && nc < w as i32 {
                            let nr = nr as usize;
                            let nc = nc as usize;
                            if raw.cfa.color_at(nr, nc) == channel {
                                sum += src[nr * w + nc];
                                count += 1;
                            }
                        }
                    }
                }
                if count > 0 {
                    out[idx] = sum / count as f32;
                }
            }
        }
    }
    out
}
