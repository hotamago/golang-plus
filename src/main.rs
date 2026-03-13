use std::path::PathBuf;

use clap::{Parser, Subcommand};

use goplus::compiler;

#[derive(Parser, Debug)]
#[command(name = "goplus")]
#[command(about = "goplus compiler (transpile goplus -> Go)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Check {
        source: PathBuf,
    },
    Transpile {
        source: PathBuf,
        #[arg(long, default_value = ".goplusgen")]
        out_dir: PathBuf,
    },
    Build {
        source: PathBuf,
        #[arg(long, default_value = ".goplusgen")]
        out_dir: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Run {
        source: PathBuf,
        #[arg(long, default_value = ".goplusgen")]
        out_dir: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { source } => compiler::check_file(&source),
        Command::Transpile { source, out_dir } => compiler::transpile_file(&source, &out_dir),
        Command::Build {
            source,
            out_dir,
            out,
        } => compiler::build_file(&source, &out_dir, out.as_deref()),
        Command::Run { source, out_dir } => compiler::run_file(&source, &out_dir),
    }
}
