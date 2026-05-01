package main

/*
Complex integration example for compiler/CI coverage.
It intentionally combines grouped imports, derive, impl, tagged enums,
match, decorators, memoization, error sugar, and raw Go-like statements.
*/
import (
    "fmt"
    "strings"
)

@derive(String)
struct Job {
    Name: string
    Attempts: int
}

impl Job {
    fn Label(self) -> string {
        return self.Name
    }
}

enum Outcome {
    Ok(string)
    Err(string)
}

fn trace(next: func(name string) (string, error), label: string) -> func(name string) (string, error) {
    return func(name string) (string, error) {
        fmt.Println("trace:", label, name)
        return next(name)
    }
}

@trace("normalize")
@retry(2)
@log
fn normalize(name: string) -> string! {
    if len(name) == 0 {
        return error("empty name")
    }
    return strings.ToUpper(name)
}

@memoize
fn score(name: string) -> int {
    return len(name) * 10
}

fn classify(value: int) -> Outcome {
    if value > 30 {
        return Outcome::Ok("large")
    }
    return Outcome::Err("small")
}

fn render(result: Outcome) -> string {
    match result {
        Outcome::Ok(label) => "ok:" + label,
        Outcome::Err(reason) => "err:" + reason,
    }
}

fn worker() {
    fmt.Println("worker")
}

fn main() -> ! {
    job := Job{Name: "goplus", Attempts: 2}
    defer fmt.Println("done")
    go worker()

    name := normalize(job.Label())?
    value := score(name)
    fmt.Println(job)
    fmt.Println(render(classify(value)))

    i := 0
    for i < job.Attempts {
        fmt.Println("attempt", i)
        i += 1
    }
    return
}
