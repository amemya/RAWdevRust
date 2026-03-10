use std::path::PathBuf;

use clap::Parser;

mod color;
mod dcp;
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

    /// DCP Profile (Optional)
    #[arg(long)]
    dcp: Option<PathBuf>,
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
    if let Some(dcp_path) = &cli.dcp {
        println!("Loading DCP profile: {:?}", dcp_path);
        match dcp::load_dcp(dcp_path) {
            Ok(profile) => {
                if let Err(e) = color::apply_dcp(&mut linear, &profile, &raw.wb_coeffs) {
                    eprintln!("Failed to apply DCP: {}. Falling back to default.", e);
                    color::apply_wb(&mut linear, &raw.wb_coeffs);
                    color::apply_color_matrix(&mut linear, &raw.cam_to_xyz, raw.cam_illuminant);
                }
            }
            Err(e) => {
                eprintln!("Failed to load DCP: {}. Falling back to default.", e);
                color::apply_wb(&mut linear, &raw.wb_coeffs);
                color::apply_color_matrix(&mut linear, &raw.cam_to_xyz, raw.cam_illuminant);
            }
        }
    } else {
        color::apply_wb(&mut linear, &raw.wb_coeffs);
        color::apply_color_matrix(&mut linear, &raw.cam_to_xyz, raw.cam_illuminant);
    }

    let rgb = color::apply_gamma(&linear);

    // 出力
    output::save_ppm(&rgb, raw.width, raw.height, &cli.output).expect("Failed to write output");

    println!("Done: {:?}", cli.output);
}
