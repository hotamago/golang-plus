package main

import "fmt"

fn trace(next: (fn(name: string) -> string!), label: string) -> (fn(name: string) -> string!) {
    return fn(name: string) -> string! {
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
