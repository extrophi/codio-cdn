<!-- Source: https://doc.rust-lang.org/stable/book/ch03-04-comments.html -->

## Keyboard shortcuts

Press `â` or `â` to navigate between chapters

Press `S` or `/` to search in the book

Press `?` to show this help

Press `Esc` to hide this help

[ ]

* Auto
* Light
* Rust
* Coal
* Navy
* Ayu

# The Rust Programming Language

## [Comments](#comments)

All programmers strive to make their code easy to understand, but sometimes
extra explanation is warranted. In these cases, programmers leave *comments* in
their source code that the compiler will ignore but people reading the source
code may find useful.

Hereâs a simple comment:

```
```
#![allow(unused)]
fn main() {
// hello, world
}
```
```

In Rust, the idiomatic comment style starts a comment with two slashes, and the
comment continues until the end of the line. For comments that extend beyond a
single line, youâll need to include `//` on each line, like this:

```
```
#![allow(unused)]
fn main() {
// So we're doing something complicated here, long enough that we need
// multiple lines of comments to do it! Whew! Hopefully, this comment will
// explain what's going on.
}
```
```

Comments can also be placed at the end of lines containing code:

Filename: src/main.rs

```
```
fn main() {
    let lucky_number = 7; // I'm feeling lucky today
}
```
```

But youâll more often see them used in this format, with the comment on a
separate line above the code itâs annotating:

Filename: src/main.rs

```
```
fn main() {
    // I'm feeling lucky today
    let lucky_number = 7;
}
```
```

Rust also has another kind of comment, documentation comments, which weâll
discuss in the [âPublishing a Crate to Crates.ioâ](ch14-02-publishing-to-crates-io.html)
section of Chapter 14.