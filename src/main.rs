use std::path::PathBuf;

use clap::Parser;

mod color;
mod decode;
mod demosaic;
mod output;

#[derive(Parser)]
#[command(name = "rawdev", about = "RAW image developer")]
struct Cli {
    /// Input RAW file (CR2, NEF, ARW, ORF, ...)
    input: PathBuf,

    /// Output file
    #[arg(short, long)]
    output: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    println!("Input:  {:?}", cli.input);
    println!("Output: {:?}", cli.output);

    // デコード
    let raw = decode::load(&cli.input).expect("Failed to decode RAW file");
    println!("Image: {}x{}, CFA: {:?}", raw.width, raw.height, raw.cfa);

    // デモザイク（RCD）→ linear Camera RGB
    let mut linear = demosaic::rcd::run(&raw);

    // カラーパイプライン (in-place処理により中間Vecアロケーションを削減)
    color::apply_wb(&mut linear, &raw.wb_coeffs);
    color::apply_color_matrix(&mut linear, &raw.cam_to_xyz, raw.cam_illuminant);
    let rgb = color::apply_gamma(&linear);

    // 出力
    output::save_ppm(&rgb, raw.width, raw.height, &cli.output).expect("Failed to write output");

    println!("Done: {:?}", cli.output);
}
