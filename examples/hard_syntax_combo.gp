package main

import (
    "errors"
    "fmt"
    "sort"
    strconv "strconv"
    strs "strings"
    "time"
)

const (
    DefaultName = "goplus"
    BitMask = 1 << 3
)

var globalCounter = 0

type Labeler interface {
    Label() string
}

type rawPair struct {
    Left  string
    Right int
}

@derive(String)
enum JobState {
    Queued
    Running
    Done
    Failed
}

enum Result<T> {
    Ok(T)
    Err(string)
}

@derive(String, Debug, JSON)
struct Account {
    ID: int `json:"id" gorm:"primaryKey;column:id"`
    Name: string `json:"name" validate:"required"`
    Email: string `json:"email" gorm:"column:email"`
    Tags: []string `json:"tags,omitempty"`
    Meta: map[string]string `json:"meta,omitempty"`
}

impl Account {
    fn Label(self) -> string {
        return fmt.Sprintf("%s<%s>#%d", self.Name, self.Email, self.ID)
    }
}

fn rawPattern() -> string {
    return `[A-Z]+@[a-z]+\.(dev|test)`
}

fn parseID(input: string) -> int! {
    return strconv.Atoi(input)?
}

fn validateAccount(account: Account) -> ! {
    if !strs.Contains(account.Email, "@") {
        return error("email must contain @")
    }
    if len(account.Tags) == 0 {
        return error("account needs at least one tag")
    }
    return
}

fn trace(next: func(id int) (Account, error), label: string) -> func(id int) (Account, error) {
    return func(id int) (Account, error) {
        fmt.Println("trace:", label, id)
        return next(id)
    }
}

@trace("load-account")
@retry(2, 1)
fn loadAccount(id: int) -> Account! {
    switch {
    case id < 0:
        return Account{}, errors.New("negative id")
    }
    tags := []string{"api", "gp", DefaultName}
    sort.Strings(tags)
    meta := map[string]string{
        "source": "hard-example",
        "mask": fmt.Sprint(BitMask),
    }
    return Account{
        ID: id,
        Name: DefaultName,
        Email: "team@goplus.dev",
        Tags: tags,
        Meta: meta,
    }
}

@memoize
fn fib(n: int) -> int {
    switch {
    case n < 2:
        return n
    }
    return fib(n-1) + fib(n-2)
}

fn classifyScore(score: int) -> JobState {
    switch {
    case score < 0:
        return JobState::Failed
    case score == 0:
        return JobState::Queued
    case score < 10:
        return JobState::Running
    default:
        return JobState::Done
    }
}

fn scoreValues(values: []int) -> int {
    fixed := [3]int{1, 2, 3}
    weights := map[string]int{"even": fixed[0], "odd": fixed[2]}
    sum := 0

    for idx, value := range values {
        switch {
        case idx%2 == 0:
            sum += value * weights["even"]
        default:
            sum += value * weights["odd"]
        }
    }

    return sum
}

fn buildBuckets(values: []int) -> map[string][]int {
    buckets := map[string][]int{
        "even": []int{},
        "odd": []int{},
    }

    for _, value := range values {
        if value%2 == 0 {
            buckets["even"] = append(buckets["even"], value)
        } else {
            buckets["odd"] = append(buckets["odd"], value)
        }
    }

    return buckets
}

fn worker(ch: chan int, values: []int) {
    defer fmt.Println("worker done")
    select {
    case ch <- scoreValues(values):
    }
}

fn waitScore(values: []int) -> int! {
    ch := make(chan int, 1)
    go worker(ch, values)

    select {
    case score := <-ch:
        return score, nil
    case <-time.After(50 * time.Millisecond):
        return 0, errors.New("score timeout")
    }
}

fn normalize(input: string) -> string! {
    clean := func(value string) (string, error) {
        trimmed := strs.TrimSpace(value)
        if trimmed == "" {
            return "", errors.New("empty input")
        }
        return strs.ToUpper(trimmed), nil
    }
    return clean(input)?
}

fn summarize(result: Result<Account>) -> string {
    match result {
        Ok(account) => account.Label(),
        Err(message) => message,
    }
}

fn bump(account: *Account) {
    account.ID += 1
    globalCounter += 1
}

fn main() -> ! {
    id := parseID("42")?
    account := loadAccount(id)?
    validateAccount(account)?
    bump(&account)

    values := []int{1, 2, 3, 4, 5}
    score := waitScore(values)?
    buckets := buildBuckets(values)
    upper := normalize(" mixed goplus/go syntax ")?
    state := classifyScore(score)
    labeler := Labeler(account)
    pair := rawPair{Left: labeler.Label(), Right: score}
    summary := summarize(Result<Account>::Ok(account))

    fmt.Println(rawPattern())
    fmt.Println(labeler.Label(), pair, summary)
    fmt.Println("state:", state, "fib:", fib(8), "upper:", upper)
    fmt.Println("buckets:", buckets["even"], buckets["odd"], "counter:", globalCounter)
    return
}
