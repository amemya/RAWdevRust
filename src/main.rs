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
    println!(
        "Image: {}x{}, CFA: {:?}",
        raw.width, raw.height, raw.cfa
    );

    // デモザイク（RCD）→ linear Camera RGB
    let cam_rgb = demosaic::rcd::run(&raw);

    // カラーパイプライン
    let wb = color::apply_wb(&cam_rgb, &raw.wb_coeffs);
    let srgb_linear = color::apply_color_matrix(&wb, &raw.cam_to_xyz, raw.cam_to_xyz_is_d65);
    let rgb = color::apply_gamma(&srgb_linear);

    // 出力
    output::save_ppm(&rgb, raw.width, raw.height, &cli.output)
        .expect("Failed to write output");

    println!("Done: {:?}", cli.output);
}
