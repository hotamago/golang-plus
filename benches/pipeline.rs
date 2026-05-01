use criterion::{Criterion, criterion_group, criterion_main};
use goplus::{codegen::generate_go, parser::parse_program, sema::analyze};

const SAMPLE: &str = r#"
package main

import "fmt"

@derive(String)
enum Result<T> {
    Ok(T)
    Err(string)
}

@memoize
fn fib(n: int) -> int {
    if n < 2 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

fn label(r: Result<int>) -> string {
    match r {
        Ok(v) => fmt.Sprintf("ok: %d", v),
        Err(e) => e,
    }
}
"#;

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_sample", |b| {
        b.iter(|| parse_program(SAMPLE).expect("parse sample"))
    });
}

fn bench_analyze(c: &mut Criterion) {
    c.bench_function("analyze_sample", |b| {
        b.iter(|| {
            let mut program = parse_program(SAMPLE).expect("parse sample");
            analyze(&mut program).expect("analyze sample")
        })
    });
}

fn bench_codegen(c: &mut Criterion) {
    c.bench_function("codegen_sample", |b| {
        b.iter(|| {
            let mut program = parse_program(SAMPLE).expect("parse sample");
            let model = analyze(&mut program).expect("analyze sample");
            generate_go(&program, &model)
        })
    });
}

criterion_group!(pipeline, bench_parse, bench_analyze, bench_codegen);
criterion_main!(pipeline);
