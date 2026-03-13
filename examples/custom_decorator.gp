package main

import "fmt"

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
    msg := greet("goplus")?
    fmt.Println(msg)
    return
}
