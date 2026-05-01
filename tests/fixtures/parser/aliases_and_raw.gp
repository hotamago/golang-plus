package main

import (
    f "fmt"
    _ "embed"
    . "strings"
)

const answer = 42
var counter int
type Label = string

fn main() {
    defer f.Println("done")
    go f.Println(TrimSpace(" work "))
    counter = answer
}
