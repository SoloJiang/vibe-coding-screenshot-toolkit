use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about = "Screenshot Tool Dev CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Version,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Version) | None => {
            println!("version: {}", env!("CARGO_PKG_VERSION"));
        }
    }
}
