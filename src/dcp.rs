use anyhow::Result;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Default)]
pub struct DcpProfile {
    pub illuminant1: Option<u16>,
    pub illuminant2: Option<u16>,
    pub color_matrix1: Option<Vec<f32>>,
    pub color_matrix2: Option<Vec<f32>>,
    pub forward_matrix1: Option<Vec<f32>>,
    pub forward_matrix2: Option<Vec<f32>>,
    pub tone_curve: Option<Vec<f32>>,
    pub map_dims: Option<[u32; 3]>,
    pub hsl_map1: Option<Vec<f32>>,
    pub hsl_map2: Option<Vec<f32>>,
    pub look_table_dims: Option<[u32; 3]>,
    pub look_table_data: Option<Vec<f32>>,
}

const TAG_CALIBRATION_ILLUMINANT_1: u16 = 50778;
const TAG_CALIBRATION_ILLUMINANT_2: u16 = 50779;
const TAG_COLOR_MATRIX_1: u16 = 50721;
const TAG_COLOR_MATRIX_2: u16 = 50722;
const TAG_FORWARD_MATRIX_1: u16 = 50964;
const TAG_FORWARD_MATRIX_2: u16 = 50965;
const TAG_PROFILE_HUE_SAT_MAP_DIMS: u16 = 50937;
const _TAG_PROFILE_HUE_SAT_MAP_DATA_1: u16 = 50938;
const _TAG_PROFILE_HUE_SAT_MAP_DATA_2: u16 = 50939;
const TAG_PROFILE_TONE_CURVE: u16 = 50940;

