package main

enum Result<T> {
    Ok(T)
    Err(string)
}

fn load() -> Result<int> {
    return Result<int>::Ok(7)
}
