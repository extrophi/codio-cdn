<!-- Source: https://doc.rust-lang.org/stable/cargo/guide/dependencies.html -->

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

# [Dependencies](#dependencies)

[crates.io](https://crates.io/) is the Rust communityâs central [*package registry*](../appendix/glossary.html#package-registry "\"package-registry\" (glossary entry)")
that serves as a location to discover and download
[packages](../appendix/glossary.html#package "\"package\" (glossary entry)"). `cargo` is configured to use it by default to find
requested packages.

To depend on a library hosted on [crates.io](https://crates.io/), add it to your `Cargo.toml`.

## [Adding a dependency](#adding-a-dependency)

If your `Cargo.toml` doesnât already have a `[dependencies]` section, add
that, then list the [crate](../appendix/glossary.html#crate "\"crate\" (glossary entry)") name and version that you would like to
use. This example adds a dependency on the `time` crate:

```
[dependencies]
time = "0.1.12"
```

The version string is a [SemVer](https://semver.org) version requirement. The [specifying
dependencies](../reference/specifying-dependencies.html) docs have more information about
the options you have here.

If you also wanted to add a dependency on the `regex` crate, you would not need
to add `[dependencies]` for each crate listed. Hereâs what your whole
`Cargo.toml` file would look like with dependencies on the `time` and `regex`
crates:

```
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
time = "0.1.12"
regex = "0.1.41"
```

Re-run `cargo build`, and Cargo will fetch the new dependencies and all of
their dependencies, compile them all, and update the `Cargo.lock`:

```
$ cargo build
      Updating crates.io index
   Downloading memchr v0.1.5
   Downloading libc v0.1.10
   Downloading regex-syntax v0.2.1
   Downloading memchr v0.1.5
   Downloading aho-corasick v0.3.0
   Downloading regex v0.1.41
     Compiling memchr v0.1.5
     Compiling libc v0.1.10
     Compiling regex-syntax v0.2.1
     Compiling memchr v0.1.5
     Compiling aho-corasick v0.3.0
     Compiling regex v0.1.41
     Compiling hello_world v0.1.0 (file:///path/to/package/hello_world)
```

`Cargo.lock` contains the exact information about which revision was used
for all of these dependencies.

Now, if `regex` gets updated, you will still build with the same revision until
you choose to run `cargo update`.

You can now use the `regex` library in `main.rs`.

```
use regex::Regex;

fn main() {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    println!("Did our date match? {}", re.is_match("2014-01-01"));
}
```

Running it will show:

```
$ cargo run
   Running `target/hello_world`
Did our date match? true
```