/// Read a little endian u16
fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Read a little endian u32
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_i32_le(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_f32_le(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub fn load_dcp(path: &Path) -> Result<DcpProfile> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    if buffer.len() < 8 {
        anyhow::bail!("File too small to be DCP/TIFF");
    }

    let _is_little_endian = match &buffer[0..2] {
        b"II" => true,
        b"MM" => anyhow::bail!("Big-endian TIFF not supported yet"),
        _ => anyhow::bail!("Invalid TIFF endianness marker"),
    };

    // magic number check (allow IIRC or 0x2A for standard TIFF)
    let magic = read_u16_le(&buffer, 2);
    if magic == 43 {
        anyhow::bail!("BigTIFF (magic 43) is not supported");
    } else if magic != 42 && magic != 0x4352 /* IIRC */ {
        anyhow::bail!("Invalid TIFF magic number: {}", magic);
    }

    let ifd_offset = read_u32_le(&buffer, 4) as usize;
    if ifd_offset + 2 > buffer.len() {
        anyhow::bail!("IFD offset out of bounds");
    }

    let mut profile = DcpProfile::default();
    
    // Read 0th IFD
    let num_entries = read_u16_le(&buffer, ifd_offset) as usize;
    
    for i in 0..num_entries {
        let entry_offset = ifd_offset + 2 + i * 12;
        if entry_offset + 12 > buffer.len() {
            break;
        }

        let tag = read_u16_le(&buffer, entry_offset);
        let typ = read_u16_le(&buffer, entry_offset + 2);
        let count = read_u32_le(&buffer, entry_offset + 4) as usize;
        let value_offset_or_data = read_u32_le(&buffer, entry_offset + 8);

        let data_size = match typ {
            1 | 2 | 6 | 7 => 1,      // BYTE, ASCII, SBYTE, UNDEFINED
            3 | 8 => 2,              // SHORT, SSHORT
            4 | 9 | 11 => 4,         // LONG, SLONG, FLOAT
            5 | 10 | 12 => 8,        // RATIONAL, SRATIONAL, DOUBLE
            _ => 0,
        };

        let Some(total_size) = count.checked_mul(data_size) else {
            anyhow::bail!("IFD entry data size overflowed");
        };
        
        let data_slice = if total_size <= 4 {
            // Data is inline
            &buffer[entry_offset + 8 .. entry_offset + 8 + total_size]
        } else {
            // Data is at offset
            let offset = value_offset_or_data as usize;
            if offset.checked_add(total_size).map_or(true, |end| end > buffer.len()) {
                continue; // Skip out of bounds
            }
            &buffer[offset .. offset + total_size]
        };

        match tag {
            TAG_CALIBRATION_ILLUMINANT_1 => {
                if typ == 3 && count >= 1 {
                    profile.illuminant1 = Some(read_u16_le(data_slice, 0));
                }
            }
            TAG_CALIBRATION_ILLUMINANT_2 => {
                if typ == 3 && count >= 1 {
                    profile.illuminant2 = Some(read_u16_le(data_slice, 0));
                }
            }
            TAG_COLOR_MATRIX_1 | TAG_COLOR_MATRIX_2 | TAG_FORWARD_MATRIX_1 | TAG_FORWARD_MATRIX_2 => {
                if typ == 10 && count >= 1 { // SRATIONAL
                    let mut mat = Vec::with_capacity(count);
                    for j in 0..count {
                        let num = read_i32_le(data_slice, j * 8);
                        let den = read_i32_le(data_slice, j * 8 + 4);
                        if den == 0 {
                            anyhow::bail!("Invalid DCP: Matrix rational denominator is zero");
                        }
                        mat.push(num as f32 / den as f32);
                    }
                    match tag {
                        TAG_COLOR_MATRIX_1 => profile.color_matrix1 = Some(mat),
                        TAG_COLOR_MATRIX_2 => profile.color_matrix2 = Some(mat),
                        TAG_FORWARD_MATRIX_1 => profile.forward_matrix1 = Some(mat),
                        TAG_FORWARD_MATRIX_2 => profile.forward_matrix2 = Some(mat),
                        _ => unreachable!(),
                    }
                }
            }
            TAG_PROFILE_TONE_CURVE => {
                if typ == 11 && count >= 1 { // FLOAT
                    let mut curve = Vec::with_capacity(count);
                    for j in 0..count {
                        curve.push(read_f32_le(data_slice, j * 4));
                    }
                    profile.tone_curve = Some(curve);
                }
            }
            TAG_PROFILE_HUE_SAT_MAP_DIMS => {
                if typ == 4 && count == 3 { // LONG
                    let h = read_u32_le(data_slice, 0);
                    let s = read_u32_le(data_slice, 4);
                    let v = read_u32_le(data_slice, 8);
                    profile.map_dims = Some([h, s, v]);
                }
            }
            _TAG_PROFILE_HUE_SAT_MAP_DATA_1 | _TAG_PROFILE_HUE_SAT_MAP_DATA_2 => {
                if typ == 11 && count >= 1 { // FLOAT
                    let mut map = Vec::with_capacity(count);
                    for j in 0..count {
                        map.push(read_f32_le(data_slice, j * 4));
                    }
                    if tag == _TAG_PROFILE_HUE_SAT_MAP_DATA_1 {
                        profile.hsl_map1 = Some(map);
                    } else {
                        profile.hsl_map2 = Some(map);
                    }
                }
            }
            50981 => { // ProfileLookTableDims
                if typ == 4 && count == 3 {
                    let h = read_u32_le(data_slice, 0);
                    let s = read_u32_le(data_slice, 4);
                    let v = read_u32_le(data_slice, 8);
                    profile.look_table_dims = Some([h, s, v]);
                }
            }
            50982 => { // ProfileLookTableData
                if typ == 11 && count >= 1 {
                    let mut map = Vec::with_capacity(count);
                    for j in 0..count {
                        map.push(read_f32_le(data_slice, j * 4));
                    }
                    profile.look_table_data = Some(map);
                }
            }
            _ => {}
        }
    }

    Ok(profile)
}

/// 自動検出：Mac/Windows の標準的な Adobe Camera Raw プロファイルパスから、
/// 指定されたカメラの "Adobe Standard" プロファイルを探索して返します。
pub fn find_default_dcp(make: &str, model: &str) -> Option<std::path::PathBuf> {
    let clean_make = make.trim();
    let clean_model = model.trim();

    // 探索するルートディレクトリ (macOS と Windows)
    let mut search_paths = Vec::new();

    // macOS System
    search_paths.push(std::path::PathBuf::from("/Library/Application Support/Adobe/CameraRaw/CameraProfiles"));
    // macOS User
    if let Some(home) = std::env::var_os("HOME") {
        let mut p = std::path::PathBuf::from(home);
        p.push("Library/Application Support/Adobe/CameraRaw/CameraProfiles");
        search_paths.push(p);
    }
    // Windows ProgramData
    if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
        let mut p = std::path::PathBuf::from(program_data);
        p.push("Adobe/CameraRaw/CameraProfiles");
        search_paths.push(p);
    }
    // Windows AppData
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let mut p = std::path::PathBuf::from(app_data);
        p.push("Adobe/CameraRaw/CameraProfiles");
        search_paths.push(p);
    }

    // "Maker Model Adobe Standard.dcp" の形式等を想定して探索
    for base_dir in search_paths {
        if !base_dir.exists() {
            continue;
        }

        let target_filename = format!("{} {} Adobe Standard.dcp", clean_make, clean_model);
        let target_filename_fallback = format!("{} Adobe Standard.dcp", clean_model); // Makeが省略されるパターン

        // 高速化のためのダイレクトルックアップ:
        // Adobe Standardプロファイルは通常 "Adobe Standard" サブディレクトリ直下にある
        let direct_paths = [
            base_dir.join("Adobe Standard").join(&target_filename),
            base_dir.join("Adobe Standard").join(&target_filename_fallback),
            base_dir.join(&target_filename),
            base_dir.join(&target_filename_fallback),
        ];

        for direct_path in direct_paths {
            if direct_path.is_file() {
                return Some(direct_path);
            }
        }

        // 見つからない場合は CameraProfiles ディレクトリツリー全体を深さ優先・深さ制限で再帰探索する
        // (最大3階層までに制限して起動時のI/O遅延を防ぐ)
        let walk_dir = walkdir::WalkDir::new(&base_dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walk_dir {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if path.is_file() && ext.eq_ignore_ascii_case("dcp") {
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                
                // 完全一致に近い条件で探す (Mark III などの誤爆を防ぐため)
                if file_name == target_filename || file_name == target_filename_fallback {
                    return Some(path.to_path_buf());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Requires local DCP profile assets to be present"]
    fn test_parse_dcp() {
        let path = PathBuf::from("assets/profiles/Canon EOS-1D X/Camera/Canon EOS-1D X/Canon EOS-1D X Camera Standard.dcp");
        assert!(path.exists());
        let profile = load_dcp(&path).unwrap();
        if let Some(curve) = profile.tone_curve {
            print!("pub const ADOBE_DEFAULT_CURVE: &[f32] = &[");
            for (i, v) in curve.iter().enumerate() {
                if i % 8 == 0 { println!(); print!("    "); }
                print!("{:.6}, ", v);
            }
            println!("\n];");
        }
    }
}
