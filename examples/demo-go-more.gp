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
func readName() (string, error) {
    return "goplus", nil
}

func statusText(s Status) string {
    switch s {
    case Status::Pending:
        return "pending"
    case Status::Running:
        return "running"
    case Status::Done:
        return "done"
    }
    return ""
}

func trace(next func(name string) (string, error), label string) func(name string) (string, error) {
    return func(name string) (string, error) {
        fmt.Println("trace:", label)
        return next(name)
    }
}

@trace("custom")
func greet(name string) (string, error) {
    return "hello " + name, nil
}

fn main() -> ! {
    name := readName()?
    fmt.Println(name)
    fmt.Println(statusText(Status::Running))

    msg := greet("goplus")?
    fmt.Println(msg)
    
    return
}
