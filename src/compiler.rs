use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{codegen::generate_go, parser::parse_program, sema::analyze};

pub fn check_file(path: &Path) -> Result<()> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read source file {}", path.display()))?;
    let mut program = parse_program(&source)
        .map_err(|diagnostics| anyhow!(render_diagnostics(path, &source, &diagnostics)))?;
    analyze(&mut program)
        .map_err(|diagnostics| anyhow!(render_diagnostics(path, &source, &diagnostics)))?;
    Ok(())
}

pub fn transpile_file(path: &Path, out_dir: &Path) -> Result<()> {
    let output = transpile_internal(path, out_dir)?;
    println!("generated {}", output.display());
    Ok(())
}

pub fn build_file(path: &Path, out_dir: &Path, out_bin: Option<&Path>) -> Result<()> {
    let go_file = transpile_internal(path, out_dir)?;
    let out_bin = out_bin
        .map(PathBuf::from)
        .unwrap_or_else(|| out_dir.join(default_binary_name(path)));

    let mut cmd = Command::new("go");
    cmd.arg("build").arg("-o").arg(&out_bin).arg(&go_file);
    configure_go_command(&mut cmd, out_dir)?;
    let status = cmd.status().context("failed to execute `go build`")?;

    if !status.success() {
        bail!("go build failed for {}", go_file.display());
    }
    println!("built {}", out_bin.display());
    Ok(())
}

pub fn run_file(path: &Path, out_dir: &Path) -> Result<()> {
    let go_file = transpile_internal(path, out_dir)?;
    let mut cmd = Command::new("go");
    cmd.arg("run").arg(&go_file);
    configure_go_command(&mut cmd, out_dir)?;
    let status = cmd.status().context("failed to execute `go run`")?;

    if !status.success() {
        bail!("go run failed for {}", go_file.display());
    }
    Ok(())
}

fn transpile_internal(path: &Path, out_dir: &Path) -> Result<PathBuf> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read source file {}", path.display()))?;
    let mut program = parse_program(&source)
        .map_err(|diagnostics| anyhow!(render_diagnostics(path, &source, &diagnostics)))?;
    let model = analyze(&mut program)
        .map_err(|diagnostics| anyhow!(render_diagnostics(path, &source, &diagnostics)))?;
    let generated = generate_go(&program, &model);

    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create output directory {}", out_dir.display()))?;

    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("main");
    let go_file = out_dir.join(format!("{stem}.go"));
    fs::write(&go_file, generated)
        .with_context(|| format!("failed to write generated file {}", go_file.display()))?;

    run_gofmt(&go_file)?;
    Ok(go_file)
}

fn run_gofmt(file: &Path) -> Result<()> {
    let status = Command::new("gofmt")
        .arg("-w")
        .arg(file)
        .status()
        .context("failed to execute `gofmt` (required for goplus output)")?;

    if !status.success() {
        bail!("gofmt failed on {}", file.display());
    }
    Ok(())
}

fn configure_go_command(cmd: &mut Command, out_dir: &Path) -> Result<()> {
    let out_dir_abs = if out_dir.is_absolute() {
        out_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read current directory")?
            .join(out_dir)
    };
    let cache_dir = out_dir_abs.join(".gocache");
    let tmp_dir = out_dir_abs.join(".gotmp");
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create Go cache dir {}", cache_dir.display()))?;
    fs::create_dir_all(&tmp_dir)
        .with_context(|| format!("failed to create Go tmp dir {}", tmp_dir.display()))?;
    cmd.env("GOCACHE", cache_dir);
    cmd.env("GOTMPDIR", tmp_dir);
    Ok(())
}

fn render_diagnostics(
    path: &Path,
    source: &str,
    diagnostics: &[crate::diag::Diagnostic],
) -> String {
    diagnostics
        .iter()
        .map(|diag| diag.render(&path.display().to_string(), source))
        .collect::<Vec<_>>()
        .join("\n")
}

fn default_binary_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("goplus");
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use super::transpile_file;

    #[test]
    fn transpile_generates_go_file() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("main.gp");
        fs::write(
            &src,
            r#"
package main

fn main() -> ! {
    return
}
"#,
        )
        .expect("write");
        let out_dir = dir.path().join(".goplusgen");
        let _ = transpile_file(&src, &out_dir);
        let generated = out_dir.join("main.go");
        assert!(generated.exists());
    }

    #[test]
    fn transpile_allows_custom_decorator() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("custom.gp");
        fs::write(
            &src,
            r#"
package main

fn trace(next: func() string, label: string) -> func() string {
    return next
}

@trace("svc")
fn main() -> string {
    return "ok"
}
"#,
        )
        .expect("write");
        let out_dir = dir.path().join(".goplusgen");
        let result = transpile_file(&src, &out_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn default_binary_is_platform_specific() {
        let name = super::default_binary_name(&PathBuf::from("demo.gp"));
        if cfg!(windows) {
            assert!(name.ends_with(".exe"));
        } else {
            assert_eq!(name, "demo");
        }
    }
}
