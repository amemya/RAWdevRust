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
