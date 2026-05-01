use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PACKAGE_CACHE_FILE_NAME: &str = ".goplus-package-cache.json";

pub(super) fn transpile_internal(
    path: &Path,
    out_dir: &Path,
    emit_source_map: bool,
) -> Result<PreparedPackage> {
    let (project, model) = load_analyzed_project(path)?;

    let (package_dir, generated_file, go_work_dir, go_args) = if let Some(module_root) =
        &project.module_root
    {
        let module_name = read_go_module_name(module_root)?;
        let module_out_root = resolve_output_module_root(module_root, out_dir)?;
        transpile_module_projects(
            &project,
            &model,
            module_root,
            &module_name,
            &module_out_root,
        )?;

        let rel_package_dir = project
            .source_dir
            .strip_prefix(module_root)
            .with_context(|| {
                format!(
                    "failed to resolve package path {} inside module {}",
                    project.source_dir.display(),
                    module_root.display()
                )
            })?;
        let package_dir = if rel_package_dir.as_os_str().is_empty() {
            module_out_root.clone()
        } else {
            module_out_root.join(rel_package_dir)
        };
        let generated_file = package_dir.join(GENERATED_GO_FILE_NAME);
        let (go_work_dir, go_args) = resolve_module_go_execution(&module_out_root, rel_package_dir);
        (package_dir, generated_file, go_work_dir, go_args)
    } else {
        let package_dir = resolve_output_package_dir(&project, out_dir)?;
        let generated_file = emit_project_package(&project, &model, &package_dir, true)?;
        let (go_work_dir, go_args) = resolve_non_module_go_execution(&package_dir)?;
        (package_dir, generated_file, go_work_dir, go_args)
    };

    let source_map = if emit_source_map {
        Some(source_map::build_source_map(&project, &generated_file)?)
    } else {
        None
    };

    Ok(PreparedPackage {
        package_dir,
        generated_file,
        go_work_dir,
        go_args,
        source_map,
    })
}

pub(super) fn emit_project_package(
    project: &Project,
    model: &SemanticModel,
    package_dir: &Path,
    copy_go_files: bool,
) -> Result<PathBuf> {
    let merged_program = merge_programs(
        &project.package_name,
        project.units.iter().map(|unit| &unit.program),
    );
    let generated = generate_go(&merged_program, model);
    fs::create_dir_all(package_dir).with_context(|| {
        format!(
            "failed to create output directory {}",
            package_dir.display()
        )
    })?;
    let generated_file = package_dir.join(GENERATED_GO_FILE_NAME);
    let cache_key = package_cache_key(project, &generated)?;
    if package_cache_is_fresh(
        project,
        package_dir,
        &generated_file,
        &cache_key,
        copy_go_files,
    ) {
        return Ok(generated_file);
    }

    if copy_go_files {
        copy_go_support_files(&project.go_files, package_dir)?;
    }

    fs::write(&generated_file, generated).with_context(|| {
        format!(
            "failed to write generated file {}",
            generated_file.display()
        )
    })?;
    run_gofmt(&generated_file)?;
    write_package_cache(package_dir, &cache_key)?;
    Ok(generated_file)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageCacheManifest {
    version: u8,
    compiler_version: String,
    cache_key: String,
}

fn package_cache_key(project: &Project, generated: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"goplus-package-cache-v1");
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(project.package_name.as_bytes());
    hasher.update(generated.as_bytes());

    for unit in &project.units {
        hasher.update(unit.path.display().to_string().as_bytes());
        hasher.update(unit.source.as_bytes());
    }
    for go_file in &project.go_files {
        hasher.update(go_file.display().to_string().as_bytes());
        let content = fs::read(go_file)
            .with_context(|| format!("failed to read Go support file {}", go_file.display()))?;
        hasher.update(content);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn package_cache_is_fresh(
    project: &Project,
    package_dir: &Path,
    generated_file: &Path,
    cache_key: &str,
    copy_go_files: bool,
) -> bool {
    if !generated_file.is_file() {
        return false;
    }
    if copy_go_files
        && project.go_files.iter().any(|source| {
            source
                .file_name()
                .map(|name| !package_dir.join(name).is_file())
                .unwrap_or(true)
        })
    {
        return false;
    }

    let manifest_path = package_dir.join(PACKAGE_CACHE_FILE_NAME);
    let Ok(content) = fs::read_to_string(manifest_path) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_str::<PackageCacheManifest>(&content) else {
        return false;
    };
    manifest.version == 1
        && manifest.compiler_version == env!("CARGO_PKG_VERSION")
        && manifest.cache_key == cache_key
}

fn write_package_cache(package_dir: &Path, cache_key: &str) -> Result<()> {
    let manifest = PackageCacheManifest {
        version: 1,
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        cache_key: cache_key.to_string(),
    };
    let content = serde_json::to_string_pretty(&manifest)?;
    fs::write(
        package_dir.join(PACKAGE_CACHE_FILE_NAME),
        format!("{content}\n"),
    )
    .with_context(|| {
        format!(
            "failed to write package cache manifest in {}",
            package_dir.display()
        )
    })?;
    Ok(())
}
