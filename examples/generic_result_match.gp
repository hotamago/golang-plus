package main

import "fmt"

const defaultName = "goplus"
type Label = string

enum Result<T> {
    Ok(T)
    Err(string)
}

fn loadName(ok: bool) -> Result<string> {
    if ok {
        return Result<string>::Ok(defaultName)
    }
    return Result<string>::Err("missing name")
}

fn render(result: Result<string>) -> string {
    match result {
        Ok(value) => "loaded:" + Label(value),
        Err(reason) => "error:" + reason,
    }
}

fn main() -> ! {
    fmt.Println(render(loadName(true)))
    fmt.Println(render(loadName(false)))
    return
}
