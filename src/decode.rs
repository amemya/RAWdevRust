use std::path::Path;

use rawler::{
    CFA,
    decoders::RawDecodeParams,
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
}

pub fn load(path: &Path) -> anyhow::Result<RawData> {
    let rawsource = RawSource::new(path)?;
    let params = RawDecodeParams::default();
    let rawimage = rawler::get_decoder(&rawsource)?.raw_image(&rawsource, &params, false)?;

    let width = rawimage.width;
    let height = rawimage.height;

    let pixels: Vec<u16> = match rawimage.data {
        RawImageData::Integer(data) => data,
        RawImageData::Float(_) => anyhow::bail!("Float RAW data is not supported yet"),
    };

    let cfa = match &rawimage.photometric {
        RawPhotometricInterpretation::Cfa(cfg) => cfg.cfa.clone(),
        other => anyhow::bail!("Unsupported photometric: {:?}", other),
    };

    let black_level = rawimage.blacklevel.as_bayer_array();
    let white_level = rawimage.whitelevel.as_bayer_array();
    let wb_coeffs = rawimage.wb_coeffs.map(|v| if v.is_nan() { 1.0 } else { v });

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
