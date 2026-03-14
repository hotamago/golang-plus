package main

import "fmt"

fn test() -> string! {
    return "test function"
}

fn trace(next: (fn(name: string) -> string!), label: string) -> (fn(name: string) -> string!) {
    fmt.Println("Begin decorator")
    temp_fun := fn(name: string) -> string! {
        fmt.Println("trace:", label)
        return next(name)
    }
    fmt.Println("Custom begin call")
    result_test, err := test()
    if err != nil {
        fmt.Println("error")
    }
    fmt.Println(result_test)
    fmt.Println("Custom end call")

    fmt.Println("End decorator")

    return temp_fun
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
