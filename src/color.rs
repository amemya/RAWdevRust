use crate::demosaic::linear_to_srgb;

/// カラーパイプライン
///
/// 入力: linear Camera RGB (Vec<f32>, RGBRGB...)
/// 出力: sRGB 8bit (Vec<u8>, RGBRGB...)

/// Step 1: ホワイトバランス係数を適用
/// wb_coeffs: [R, G, B, E]（rawler から取得）
pub fn apply_wb(pixels: &[f32], wb_coeffs: &[f32; 4]) -> Vec<f32> {
    // G 基準に正規化
    let wr = wb_coeffs[0] / wb_coeffs[1];
    let wg = 1.0f32;
    let wb = wb_coeffs[2] / wb_coeffs[1];

    pixels.chunks_exact(3).flat_map(|p| {
        [
            (p[0] * wr).clamp(0.0, 1.0),
            (p[1] * wg).clamp(0.0, 1.0),
            (p[2] * wb).clamp(0.0, 1.0),
        ]
    }).collect()
}

/// Step 2: Camera RGB → linear sRGB
///
/// cam_to_xyz:  rawimage.color_matrixから取得した cam_to_xyz [[f32;4];3]
/// is_d65:      true なら D65 ベース → Bradford 適応不要
///              false なら D50 ベース → Bradford D50→D65 を挟む
pub fn apply_color_matrix(pixels: &[f32], cam_to_xyz: &[[f32; 4]; 3], is_d65: bool) -> Vec<f32> {
    // XYZ(D65) → linear sRGB 行列（IEC 61966-2-1）
    #[rustfmt::skip]
    let xyz_to_srgb: [[f32; 3]; 3] = [
        [ 3.2404542, -1.5371385, -0.4985314],
        [-0.9692660,  1.8760108,  0.0415560],
        [ 0.0556434, -0.2040259,  1.0572252],
    ];

    // cam_to_xyz の 3×3 部分を抽出
    let c2x = [
        [cam_to_xyz[0][0], cam_to_xyz[0][1], cam_to_xyz[0][2]],
        [cam_to_xyz[1][0], cam_to_xyz[1][1], cam_to_xyz[1][2]],
        [cam_to_xyz[2][0], cam_to_xyz[2][1], cam_to_xyz[2][2]],
    ];

    // D50 ベースの場合は Bradford クロマティック適応 D50→D65 を追加
    #[rustfmt::skip]
    let bradford_d50_to_d65: [[f32; 3]; 3] = [
        [ 0.9555766, -0.0230393,  0.0631636],
        [-0.0282895,  1.0099416,  0.0210077],
        [ 0.0122982, -0.0204830,  1.3299098],
    ];

    // 合成行列: sRGB = (xyz_to_srgb) · [Bradford?] · cam_to_xyz · cam_rgb
    let full: [[f32; 3]; 3] = if is_d65 {
        mat3x3_mul(&xyz_to_srgb, &c2x)
    } else {
        let adapted = mat3x3_mul(&bradford_d50_to_d65, &c2x);
        mat3x3_mul(&xyz_to_srgb, &adapted)
    };

    pixels.chunks_exact(3).flat_map(|p| {
        let r = full[0][0] * p[0] + full[0][1] * p[1] + full[0][2] * p[2];
        let g = full[1][0] * p[0] + full[1][1] * p[1] + full[1][2] * p[2];
        let b = full[2][0] * p[0] + full[2][1] * p[1] + full[2][2] * p[2];
        [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
    }).collect()
}

/// Step 3: linear sRGB → sRGB ガンマ変換 + u8 変換
pub fn apply_gamma(pixels: &[f32]) -> Vec<u8> {
    pixels.iter().map(|&v| linear_to_srgb(v)).collect()
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
