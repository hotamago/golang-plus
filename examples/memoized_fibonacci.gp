package main

import "fmt"

@memoize
fn fib(n: int) -> int {
    if n == 0 {
        return 0
    }
    if n == 1 {
        return 1
    }
    return fib(n-1) + fib(n-2)
}

fn main() {
    i := 0
    for i <= 10 {
        fmt.Println(i, fib(i))
        i += 1
    }
}
