use std::io::{BufWriter, Write};
use std::path::Path;

/// RGBRGB... バッファを PPM ファイルとして保存
pub fn save_ppm(rgb: &[u8], width: usize, height: usize, path: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "P6\n{} {}\n255\n", width, height)?;
    writer.write_all(rgb)?;
    Ok(())
}

/// RGBRGB... バッファを PNG ファイルとして保存 (PNG-24: 8-bit RGB)
pub fn save_png(rgb: &[u8], width: usize, height: usize, path: &Path) -> anyhow::Result<()> {
    image::save_buffer(
        path,
        rgb,
        width as u32,
        height as u32,
        image::ColorType::Rgb8,
    )?;
    Ok(())
}
