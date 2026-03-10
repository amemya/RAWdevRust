use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

/// DCP Profile containing matrices, hue/sat maps, and tone curve.
#[derive(Debug, Default)]
pub struct DcpProfile {
    pub illuminant1: Option<u16>,
    pub illuminant2: Option<u16>,
    pub color_matrix1: Option<Vec<f32>>,
    pub color_matrix2: Option<Vec<f32>>,
    pub forward_matrix1: Option<Vec<f32>>,
    pub forward_matrix2: Option<Vec<f32>>,
    // We will add tone curves and HSL maps later
}

const TAG_CALIBRATION_ILLUMINANT_1: u16 = 50778;
const TAG_CALIBRATION_ILLUMINANT_2: u16 = 50779;
const TAG_COLOR_MATRIX_1: u16 = 50721;
const TAG_COLOR_MATRIX_2: u16 = 50722;
const TAG_FORWARD_MATRIX_1: u16 = 50964;
const TAG_FORWARD_MATRIX_2: u16 = 50965;
const TAG_PROFILE_HUE_SAT_MAP_DIMS: u16 = 50937;
const TAG_PROFILE_HUE_SAT_MAP_DATA_1: u16 = 50938;
const TAG_PROFILE_HUE_SAT_MAP_DATA_2: u16 = 50939;
const TAG_PROFILE_TONE_CURVE: u16 = 50940;

pub fn load_dcp(path: &Path) -> Result<DcpProfile> {
    let file = File::open(path).context("Failed to open DCP file")?;
    let mut decoder = Decoder::new(file).context("Failed to initialize TIFF decoder")?;

    let mut profile = DcpProfile::default();

    // Just testing if we can read tags and what the tiff crate returns
    // For now we just return an empty profile
    // decoder.get_tag() is the usual way

    Ok(profile)
}
