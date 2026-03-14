package main

import "fmt"

@derive(String)
enum Status {
    Pending
    Running
    Done
}

@log
@retry(3, 10)
fn readName() -> string! {
    return "goplus"
}

fn statusText(s: Status) -> string {
    match s {
        Status::Pending => "pending",
        Status::Running => "running",
        Status::Done => "done",
    }
}

fn trace(next: func(name string) (string, error), label: string) -> func(name string) (string, error) {
    return func(name string) (string, error) {
        fmt.Println("trace:", label)
        return next(name)
    }
}

@trace("custom")
fn greet(name: string) -> string! {
    return "hello " + name
}

fn main() -> ! {
    name := readName()?
    fmt.Println(name)
    fmt.Println(statusText(Status::Running))

    msg := greet("goplus")?
    fmt.Println(msg)
    
    return
}
