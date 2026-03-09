pub mod bilinear;
pub mod rcd;

/// リニア値（[0,1]）を sRGB ガンマ変換して u8 に変換する共通関数
#[inline]
pub fn linear_to_srgb(v: f32) -> u8 {
    let c = if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}
