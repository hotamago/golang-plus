package main

import "fmt"
import "strings"

@derive(String)
struct User {
    Name: string
    Email: string
}

impl User {
    fn Display(self) -> string {
        return self.Name + " <" + self.Email + ">"
    }
}

fn requireText(value: string, field: string) -> string! {
    trimmed := strings.TrimSpace(value)
    if len(trimmed) == 0 {
        return error(field + " is required")
    }
    return trimmed
}

fn newUser(name: string, email: string) -> User! {
    cleanName := requireText(name, "name")?
    cleanEmail := requireText(email, "email")?
    return User{Name: cleanName, Email: cleanEmail}
}

fn main() -> ! {
    user := newUser("Ada", "ada@example.com")?
    fmt.Println(user.Display())
    fmt.Println(user)
    return
}
