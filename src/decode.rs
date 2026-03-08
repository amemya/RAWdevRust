use std::path::Path;

use rawler::{
    CFA,
    decoders::RawDecodeParams,
    imgop::{Dim2, crop},
    rawimage::{RawImageData, RawPhotometricInterpretation},
    rawsource::RawSource,
};

/// rawlerから取り出したベイヤー配列と必要なメタデータ
pub struct RawData {
    pub pixels: Vec<u16>,
    pub width: usize,
    pub height: usize,
    /// active_area クロップ済みの CFA（シフト適用済み）
    pub cfa: CFA,
    pub black_level: [f32; 4],
    pub white_level: [f32; 4],
    /// カメラホワイトバランス係数 [R, G, B, E] (NaN → 1.0)
    pub wb_coeffs: [f32; 4],
}

pub fn load(path: &Path) -> anyhow::Result<RawData> {
    let rawsource = RawSource::new(path)?;
    let params = RawDecodeParams::default();
    let rawimage = rawler::get_decoder(&rawsource)?.raw_image(&rawsource, &params, false)?;

    let full_width = rawimage.width;

    // ピクセルデータ（u16）
    let pixels_full: Vec<u16> = match rawimage.data {
        RawImageData::Integer(data) => data,
        RawImageData::Float(_) => anyhow::bail!("Float RAW data is not supported yet"),
    };

    // CFA
    let cfa_full = match &rawimage.photometric {
        RawPhotometricInterpretation::Cfa(cfg) => cfg.cfa.clone(),
        other => anyhow::bail!("Unsupported photometric: {:?}", other),
    };

    let black_level = rawimage.blacklevel.as_bayer_array();
    let white_level = rawimage.whitelevel.as_bayer_array();
    let wb_coeffs = rawimage.wb_coeffs.map(|v| if v.is_nan() { 1.0 } else { v });

    // active_area でクロップ（Optical Black 除去）
    let (pixels, width, height, cfa) = if let Some(area) = rawimage.active_area {
        let full_h = pixels_full.len() / full_width;
        let cropped = crop(
            &pixels_full,
            Dim2::new(full_width, full_h),
            area,
        );
        let w = area.width();
        let h = area.height();
        // CFA パターンをクロップ開始位置に合わせてシフト
        let cfa_shifted = cfa_full.shift(area.x(), area.y());
        eprintln!(
            "Active area: x={} y={} {}x{} (full: {}x{})",
            area.x(), area.y(), w, h, full_width, full_h
        );
        (cropped, w, h, cfa_shifted)
    } else {
        eprintln!("No active_area; using full image");
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
    })
}
