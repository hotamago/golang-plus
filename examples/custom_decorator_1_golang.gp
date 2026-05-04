package main

import "fmt"

func trace(next func(name string) (string, error), label string) (func(name string) (string, error)) {
    return func(name string) (string, error) {
        fmt.Println("trace:", label)
        return next(name)
    }
}

@trace("custom")
func greet(name string) (string, error) {
    return "hello " + name, nil
}

func main() error {
    msg := greet("goplus")?
    fmt.Println(msg)
    return
}
