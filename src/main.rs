use std::path::PathBuf;

use clap::{Parser, ValueEnum};

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

    /// Output Color Space
    #[arg(long, value_enum, default_value_t = ColorSpaceOpt::Srgb)]
    color_space: ColorSpaceOpt,
}

#[derive(ValueEnum, Clone, Debug)]
enum ColorSpaceOpt {
    Srgb,
    P3,
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

    // カラースペースの選択
    let target_color_space = match cli.color_space {
        ColorSpaceOpt::Srgb => color::TargetColorSpace::Srgb,
        ColorSpaceOpt::P3 => color::TargetColorSpace::DisplayP3,
    };

    let apply_default_pipeline = |pixels: &mut [f32]| {
        color::apply_wb(pixels, &raw.wb_coeffs);
        color::apply_color_matrix(pixels, &raw.cam_to_xyz, raw.cam_illuminant, target_color_space);
    };

    // カラーパイプライン (in-place処理により中間Vecアロケーションを削減)
    let dcp_path = cli.dcp.as_ref().cloned().or_else(|| {
        println!("No DCP profile specified. Searching for default Adobe profile for {} {}...", &raw.make, &raw.model);
        dcp::find_default_dcp(&raw.make, &raw.model)
    });

    if let Some(path) = dcp_path {
        println!("Loading DCP profile: {:?}", path);
        match dcp::load_dcp(&path) {
            Ok(profile) => {
                if let Err(e) = color::apply_dcp(&mut linear, &profile, &raw.wb_coeffs, target_color_space) {
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

    // 出力フォーマットの出し分け
    let ext = cli.output.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "" || ext.eq_ignore_ascii_case("ppm") {
        output::save_ppm(&rgb, raw.width, raw.height, &cli.output).expect("Failed to write PPM output");
    } else if ext.eq_ignore_ascii_case("png") {
        output::save_png(&rgb, raw.width, raw.height, &cli.output, &raw.exif, target_color_space).expect("Failed to write PNG output");
    } else {
        eprintln!("Unsupported output extension: {}", ext);
        eprintln!("Writing as PPM by default...");
        output::save_ppm(&rgb, raw.width, raw.height, &cli.output.with_extension("ppm")).expect("Failed to write PPM output");
    }

    println!("Done: {:?}", cli.output);
}
