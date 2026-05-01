package main

@retry(0)
fn load() -> string {
    return "ok"
}
