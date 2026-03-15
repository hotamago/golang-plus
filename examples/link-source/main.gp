package main

import "fmt"
import "example.com/goplus-link-source/pkg/support"

fn main() -> ! {
    fmt.Println(buildBanner("goplus"))
    fmt.Println(callFromGoFile())
    fmt.Println(support.FromNestedPackages())
    return
}
