<!-- Source: https://doc.rust-lang.org/stable/cargo/guide/why-cargo-exists.html -->

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

# [Why Cargo Exists](#why-cargo-exists)

## [Preliminaries](#preliminaries)

In Rust, as you may know, a library or executable program is called a
[*crate*](../appendix/glossary.html#crate "\"crate\" (glossary entry)"). Crates are compiled using the Rust compiler,
`rustc`. When starting with Rust, the first source code most people encounter
is that of the classic âhello worldâ program, which they compile by invoking
`rustc` directly:

```
$ rustc hello.rs
$ ./hello
Hello, world!
```

Note that the above command required that you specify the file name
explicitly. If you were to directly use `rustc` to compile a different program,
a different command line invocation would be required. If you needed to specify
any specific compiler flags or include external dependencies, then the
needed command would be even more specific (and complex).

Furthermore, most non-trivial programs will likely have dependencies on
external libraries, and will therefore also depend transitively on *their*
dependencies. Obtaining the correct versions of all the necessary dependencies
and keeping them up to date would be hard and error-prone if done by
hand.

Rather than work only with crates and `rustc`, you can avoid the difficulties
involved with performing the above tasks by introducing a higher-level
[â*package*â](../appendix/glossary.html#package "\"package\" (glossary entry)") abstraction and by using a
[*package manager*](../appendix/glossary.html#package-manager "\"package manager\" (glossary entry)").

## [Enter: Cargo](#enter-cargo)

*Cargo* is the Rust package manager. It is a tool that allows Rust
[*packages*](../appendix/glossary.html#package "\"package\" (glossary entry)") to declare their various dependencies and ensure
that youâll always get a repeatable build.

To accomplish this goal, Cargo does four things:

* Introduces two metadata files with various bits of package information.
* Fetches and builds your packageâs dependencies.
* Invokes `rustc` or another build tool with the correct parameters to build
  your package.
* Introduces conventions to make working with Rust packages easier.

To a large extent, Cargo normalizes the commands needed to build a given
program or library; this is one aspect to the above mentioned conventions. As
we show later, the same command can be used to build different
[*artifacts*](../appendix/glossary.html#artifact "\"artifact\" (glossary entry)"), regardless of their names. Rather than invoke
`rustc` directly, you can instead invoke something generic such as `cargo build` and let cargo worry about constructing the correct `rustc`
invocation. Furthermore, Cargo will automatically fetch any dependencies
you have defined for your artifact from a [*registry*](../appendix/glossary.html#registry "\"registry\" (glossary entry)"),
and arrange for them to be added into your build as needed.

It is only a slight exaggeration to say that once you know how to build one
Cargo-based project, you know how to build *all* of them.