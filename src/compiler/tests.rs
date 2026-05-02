use std::{fs, path::PathBuf, process::Command};

use tempfile::tempdir;

use super::{
    DiagnosticFormat, GENERATED_GO_FILE_NAME, build_file, check_file_with_format, fmt_check_file,
    transpile_file, transpile_file_with_options,
};

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
    let result = transpile_file(&src, &out_dir);
    assert!(result.is_ok());
    let generated = out_dir.join(GENERATED_GO_FILE_NAME);
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
fn transpile_merges_multi_file_package_and_copies_go_sources() {
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("main.gp"),
        r#"
package main

fn main() -> ! {
    printMessage()
    return
}
"#,
    )
    .expect("write gp");
    fs::write(
        dir.path().join("helper.gp"),
        r#"
package main

fn printMessage() {
    fmt.Println(messageFromGo())
}
"#,
    )
    .expect("write helper");
    fs::write(
        dir.path().join("bridge.go"),
        r#"
package main

func messageFromGo() string {
    return "hello from go"
}
"#,
    )
    .expect("write go");

    let out_dir = dir.path().join(".goplusgen");
    let result = transpile_file(&dir.path().join("main.gp"), &out_dir);
    assert!(result.is_ok());

    let generated = fs::read_to_string(out_dir.join(GENERATED_GO_FILE_NAME)).expect("generated");
    assert!(generated.contains("func mainWarp() error"));
    assert!(generated.contains("func printMessage()"));
    assert!(out_dir.join("bridge.go").exists());
}

#[test]
fn transpile_mirrors_module_relative_output_layout() {
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("go.mod"),
        "module example.com/demo\n\ngo 1.23.0\n",
    )
    .expect("write go.mod");
    let src_dir = dir.path().join("cmd").join("demo");
    fs::create_dir_all(&src_dir).expect("mkdir");
    fs::write(
        src_dir.join("main.gp"),
        r#"
package main

fn main() -> ! {
    return
}
"#,
    )
    .expect("write gp");

    let out_root = dir.path().join(".goplusgen");
    let result = transpile_file(&src_dir.join("main.gp"), &out_root);
    assert!(result.is_ok());
    assert!(
        out_root
            .join("cmd")
            .join("demo")
            .join(GENERATED_GO_FILE_NAME)
            .exists()
    );
}

#[test]
fn build_links_gp_and_go_sources() {
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("main.gp"),
        r#"
package main

import "fmt"

fn main() -> ! {
    fmt.Println(fromGo())
    return
}
"#,
    )
    .expect("write main");
    fs::write(
        dir.path().join("helper.gp"),
        r#"
package main

fn fromGp() -> string {
    return "world"
}
"#,
    )
    .expect("write helper");
    fs::write(
        dir.path().join("bridge.go"),
        r#"
package main

func fromGo() string {
    return "hello " + fromGp()
}
"#,
    )
    .expect("write bridge");

    let out_dir = dir.path().join(".goplusgen");
    let out_bin = dir
        .path()
        .join(if cfg!(windows) { "demo.exe" } else { "demo" });
    let result = build_file(&dir.path().join("main.gp"), &out_dir, Some(&out_bin));
    assert!(result.is_ok());
    assert!(out_bin.exists());
}

