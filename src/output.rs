use std::io::{BufWriter, Write};
use std::path::Path;
use little_exif::metadata::Metadata;
use little_exif::exif_tag::ExifTag;
use little_exif::rational::uR64;

/// RGBRGB... バッファを PPM ファイルとして保存
pub fn save_ppm(rgb: &[u8], width: usize, height: usize, path: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "P6\n{} {}\n255\n", width, height)?;
    writer.write_all(rgb)?;
    Ok(())
}

/// RGBRGB... バッファを PNG ファイルとして保存 (PNG-24: 8-bit RGB)
pub fn save_png(rgb: &[u8], width: usize, height: usize, path: &Path, exif_info: &crate::decode::ExifInfo) -> anyhow::Result<()> {
    image::save_buffer(
        path,
        rgb,
        width as u32,
        height as u32,
        image::ColorType::Rgb8,
    )?;

    let mut metadata = Metadata::new();
    
    if let Some(make) = &exif_info.make {
        metadata.set_tag(ExifTag::Make(make.clone()));
    }
    if let Some(model) = &exif_info.model {
        metadata.set_tag(ExifTag::Model(model.clone()));
    }
    if let Some(dt) = &exif_info.datetime {
        metadata.set_tag(ExifTag::DateTimeOriginal(dt.clone()));
    }
    if let Some(iso) = exif_info.iso {
        metadata.set_tag(ExifTag::ISO(vec![iso as u16]));
    }
    if let Some((n, d)) = exif_info.f_number {
        metadata.set_tag(ExifTag::FNumber(vec![uR64 { nominator: n, denominator: d }]));
    }
    if let Some((n, d)) = exif_info.exposure_time {
        metadata.set_tag(ExifTag::ExposureTime(vec![uR64 { nominator: n, denominator: d }]));
    }
    if let Some((n, d)) = exif_info.focal_length {
        metadata.set_tag(ExifTag::FocalLength(vec![uR64 { nominator: n, denominator: d }]));
    }

    metadata.write_to_file(path).map_err(|e| anyhow::anyhow!("Failed to write EXIF: {:?}", e))?;

    Ok(())
}
