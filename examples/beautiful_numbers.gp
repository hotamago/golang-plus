package main

import "bufio"
import "fmt"
import "os"
import "strconv"

fn nextToken(scanner: *bufio.Scanner) -> string! {
    if !scanner.Scan() {
        return error("unexpected EOF")
    }
    return scanner.Text()
}

fn nextTokenNoErr(scanner: *bufio.Scanner) -> string {
    if !scanner.Scan() {
        panic("unexpected EOF")
    }
    return scanner.Text()
}

fn atoiStrict(s: string) -> int! {
    return strconv.Atoi(s)?
}

fn passthrough(next: func(string) int, tag: string) -> func(string) int {
    return next
}

@passthrough("beautiful-numbers")
@memoize
fn minMovesToBeautiful(x: string) -> int {
    n := len(x)
    negInf := -1_000_000

    dp := make([][]int, n+1)

    i := 0
    for i <= n {
        dp[i] = make([]int, 10)
        sInit := 0
        for sInit <= 9 {
            dp[i][sInit] = negInf
            sInit += 1
        }
        i += 1
    }
    dp[0][0] = 0

    pos := 0
    for pos < n {
        curDigit := int(x[pos] - 48)
        start := 0
        if pos == 0 {
            start = 1
        }

        s := 0
        for s <= 9 {
            cur := dp[pos][s]
            if cur < 0 {
                s += 1
                continue
            }

            d := start
            for d <= 9 {
                ns := s + d
                if ns > 9 {
                    d += 1
                    continue
                }
                keep := cur
                if d == curDigit {
                    keep += 1
                }
                if keep > dp[pos+1][ns] {
                    dp[pos+1][ns] = keep
                }
                d += 1
            }
            s += 1
        }
        pos += 1
    }

    bestKeep := 0
    sum := 0
    for sum <= 9 {
        if dp[n][sum] > bestKeep {
            bestKeep = dp[n][sum]
        }
        sum += 1
    }

    return n - bestKeep
}

fn main() -> ! {
    scanner := bufio.NewScanner(os.Stdin)
    scanner.Split(bufio.ScanWords)

    out := bufio.NewWriter(os.Stdout)
    defer out.Flush()

    tText := nextToken(scanner)?
    t := atoiStrict(tText)?

    tc := 0
    for tc < t {
        x := nextTokenNoErr(scanner)
        ans := minMovesToBeautiful(x)
        fmt.Fprintln(out, ans)
        tc += 1
    }
    return
}