#[test]
fn build_nested_module_package_with_imports_and_mixed_sources() {
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("go.mod"),
        "module example.com/demo\n\ngo 1.23.0\n",
    )
    .expect("write go.mod");

    let app_dir = dir.path().join("cmd").join("demo");
    let support_dir = dir.path().join("pkg").join("support");
    let label_dir = dir.path().join("internal").join("label");
    fs::create_dir_all(&app_dir).expect("mkdir app");
    fs::create_dir_all(&support_dir).expect("mkdir support");
    fs::create_dir_all(&label_dir).expect("mkdir label");

    fs::write(
        app_dir.join("main.gp"),
        r#"
package main

import "fmt"
import "example.com/demo/pkg/support"

fn main() -> ! {
    fmt.Println(buildBanner("goplus"))
    fmt.Println(callFromGoFile())
    fmt.Println(support.FromNestedPackages())
    return
}
"#,
    )
    .expect("write main");
    fs::write(
        app_dir.join("messages.gp"),
        r#"
package main

fn buildBanner(name: string) -> string {
    return "[same package gp] hello, " + name
}

fn fromGpFile() -> string {
    return "same-folder .gp files share one package namespace"
}
"#,
    )
    .expect("write messages");
    fs::write(
        app_dir.join("bridge.go"),
        r#"
package main

func callFromGoFile() string {
    return "[same package go] " + fromGpFile()
}
"#,
    )
    .expect("write bridge");
    fs::write(
        support_dir.join("support.gp"),
        r#"
package support

import "example.com/demo/internal/label"

fn FromNestedPackages() -> string {
    return label.Prefix() + " support.FromNestedPackages()"
}
"#,
    )
    .expect("write support");
    fs::write(
        label_dir.join("label.gp"),
        r#"
package label

fn Prefix() -> string {
    return "[imported package]"
}
"#,
    )
    .expect("write label");

    let out_root = dir.path().join(".goplusgen");
    let out_bin = dir.path().join(if cfg!(windows) {
        "nested.exe"
    } else {
        "nested"
    });
    let result = build_file(&app_dir.join("main.gp"), &out_root, Some(&out_bin));
    assert!(result.is_ok());
    assert!(
        out_root
            .join("cmd")
            .join("demo")
            .join(GENERATED_GO_FILE_NAME)
            .exists()
    );
    assert!(
        out_root
            .join("pkg")
            .join("support")
            .join(GENERATED_GO_FILE_NAME)
            .exists()
    );
    assert!(
        out_root
            .join("internal")
            .join("label")
            .join(GENERATED_GO_FILE_NAME)
            .exists()
    );
    assert!(out_bin.exists());

    let output = Command::new(&out_bin).output().expect("run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[same package gp] hello, goplus"));
    assert!(stdout.contains("[same package go] same-folder .gp files share one package namespace"));
    assert!(stdout.contains("[imported package] support.FromNestedPackages()"));
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

#[test]
fn transpile_can_emit_source_map_stub() {
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
    let result = transpile_file_with_options(&src, &out_dir, true);
    assert!(result.is_ok());
    let map_path = out_dir.join("zz_goplus_gen.go.map");
    assert!(map_path.exists());
    let map = fs::read_to_string(map_path).expect("source map");
    assert!(map.contains(r#""sources""#));
    assert!(map.contains(r#""mappings""#));
    assert!(map.contains(r#""kind": "function""#));
    assert!(!map.contains(r#""mappings": []"#));
}

#[test]
fn transpile_writes_package_cache_manifest() {
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
    assert!(transpile_file(&src, &out_dir).is_ok());
    assert!(out_dir.join(".goplus-package-cache.json").exists());
    assert!(transpile_file(&src, &out_dir).is_ok());
}

#[test]
fn build_handles_import_alias_raw_decls_and_generic_tagged_enums() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("main.gp");
    fs::write(
        &src,
        r#"
package main

import f "fmt"

const banner = "value"
type Label = string

enum Result<T> {
    Ok(T)
    Err(string)
}

fn load() -> Result<int> {
    return Result<int>::Ok(7)
}

fn main() -> ! {
    result := load()
    match result {
        Ok(v) => {
            f.Println(banner, Label("ok"), v)
        },
        Err(e) => {
            f.Println(e)
        },
    }
    return
}
"#,
    )
    .expect("write");
    let out_dir = dir.path().join(".goplusgen");
    let out_bin = dir.path().join(if cfg!(windows) {
        "generic.exe"
    } else {
        "generic"
    });
    let result = build_file(&src, &out_dir, Some(&out_bin));
    assert!(result.is_ok());
    let generated = fs::read_to_string(out_dir.join(GENERATED_GO_FILE_NAME)).expect("generated");
    assert!(generated.contains("f \"fmt\""));
    assert!(generated.contains("type Result[T any] struct"));
    assert!(generated.contains("ResultOk[int](7)"));
}

#[test]
fn check_supports_json_diagnostics() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("main.gp");
    fs::write(
        &src,
        r#"
package main

fn run() -> string {
    value := load()?
    return value
}
"#,
    )
    .expect("write");
    let err = check_file_with_format(&src, DiagnosticFormat::Json).expect_err("check fails");
    let rendered = err.to_string();
    assert!(rendered.contains("\"diagnostics\""));
    assert!(rendered.contains("\"code\""));
}

#[test]
fn fmt_check_validates_without_rewriting() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("main.gp");
    fs::write(&src, "package main\n\nfn main() -> ! {\n\treturn\n}\n").expect("write");
    assert!(fmt_check_file(&src).is_ok());
}
