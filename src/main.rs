use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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
        #[arg(long, value_enum, default_value_t = DiagnosticFormatArg::Human)]
        diagnostic_format: DiagnosticFormatArg,
    },
    Transpile {
        source: PathBuf,
        #[arg(long, default_value = ".goplusgen")]
        out_dir: PathBuf,
        #[arg(long)]
        emit_source_map: bool,
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
    Fmt {
        source: PathBuf,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        stdout: bool,
    },
    Lint {
        source: PathBuf,
        #[arg(long, value_enum, default_value_t = DiagnosticFormatArg::Human)]
        diagnostic_format: DiagnosticFormatArg,
    },
    Navigate {
        #[arg(long)]
        source_map: PathBuf,
        #[arg(long)]
        file: String,
        #[arg(long)]
        line: usize,
        #[arg(long)]
        column: usize,
        #[arg(long)]
        reverse: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum DiagnosticFormatArg {
    Human,
    Json,
}

impl From<DiagnosticFormatArg> for compiler::DiagnosticFormat {
    fn from(value: DiagnosticFormatArg) -> Self {
        match value {
            DiagnosticFormatArg::Human => compiler::DiagnosticFormat::Human,
            DiagnosticFormatArg::Json => compiler::DiagnosticFormat::Json,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            source,
            diagnostic_format,
        } => compiler::check_file_with_format(&source, diagnostic_format.into()),
        Command::Transpile {
            source,
            out_dir,
            emit_source_map,
        } => compiler::transpile_file_with_options(&source, &out_dir, emit_source_map),
        Command::Build {
            source,
            out_dir,
            out,
        } => compiler::build_file(&source, &out_dir, out.as_deref()),
        Command::Run { source, out_dir } => compiler::run_file(&source, &out_dir),
        Command::Fmt {
            source,
            check,
            stdout,
        } => {
            if check {
                compiler::fmt_check_file(&source)
            } else if stdout {
                compiler::fmt_file_stdout(&source)
            } else {
                compiler::fmt_file(&source)
            }
        }
        Command::Lint {
            source,
            diagnostic_format,
        } => compiler::lint_file_with_format(&source, diagnostic_format.into()),
        Command::Navigate {
            source_map,
            file,
            line,
            column,
            reverse,
        } => compiler::navigate_source_map(&source_map, &file, line, column, reverse),
    }
}
