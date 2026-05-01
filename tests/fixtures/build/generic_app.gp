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
