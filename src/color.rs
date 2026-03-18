use crate::demosaic::linear_to_srgb;
use rawler::imgop::xyz::Illuminant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetColorSpace {
    Srgb,
    DisplayP3,
}

// 標準の色空間変換マトリクス群
#[rustfmt::skip]
pub const XYZ_TO_SRGB: [[f32; 3]; 3] = [
    [ 3.2404542, -1.5371385, -0.4985314],
    [-0.9692660,  1.8760108,  0.0415560],
    [ 0.0556434, -0.2040259,  1.0572252],
];

#[rustfmt::skip]
pub const XYZ_TO_DISPLAY_P3: [[f32; 3]; 3] = [
    [ 2.4934969, -0.9313836, -0.4027108],
    [-0.8294890,  1.7626641,  0.0236246],
    [ 0.0358458, -0.0761724,  0.9568845],
];

#[rustfmt::skip]
pub const BRADFORD_D50_TO_D65: [[f32; 3]; 3] = [
    [ 0.9555766, -0.0230393,  0.0631636],
    [-0.0282895,  1.0099416,  0.0210077],
    [ 0.0122982, -0.0204830,  1.3299098],
];

/// カラーパイプライン
///
/// 入力: linear Camera RGB (Vec<f32>, RGBRGB...)
/// 出力: sRGB 8bit (Vec<u8>, RGBRGB...)

/// Step 1: ホワイトバランス係数を適用
/// wb_coeffs: [R, G, B, E]（rawler から取得）
pub fn apply_wb(pixels: &mut [f32], wb_coeffs: &[f32; 4]) {
    // G 基準に正規化
    let wr = wb_coeffs[0] / wb_coeffs[1];
    let wg = 1.0f32;
    let wb = wb_coeffs[2] / wb_coeffs[1];

    assert!(
        pixels.len() % 3 == 0,
        "apply_wb expects RGBRGB... packed array"
    );

    for p in pixels.chunks_exact_mut(3) {
        // ハイライトの早期クリップを防ぐため、ここではクランプしない。
        // マトリクス適用後にクランプする
        p[0] = p[0] * wr;
        p[1] = p[1] * wg;
        p[2] = p[2] * wb;
    }
}

/// Step 2: Camera RGB → linear sRGB
///
/// cam_to_xyz:      rawimage.color_matrixから取得した cam_to_xyz [[f32;4];3]
/// cam_illuminant:  color_matrix 導出時のテスト光源。
///                  D50 なら Bradford 適応で D65 に写し、それ以外（D65や未知）はそのまま利用
pub fn apply_color_matrix(
    pixels: &mut [f32],
    cam_to_xyz: &[[f32; 4]; 3],
    cam_illuminant: Option<Illuminant>,
    color_space: TargetColorSpace,
) {
    assert!(
        pixels.len() % 3 == 0,
        "apply_color_matrix expects RGBRGB... packed array"
    );

    // XYZ(D65) → linear Target RGB
    let xyz_to_rgb = match color_space {
        TargetColorSpace::Srgb => XYZ_TO_SRGB,
        TargetColorSpace::DisplayP3 => XYZ_TO_DISPLAY_P3,
    };

    // cam_to_xyz の 3×3 部分を抽出
    let c2x = [
        [cam_to_xyz[0][0], cam_to_xyz[0][1], cam_to_xyz[0][2]],
        [cam_to_xyz[1][0], cam_to_xyz[1][1], cam_to_xyz[1][2]],
        [cam_to_xyz[2][0], cam_to_xyz[2][1], cam_to_xyz[2][2]],
    ];

    // D50 ベースの場合は Bradford クロマティック適応 D50→D65 を追加
    let bradford_d50_to_d65 = BRADFORD_D50_TO_D65;

    // 合成行列: RGB = (xyz_to_rgb) · [Bradford?] · cam_to_xyz · cam_rgb
    let full: [[f32; 3]; 3] = match cam_illuminant {
        Some(Illuminant::D50) => {
            let adapted = mat3x3_mul(&bradford_d50_to_d65, &c2x);
            mat3x3_mul(&xyz_to_rgb, &adapted)
        }
        Some(Illuminant::D65) => mat3x3_mul(&xyz_to_rgb, &c2x),
        _ => {
            // 他の光源、または未知の場合は誤ったBradford適応を避ける
            mat3x3_mul(&xyz_to_rgb, &c2x)
        }
    };

    for p in pixels.chunks_exact_mut(3) {
        let r = full[0][0] * p[0] + full[0][1] * p[1] + full[0][2] * p[2];
        let g = full[1][0] * p[0] + full[1][1] * p[1] + full[1][2] * p[2];
        let b = full[2][0] * p[0] + full[2][1] * p[1] + full[2][2] * p[2];
        p[0] = r.clamp(0.0, 1.0);
        p[1] = g.clamp(0.0, 1.0);
        p[2] = b.clamp(0.0, 1.0);
    }
}

