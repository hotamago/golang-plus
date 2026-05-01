use super::*;

pub(super) fn resolve_module_go_execution(
    module_out_root: &Path,
    rel_package_dir: &Path,
) -> (PathBuf, Vec<OsString>) {
    let rel = rel_package_dir.to_string_lossy().replace('\\', "/");
    let target = if rel.is_empty() {
        ".".to_string()
    } else {
        format!("./{}", rel.trim_start_matches("./"))
    };
    (module_out_root.to_path_buf(), vec![OsString::from(target)])
}

pub(super) fn resolve_non_module_go_execution(
    package_dir: &Path,
) -> Result<(PathBuf, Vec<OsString>)> {
    Ok((
        package_dir.to_path_buf(),
        collect_go_file_args(package_dir)?,
    ))
}

pub(super) fn collect_go_file_args(package_dir: &Path) -> Result<Vec<OsString>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(package_dir).with_context(|| {
        format!(
            "failed to read generated package dir {}",
            package_dir.display()
        )
    })? {
        let entry =
            entry.with_context(|| format!("failed to inspect {}", package_dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", entry.path().display()))?;
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("go") {
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with("_test.go"))
            .unwrap_or(false)
        {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };
        files.push(file_name.to_os_string());
    }

    files.sort();
    if files.is_empty() {
        bail!(
            "no generated Go files found in package directory {}",
            package_dir.display()
        );
    }
    Ok(files)
}

pub(super) fn resolve_user_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("failed to read current directory")?
            .join(path))
    }
}

pub(super) fn run_gofmt(file: &Path) -> Result<()> {
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

pub(super) fn configure_go_command(cmd: &mut Command, package_dir: &Path) -> Result<()> {
    let package_dir_abs = if package_dir.is_absolute() {
        package_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read current directory")?
            .join(package_dir)
    };
    let cache_dir = package_dir_abs.join(".gocache");
    let tmp_dir = package_dir_abs.join(".gotmp");
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create Go cache dir {}", cache_dir.display()))?;
    fs::create_dir_all(&tmp_dir)
        .with_context(|| format!("failed to create Go tmp dir {}", tmp_dir.display()))?;
    cmd.env("GOCACHE", cache_dir);
    cmd.env("GOTMPDIR", tmp_dir);
    Ok(())
}
pub(super) fn default_binary_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .or_else(|| path.file_name().and_then(|name| name.to_str()))
        .unwrap_or("goplus");
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}
