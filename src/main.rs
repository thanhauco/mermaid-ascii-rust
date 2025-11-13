use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

mod parser;
mod render;

use crate::render::{render_properties, RenderOptions};

#[derive(Parser, Debug)]
#[command(
    name = "mermaid-ascii",
    about = "Generate ASCII diagrams from Mermaid definitions."
)]
struct Cli {
    /// Mermaid file to parse. Use '-' or omit to read from stdin.
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Use ASCII characters only
    #[arg(short = 'a', long = "ascii")]
    use_ascii: bool,

    /// Show coordinate helpers in the output
    #[arg(short, long)]
    coords: bool,

    /// Horizontal space between nodes
    #[arg(short = 'x', long = "paddingX", default_value_t = 5)]
    padding_x: i32,

    /// Vertical space between nodes
    #[arg(short = 'y', long = "paddingY", default_value_t = 5)]
    padding_y: i32,

    /// Padding between text and border
    #[arg(short = 'p', long = "borderPadding", default_value_t = 1)]
    border_padding: i32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut builder = env_logger::Builder::from_default_env();
    if cli.verbose {
        builder.filter_level(log::LevelFilter::Debug);
    } else {
        builder.filter_level(log::LevelFilter::Info);
    }
    builder.init();

    let mut input = String::new();
    match cli.file {
        Some(path) if path.to_string_lossy() != "-" => {
            input = fs::read_to_string(&path)?;
        }
        _ => {
            io::stdin().read_to_string(&mut input)?;
        }
    }

    let mut properties = parser::mermaid_file_to_map(&input, "cli")?;
    properties.padding_x = cli.padding_x;
    properties.padding_y = cli.padding_y;

    let options = RenderOptions {
        border_padding: cli.border_padding,
        use_ascii: cli.use_ascii,
        show_coords: cli.coords,
    };

    let drawing = render_properties(&mut properties, &options)?;
    println!("{}", drawing);
    Ok(())
}
