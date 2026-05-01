package main

import "fmt"
import "time"

fn printLater() {
    time.Sleep(1 * time.Millisecond)
    fmt.Println("worker")
}

fn main() {
    go printLater()

    select {
    case <-time.After(10 * time.Millisecond):
        fmt.Println("timer fired")
    }

    switch len("go") {
    case 2:
        fmt.Println("two letters")
    default:
        fmt.Println("other length")
    }
}
