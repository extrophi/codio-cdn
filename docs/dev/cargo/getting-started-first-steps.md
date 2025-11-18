<!-- Source: https://doc.rust-lang.org/stable/cargo/getting-started/first-steps.html -->

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

# The Cargo Book

# [First Steps with Cargo](#first-steps-with-cargo)

This section provides a quick sense for the `cargo` command line tool. We
demonstrate its ability to generate a new [***package***](../appendix/glossary.html#package "\"package\" (glossary entry)") for us,
its ability to compile the [***crate***](../appendix/glossary.html#crate "\"crate\" (glossary entry)") within the package, and
its ability to run the resulting program.

To start a new package with Cargo, use `cargo new`:

```
$ cargo new hello_world
```

Cargo defaults to `--bin` to make a binary program. To make a library, we
would pass `--lib`, instead.

Letâs check out what Cargo has generated for us:

```
$ cd hello_world
$ tree .
.
âââ Cargo.toml
âââ src
    âââ main.rs

1 directory, 2 files
```

This is all we need to get started. First, letâs check out `Cargo.toml`:

```
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
```

This is called a [***manifest***](../appendix/glossary.html#manifest "\"manifest\" (glossary entry)"), and it contains all of the
metadata that Cargo needs to compile your package.

Hereâs whatâs in `src/main.rs`:

```
```
fn main() {
    println!("Hello, world!");
}
```
```

Cargo generated a âhello worldâ program for us, otherwise known as a
[***binary crate***](../appendix/glossary.html#crate "\"crate\" (glossary entry)"). Letâs compile it:

```
$ cargo build
   Compiling hello_world v0.1.0 (file:///path/to/package/hello_world)
```

And then run it:

```
$ ./target/debug/hello_world
Hello, world!
```

We can also use `cargo run` to compile and then run it, all in one step:

```
$ cargo run
     Fresh hello_world v0.1.0 (file:///path/to/package/hello_world)
   Running `target/hello_world`
Hello, world!
```

## [Going further](#going-further)

For more details on using Cargo, check out the [Cargo Guide](../guide/index.html)