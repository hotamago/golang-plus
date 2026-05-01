package main

enum Status {
    Pending
    Done
}

fn label(s: Status) -> string {
    match s {
        Pending => "pending",
        Done => "done",
    }
}