/// Step 3: linear sRGB → sRGB ガンマ変換 + u8 変換
pub fn apply_gamma(pixels: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len());
    out.extend(pixels.iter().map(|&v| linear_to_srgb(v)));
    out
}

// ─── 行列演算ヘルパー ──────────────────────────────────────────────────────

/// 3×3 · 3×3 → 3×3
fn mat3x3_mul(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut r = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                r[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    r
}

// ─── DCP 専用処理 ────────────────────────────────────────────────────────────

/// Phase 4: DCPプロファイルの適用
///
/// wb_coeffs を乗算して White Balanced Camera RGB とし、
/// DCP の ForwardMatrix を用いて XYZ(D50) に変換し、
/// さらに sRGB (linear) へ変換。その後 ToneCurve を適用します。
pub fn apply_dcp(pixels: &mut [f32], dcp: &crate::dcp::DcpProfile, wb_coeffs: &[f32; 4], color_space: TargetColorSpace) -> anyhow::Result<()> {
    // Validate ForwardMatrix before mutating any pixels
    // Prefer ForwardMatrix1 by default; fall back to ForwardMatrix2 if needed.
    let forward = dcp.forward_matrix1.as_ref()
        .or(dcp.forward_matrix2.as_ref());
    
    let fm_data = forward.ok_or_else(|| anyhow::anyhow!("DCP has no ForwardMatrix"))?;
    if fm_data.len() < 9 {
        anyhow::bail!("DCP ForwardMatrix has less than 9 elements");
    }

    let fm = [
        [fm_data[0], fm_data[1], fm_data[2]],
        [fm_data[3], fm_data[4], fm_data[5]],
        [fm_data[6], fm_data[7], fm_data[8]],
    ];

    // 1. WB適用 (Safe to mutate now)
    apply_wb(pixels, wb_coeffs);

    // XYZ(D65) -> Target RGB
    let xyz_d65_to_rgb = match color_space {
        TargetColorSpace::Srgb => XYZ_TO_SRGB,
        TargetColorSpace::DisplayP3 => XYZ_TO_DISPLAY_P3,
    };

    // Bradford D50 -> D65
    let d50_to_d65 = BRADFORD_D50_TO_D65;

    // ForwardMatrix -> XYZ(D50) -> XYZ(D65) -> Target RGB
    let d50_rgb = mat3x3_mul(&xyz_d65_to_rgb, &d50_to_d65);
    let cam_to_rgb = mat3x3_mul(&d50_rgb, &fm);

    for p in pixels.chunks_exact_mut(3) {
        let r = cam_to_rgb[0][0] * p[0] + cam_to_rgb[0][1] * p[1] + cam_to_rgb[0][2] * p[2];
        let g = cam_to_rgb[1][0] * p[0] + cam_to_rgb[1][1] * p[1] + cam_to_rgb[1][2] * p[2];
        let b = cam_to_rgb[2][0] * p[0] + cam_to_rgb[2][1] * p[1] + cam_to_rgb[2][2] * p[2];
        
        p[0] = r;
        p[1] = g;
        p[2] = b;
    }

    // 2.5 3D LUT (LookTable / HueSatMap)
    if let Some(dims) = dcp.look_table_dims {
        if let Some(ref data) = dcp.look_table_data {
            apply_3d_lut_hsv(pixels, dims, data, true);
        }
    } else if let Some(dims) = dcp.map_dims {
        if let Some(ref data) = dcp.hsl_map1.as_ref().or(dcp.hsl_map2.as_ref()) {
            apply_3d_lut_hsv(pixels, dims, data, false);
        }
    }

    // 3. ToneCurve 適用
    // When a DCP profile does not contain a Tone Curve (e.g. Adobe Standard),
    // it expects the raw converter to apply a default camera base curve to avoid dark/flat images.
    let curve = dcp.tone_curve.as_ref().map(|v| v.as_slice()).unwrap_or(DEFAULT_BASE_CURVE);
    apply_tone_curve(pixels, curve);
    
    // 最後にクランプ
    for p in pixels.iter_mut() {
        *p = p.clamp(0.0, 1.0);
    }

    Ok(())
}

pub const DEFAULT_BASE_CURVE: &[f32] = &[
    0.000000, 0.000000, 0.000815, 0.000216, 0.001629, 0.000527, 0.002444, 0.000719, 
    0.003263, 0.000994, 0.004177, 0.001278, 0.005225, 0.001604, 0.006413, 0.002283, 
    0.007745, 0.003488, 0.009225, 0.005017, 0.010859, 0.006432, 0.012650, 0.008108, 
    0.014603, 0.010093, 0.016721, 0.012621, 0.019008, 0.015511, 0.021468, 0.018849, 
    0.024104, 0.022723, 0.026920, 0.027082, 0.029919, 0.032464, 0.033105, 0.038840, 
    0.036480, 0.046109, 0.040047, 0.053969, 0.043811, 0.062670, 0.047773, 0.071868, 
    0.051936, 0.082485, 0.056304, 0.094067, 0.060878, 0.106574, 0.065663, 0.119607, 
    0.070660, 0.133852, 0.075872, 0.149402, 0.081302, 0.165296, 0.086952, 0.182260, 
    0.092824, 0.199432, 0.098922, 0.217684, 0.105247, 0.236403, 0.111802, 0.256006, 
    0.118589, 0.275811, 0.125610, 0.295711, 0.132868, 0.316630, 0.140365, 0.337569, 
    0.148104, 0.358112, 0.156085, 0.379081, 0.164312, 0.399856, 0.172787, 0.420881, 
    0.181511, 0.441377, 0.190487, 0.462955, 0.199717, 0.483637, 0.209202, 0.504881, 
    0.218945, 0.525044, 0.228948, 0.545642, 0.239212, 0.565098, 0.249740, 0.584290, 
    0.260533, 0.603064, 0.271594, 0.621669, 0.282924, 0.639215, 0.294525, 0.656147, 
    0.306398, 0.672549, 0.318547, 0.689028, 0.330972, 0.704691, 0.343675, 0.719249, 
    0.356657, 0.734295, 0.369922, 0.748306, 0.383470, 0.762125, 0.397303, 0.777039, 
    0.411423, 0.789826, 0.425831, 0.803516, 0.440530, 0.815911, 0.455520, 0.829064, 
    0.470804, 0.840157, 0.486382, 0.852910, 0.502258, 0.863592, 0.518431, 0.874068, 
    0.534905, 0.884035, 0.551679, 0.893294, 0.568757, 0.901417, 0.586139, 0.909956, 
    0.603827, 0.917647, 0.621823, 0.924610, 0.640128, 0.931915, 0.658743, 0.939778, 
    0.677670, 0.945235, 0.696911, 0.951333, 0.716466, 0.957195, 0.736338, 0.962819, 
    0.756528, 0.968076, 0.777038, 0.972538, 0.797867, 0.977568, 0.819019, 0.982376, 
    0.840495, 0.986933, 0.862296, 0.991268, 0.884423, 0.995156, 0.906877, 0.998027, 
    0.929661, 0.999145, 0.952775, 0.999426, 0.976221, 0.999711, 1.000000, 1.000000, 
];

/// Tone Curve (x, y 配列) を1D LUTを用いて高速に補間適用
fn apply_tone_curve(pixels: &mut [f32], curve: &[f32]) {
    // curveは [x0, y0, x1, y1, ...]
    let num_points = curve.len() / 2;
    if num_points < 2 {
        return;
    }

    // 1D LUT (4096 sample points) for O(1) per-pixel lookup instead of nested loops
    const LUT_SIZE: usize = 4096;
    let mut lut = [0.0; LUT_SIZE];
    
    for i in 0..LUT_SIZE {
        let x = i as f32 / (LUT_SIZE - 1) as f32;
        
        let mut y = curve[(num_points - 1) * 2 + 1];
        if x <= curve[0] {
            y = curve[1];
        } else {
            for j in 0..num_points - 1 {
                let x0 = curve[j * 2];
                let y0 = curve[j * 2 + 1];
                let x1 = curve[(j + 1) * 2];
                let y1 = curve[(j + 1) * 2 + 1];

                if x >= x0 && x <= x1 {
                    let d = x1 - x0;
                    if d > 1e-6 {
                        let t = (x - x0) / d;
                        y = y0 + t * (y1 - y0);
                    } else {
                        y = y0;
                    }
                    break;
                }
            }
        }
        lut[i] = y;
    }

    for p in pixels.iter_mut() {
        let x = p.clamp(0.0, 1.0);
        let idx = (x * (LUT_SIZE - 1) as f32).round() as usize;
        *p = lut[idx];
    }
}

/// HSV色空間での 3D LUT 適用
/// LUT data is stored linearly: V, S, H
/// pixels input is linear sRGB (or linear camera-referred). Wait, usually LUTs
/// are applied in HSV space. The DCP specification applies HueSatMap / LookTable
/// on linear ProPhoto-like RGB or camera RGB. For simplicity we'll convert sRGB -> HSV -> apply -> sRGB.
fn apply_3d_lut_hsv(pixels: &mut [f32], dims: [u32; 3], data: &[f32], is_look_table: bool) {
    let dh = dims[0] as usize; // Hue divisions
    let ds = dims[1] as usize; // Saturation divisions
    let dv = dims[2] as usize; // Value divisions

    if dh < 1 || ds < 2 || dv < 2 {
        return; // Avoid underflow and division by zero
    }
    
    // Validate LUT dimensions against data length using checked arithmetic.
    // If the LUT is inconsistent or would overflow, skip applying it.
    let lut_valid = dh
        .checked_mul(ds)
        .and_then(|x| x.checked_mul(dv))
        .and_then(|x| x.checked_mul(3))
        .map(|total| total <= data.len())
        .unwrap_or(false);

    if !lut_valid {
        return;
    }
    
    // Helper to fetch (h_shift, s_scale, v_scale) at grid point (hoisted out of pixel loop)
    let fetch = |hv: usize, sv: usize, vv: usize| -> (f32, f32, f32) {
        let idx_opt = vv
            .checked_mul(ds)
            .and_then(|x| x.checked_mul(dh))
            .and_then(|x| x.checked_add(sv.saturating_mul(dh)))
            .and_then(|x| x.checked_add(hv))
            .and_then(|x| x.checked_mul(3));

        if let Some(idx) = idx_opt {
            if idx + 2 < data.len() {
                (data[idx], data[idx + 1], data[idx + 2])
            } else {
                (0.0, 1.0, 1.0)
            }
        } else {
            (0.0, 1.0, 1.0)
        }
    };

    let lerp = |a: f32, b: f32, f: f32| a + f * (b - a);
    let interp = |v000: f32, v100: f32, v010: f32, v110: f32,
                  v001: f32, v101: f32, v011: f32, v111: f32,
                  df: f32, dsf: f32, dvf: f32| -> f32 {
        let i00 = lerp(v000, v100, df);
        let i10 = lerp(v010, v110, df);
        let i01 = lerp(v001, v101, df);
        let i11 = lerp(v011, v111, df);
        let i0 = lerp(i00, i10, dsf);
        let i1 = lerp(i01, i11, dsf);
        lerp(i0, i1, dvf)
    };

    // elements per LUT entry: 3 (H shift, S scale, V scale) for HueSatMap
    // LookTable has 3 elements too.
    for p in pixels.chunks_exact_mut(3) {
        let r = p[0];
        let g = p[1];
        let b = p[2];

        // 簡易RGB -> HSV変換
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;
        let s = if max > 1e-6 { delta / max } else { 0.0 };
        let mut h = 0.0;
        if delta > 1e-6 {
            if max == r {
                h = (g - b) / delta;
            } else if max == g {
                h = 2.0 + (b - r) / delta;
            } else {
                h = 4.0 + (r - g) / delta;
            }
            h *= 60.0;
            if h < 0.0 {
                h += 360.0;
            }
        }

        // 3D次元のインデックス計算 (Trilinear Interpolation)
        // h: 0..360 -> h_idx: 0..(dh-1)  (wrap-around for Hue)
        // s: 0..1  -> s_idx: 0..(ds-1)
        // v: 0..1  -> v_idx: 0..(dv-1)

        let h_norm = (h / 360.0).clamp(0.0, 1.0);
        let s_norm = s.clamp(0.0, 1.0);

        // v can be negative if the color matrix result was out of gamut.
        // powf with negative base returns NaN, so we clamp v to [0, 1] first.
        let v_safe = v.clamp(0.0, 1.0);

        // The DNG specification says ProfileLookTable is indexed by HSV values that
        // correspond to a generic base curve applied to linear ProPhoto RGB.
        // We'll use a simple gamma 1/1.8 approximation just for indexing.
        let v_gamma = if is_look_table { v_safe.powf(1.0 / 1.8) } else { v_safe };
        let v_norm = v_gamma.clamp(0.0, 1.0);

        if v_norm.is_nan() || s_norm.is_nan() || h_norm.is_nan() {
            continue; // Prevent NaN from corrupting indices or pixel values
        }

        // Trilinear Interpolation
        let hf = h_norm * dh as f32;
        let sf = s_norm * (ds - 1) as f32;
        let vf = v_norm * (dv - 1) as f32;

        let h0 = (hf.floor() as usize) % dh;
        let s0 = sf.floor() as usize;
        let v0 = vf.floor() as usize;

        // Wrap hue (360 -> 0)
        let h1 = if h0 + 1 >= dh { 0 } else { h0 + 1 };
        // Clamp saturation and value
        let s1 = (s0 + 1).min(ds - 1);
        let v1 = (v0 + 1).min(dv - 1);

        let dh_frac = hf - hf.floor();
        let ds_frac = sf - sf.floor();
        let dv_frac = vf - vf.floor();

        // 8 corners
        let c000 = fetch(h0, s0, v0);
        let c100 = fetch(h1, s0, v0);
        let c010 = fetch(h0, s1, v0);
        let c110 = fetch(h1, s1, v0);
        let c001 = fetch(h0, s0, v1);
        let c101 = fetch(h1, s0, v1);
        let c011 = fetch(h0, s1, v1);
        let c111 = fetch(h1, s1, v1);

        let h_shift = interp(c000.0, c100.0, c010.0, c110.0, c001.0, c101.0, c011.0, c111.0, dh_frac, ds_frac, dv_frac);
        let s_scale = interp(c000.1, c100.1, c010.1, c110.1, c001.1, c101.1, c011.1, c111.1, dh_frac, ds_frac, dv_frac);
        let v_scale = interp(c000.2, c100.2, c010.2, c110.2, c001.2, c101.2, c011.2, c111.2, dh_frac, ds_frac, dv_frac);

        let mut out_h = h;
        let mut out_s = s;
        let mut out_v = v;

        if is_look_table {
            // ProfileLookTable applies shifts relative to current HSV
            out_h += h_shift; // usually stored in degrees
            out_s *= s_scale;
            out_v *= v_scale;
        } else {
            // HueSatMap
            out_h += h_shift;
            out_s *= s_scale;
            out_v *= v_scale;
        }

        // HSV -> RGB (reconstructed)
        out_h = out_h.rem_euclid(360.0);
        out_s = out_s.clamp(0.0, 1.0);
            
            let c = out_v * out_s;
            let x = c * (1.0 - ((out_h / 60.0) % 2.0 - 1.0).abs());
            let m = out_v - c;

            let (r_, g_, b_) = if out_h < 60.0 {
                (c, x, 0.0)
            } else if out_h < 120.0 {
                (x, c, 0.0)
            } else if out_h < 180.0 {
                (0.0, c, x)
            } else if out_h < 240.0 {
                (0.0, x, c)
            } else if out_h < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };

            p[0] = r_ + m;
            p[1] = g_ + m;
            p[2] = b_ + m;
    }
}
