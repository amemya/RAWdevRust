use std::path::PathBuf;

use clap::Parser;

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

    // デモザイク（RCD）
    let rgb = demosaic::rcd::run(&raw);

    // 出力
    output::save_ppm(&rgb, raw.width, raw.height, &cli.output)
        .expect("Failed to write output");

    println!("Done: {:?}", cli.output);
}
