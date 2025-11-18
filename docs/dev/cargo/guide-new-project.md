<!-- Source: https://doc.rust-lang.org/stable/cargo/guide/creating-a-new-project.html -->

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

# [Creating a New Package](#creating-a-new-package)

To start a new [package](../appendix/glossary.html#package "\"package\" (glossary entry)") with Cargo, use `cargo new`:

```
$ cargo new hello_world --bin
```

Weâre passing `--bin` because weâre making a binary program: if we
were making a library, weâd pass `--lib`. This also initializes a new `git`
repository by default. If you donât want it to do that, pass `--vcs none`.

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

Letâs take a closer look at `Cargo.toml`:

```
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
```

This is called a [***manifest***](../appendix/glossary.html#manifest "\"manifest\" (glossary entry)"), and it contains all of the
metadata that Cargo needs to compile your package. This file is written in the
[TOML](https://toml.io/) format (pronounced /tÉmÉl/).

Hereâs whatâs in `src/main.rs`:

```
```
fn main() {
    println!("Hello, world!");
}
```
```

Cargo generated a âhello worldâ program for you, otherwise known as a
[*binary crate*](../appendix/glossary.html#crate "\"crate\" (glossary entry)"). Letâs compile it:

```
$ cargo build
   Compiling hello_world v0.1.0 (file:///path/to/package/hello_world)
```

And then run it:

```
$ ./target/debug/hello_world
Hello, world!
```

You can also use `cargo run` to compile and then run it, all in one step (You
wonât see the `Compiling` line if you have not made any changes since you last
compiled):

```
$ cargo run
   Compiling hello_world v0.1.0 (file:///path/to/package/hello_world)
     Running `target/debug/hello_world`
Hello, world!
```

Youâll now notice a new file, `Cargo.lock`. It contains information about your
dependencies. Since there are none yet, itâs not very interesting.

Once youâre ready for release, you can use `cargo build --release` to compile
your files with optimizations turned on:

```
$ cargo build --release
   Compiling hello_world v0.1.0 (file:///path/to/package/hello_world)
```

`cargo build --release` puts the resulting binary in `target/release` instead of
`target/debug`.

Compiling in debug mode is the default for development. Compilation time is
shorter since the compiler doesnât do optimizations, but the code will run
slower. Release mode takes longer to compile, but the code will run faster.