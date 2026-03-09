use std::path::Path;

use rawler::{
    CFA,
    decoders::RawDecodeParams,
    imgop::{
        Dim2, crop,
        xyz::Illuminant,
    },
    rawimage::{RawImageData, RawPhotometricInterpretation},
    rawsource::RawSource,
};

/// rawlerから取り出したベイヤー配列と必要なメタデータ
pub struct RawData {
    pub pixels: Vec<u16>,
    pub width: usize,
    pub height: usize,
    pub cfa: CFA,
    pub black_level: [f32; 4],
    pub white_level: [f32; 4],
    /// カメラホワイトバランス係数 [R, G, B, E] (NaN → 1.0)
    pub wb_coeffs: [f32; 4],
    /// Camera RGB → XYZ 変換行列 [[f32;4];3]  (rows=XYZ, cols=CamRGB+pad)
    /// D65 ベースの行列を優先して格納
    pub cam_to_xyz: [[f32; 4]; 3],
    /// color_matrix を導出した時点でのテスト光源
    pub cam_illuminant: Option<Illuminant>,
}

pub fn load(path: &Path) -> anyhow::Result<RawData> {
    let rawsource = RawSource::new(path)?;
    let params = RawDecodeParams::default();
    let rawimage = rawler::get_decoder(&rawsource)?.raw_image(&rawsource, &params, false)?;

    let full_width = rawimage.width;

    // cam_to_xyz: rawimage.color_matrix（非 deprecated）から取得
    let (cam_to_xyz, cam_illuminant) = {
        // D65 → D50 の順に優先
        let (flat, illuminant) = if let Some(m) = rawimage.color_matrix.get(&Illuminant::D65) {
            (Some(m.clone()), Some(Illuminant::D65))
        } else if let Some(m) = rawimage.color_matrix.get(&Illuminant::D50) {
            (Some(m.clone()), Some(Illuminant::D50))
        } else if let Some((&k, m)) = rawimage.color_matrix.iter().next() {
            eprintln!("Warning: color_matrix illuminant is {:?} (not D50/D65)", k);
            (Some(m.clone()), Some(k.clone()))
        } else {
            eprintln!("Warning: no color_matrix found, using identity");
            (None, None)
        };

        if let Some(flat) = flat {
            if flat.len() >= 9 {
                // FlatColorMatrix は行優先 xyz_to_cam: [cam][xyz]
                // 最初の 9 値 (3cam × 3xyz) を使用
                let xyz2cam_raw = [
                    [flat[0], flat[1], flat[2]],
                    [flat[3], flat[4], flat[5]],
                    [flat[6], flat[7], flat[8]],
                ];
                // 各行をその和で1になるよう正規化
                // 目的: 定数ベクトル XYZ=[1,1,1]（等エネルギー白）を入力したとき、出力が cam=[1,1,1] となるように各行のスケールを調整するための処理。
                // 実際の標準テスト光源（D50/D65等）のXYZ値は(1,1,1)ではないが、カラーマトリクス適用後の
                // ホワイトバランス処理等との兼ね合いで、XYZ=[1,1,1] → cam=[1,1,1] の対応関係を作るための正規化。
                let mut xyz2cam = xyz2cam_raw;
            for row in &mut xyz2cam {
                let sum: f32 = row.iter().sum();
                if sum.abs() > 1e-10 {
                    for v in row.iter_mut() { *v /= sum; }
                }
            }

                // 逆行列計算 → cam_to_xyz
                let (inv, final_illuminant) = match mat3x3_inverse(&xyz2cam) {
                    Some(m) => (m, illuminant),
                    None => {
                        eprintln!("Warning: matrix inversion failed, using identity");
                        ([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], None)
                    }
                };

                let m = [
                    [inv[0][0], inv[0][1], inv[0][2], 0.0],
                    [inv[1][0], inv[1][1], inv[1][2], 0.0],
                    [inv[2][0], inv[2][1], inv[2][2], 0.0],
                ];
                (m, final_illuminant)
            } else {
                eprintln!("Warning: color_matrix length < 9 ({}), using identity", flat.len());
                ([[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0]], None)
            }
        } else {
            ([[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0]], None)
        }
    };

    let cfa_full = match &rawimage.photometric {
        RawPhotometricInterpretation::Cfa(cfg) => cfg.cfa.clone(),
        other => anyhow::bail!("Unsupported photometric: {:?}", other),
    };
    let black_level = rawimage.blacklevel.as_bayer_array();
    let white_level = rawimage.whitelevel.as_bayer_array();
    let wb_coeffs = rawimage.wb_coeffs.map(|v| if v.is_nan() { 1.0 } else { v });
    let active_area = rawimage.active_area;

    let pixels_full: Vec<u16> = match rawimage.data {
        RawImageData::Integer(data) => data,
        RawImageData::Float(_) => anyhow::bail!("Float RAW data is not supported yet"),
    };

    let (pixels, width, height, cfa) = if let Some(area) = active_area {
        let full_h = pixels_full.len() / full_width;
        let cropped = crop(&pixels_full, Dim2::new(full_width, full_h), area);
        let cfa_shifted = cfa_full.shift(area.x(), area.y());
        eprintln!(
            "Active area: x={} y={} {}x{} (full: {}x{})",
            area.x(), area.y(), area.width(), area.height(), full_width, full_h
        );
        (cropped, area.width(), area.height(), cfa_shifted)
    } else {
        let full_h = pixels_full.len() / full_width;
        (pixels_full, full_width, full_h, cfa_full)
    };

    Ok(RawData {
        pixels,
        width,
        height,
        cfa,
        black_level,
        white_level,
        wb_coeffs,
        cam_to_xyz,
        cam_illuminant,
    })
}

/// 3×3 行列の逆行列（行列式が 0 に近い場合は None）
fn mat3x3_inverse(m: &[[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let det = m[0][0] * (m[1][1]*m[2][2] - m[1][2]*m[2][1])
            - m[0][1] * (m[1][0]*m[2][2] - m[1][2]*m[2][0])
            + m[0][2] * (m[1][0]*m[2][1] - m[1][1]*m[2][0]);
    if det.abs() < 1e-10 { return None; }
    let inv_det = 1.0 / det;
    Some([
        [
             (m[1][1]*m[2][2] - m[1][2]*m[2][1]) * inv_det,
            -(m[0][1]*m[2][2] - m[0][2]*m[2][1]) * inv_det,
             (m[0][1]*m[1][2] - m[0][2]*m[1][1]) * inv_det,
        ],
        [
            -(m[1][0]*m[2][2] - m[1][2]*m[2][0]) * inv_det,
             (m[0][0]*m[2][2] - m[0][2]*m[2][0]) * inv_det,
            -(m[0][0]*m[1][2] - m[0][2]*m[1][0]) * inv_det,
        ],
        [
             (m[1][0]*m[2][1] - m[1][1]*m[2][0]) * inv_det,
            -(m[0][0]*m[2][1] - m[0][1]*m[2][0]) * inv_det,
             (m[0][0]*m[1][1] - m[0][1]*m[1][0]) * inv_det,
        ],
    ])
}
