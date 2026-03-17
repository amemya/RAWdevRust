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

    let apply_default_pipeline = |pixels: &mut [f32]| {
        color::apply_wb(pixels, &raw.wb_coeffs);
        color::apply_color_matrix(pixels, &raw.cam_to_xyz, raw.cam_illuminant);
    };

    // カラーパイプライン (in-place処理により中間Vecアロケーションを削減)
    let dcp_path = cli.dcp.or_else(|| {
        println!("No DCP profile specified. Searching for default Adobe profile for {} {}...", raw.make, raw.model);
        dcp::find_default_dcp(&raw.make, &raw.model)
    });

    if let Some(path) = dcp_path {
        println!("Loading DCP profile: {:?}", path);
        match dcp::load_dcp(&path) {
            Ok(profile) => {
                if let Err(e) = color::apply_dcp(&mut linear, &profile, &raw.wb_coeffs) {
                    eprintln!("Failed to apply DCP: {}. Falling back to default.", e);
                    apply_default_pipeline(&mut linear);
                }
            }
            Err(e) => {
                eprintln!("Failed to load DCP: {}. Falling back to default.", e);
                apply_default_pipeline(&mut linear);
            }
        }
    } else {
        println!("No DCP profile found. Using default color matrix.");
        apply_default_pipeline(&mut linear);
    }

    let rgb = color::apply_gamma(&linear);

    // 出力
    output::save_ppm(&rgb, raw.width, raw.height, &cli.output).expect("Failed to write output");

    println!("Done: {:?}", cli.output);
}
