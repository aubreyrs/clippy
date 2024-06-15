mod util;

use clap::Parser;
use log::error;
use util::{config::Config, logging, processing};

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    config: String,
}

fn main() {
    logging::setup_logging();

    let args = Cli::parse();

    match Config::from_file(&args.config) {
        Ok(config) => {
            if let Err(e) = processing::add_fade_effects(&config) {
                error!("Oops! Something went wrong: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to read config file: {}", e);
        }
    }
}
