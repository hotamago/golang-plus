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

fn main() -> ! {
    name := readName()?
    fmt.Println(name)
    fmt.Println(statusText(Status::Running))
    return
}
