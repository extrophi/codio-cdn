<!-- Source: https://docs.rs/clap/latest/clap/ -->

[Docs.rs](/)

* clap-4.5.52

  + clap 4.5.52
  + [Permalink](/clap/4.5.52/clap/ "Get a link to this specific version")
  + [Docs.rs crate page](/crate/clap/latest "See clap in docs.rs")
  + [MIT](https://spdx.org/licenses/MIT) OR [Apache-2.0](https://spdx.org/licenses/Apache-2.0)

  + Links
  + [Repository](https://github.com/clap-rs/clap)
  + [crates.io](https://crates.io/crates/clap "See clap in crates.io")
  + [Source](/crate/clap/latest/source/ "Browse source of clap-4.5.52")

  + Owners
  + [kbknapp](https://crates.io/users/kbknapp)
  + [github:clap-rs:admins](https://crates.io/teams/github%3Aclap-rs%3Aadmins)
  + [github:rust-cli:maintainers](https://crates.io/teams/github%3Arust-cli%3Amaintainers)

  + Dependencies
  + - [clap\_builder =4.5.52
      *normal*](/clap_builder/%3D4.5.52/)
    - [clap\_derive =4.5.49
      *normal*
      *optional*](/clap_derive/%3D4.5.49/)
    - [automod ^1.0.14
      *dev*](/automod/%5E1.0.14/)
    - [clap-cargo ^0.15.0
      *dev*](/clap-cargo/%5E0.15.0/)
    - [jiff ^0.2.3
      *dev*](/jiff/%5E0.2.3/)
    - [rustversion ^1.0.15
      *dev*](/rustversion/%5E1.0.15/)
    - [semver ^1.0.26
      *dev*](/semver/%5E1.0.26/)
    - [shlex ^1.3.0
      *dev*](/shlex/%5E1.3.0/)
    - [snapbox ^0.6.16
      *dev*](/snapbox/%5E0.6.16/)
    - [trybuild ^1.0.91
      *dev*](/trybuild/%5E1.0.91/)
    - [trycmd ^0.15.3
      *dev*](/trycmd/%5E0.15.3/)

  + Versions

  + [**48.61%**
    of the crate is documented](/crate/clap/latest)
* Platform
  + [aarch64-apple-darwin](/crate/clap/latest/target-redirect/aarch64-apple-darwin/clap/)
  + [aarch64-unknown-linux-gnu](/crate/clap/latest/target-redirect/aarch64-unknown-linux-gnu/clap/)
  + [i686-pc-windows-msvc](/crate/clap/latest/target-redirect/i686-pc-windows-msvc/clap/)
  + [x86\_64-pc-windows-msvc](/crate/clap/latest/target-redirect/x86_64-pc-windows-msvc/clap/)
  + [x86\_64-unknown-linux-gnu](/crate/clap/latest/target-redirect/clap/)
* [Feature flags](/crate/clap/latest/features "Browse available feature flags of clap-4.5.52")

* docs.rs
  + [About docs.rs](/about)
  + [Badges](/about/badges)
  + [Builds](/about/builds)
  + [Metadata](/about/metadata)
  + [Shorthand URLs](/about/redirections)
  + [Download](/about/download)
  + [Rustdoc JSON](/about/rustdoc-json)
  + [Build queue](/releases/queue)
  + [Privacy policy](https://foundation.rust-lang.org/policies/privacy-policy/#docs.rs)

* Rust
  + [Rust website](https://www.rust-lang.org/)
  + [The Book](https://doc.rust-lang.org/book/)
  + [Standard Library API Reference](https://doc.rust-lang.org/std/)
  + [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
  + [The Cargo Guide](https://doc.rust-lang.org/cargo/guide/)
  + [Clippy Documentation](https://doc.rust-lang.org/nightly/clippy)

## Crate clap

[![logo](https://raw.githubusercontent.com/clap-rs/clap/master/assets/clap.png)](../clap/index.html)

## [clap](../clap/index.html)4.5.52

* [All Items](all.html)

### Sections

* [Aspirations](#aspirations "Aspirations")
* [Example](#example "Example")
  + [Related Projects](#related-projects "Related Projects")

### [Crate Items](#modules)

* [Modules](#modules "Modules")
* [Macros](#macros "Macros")
* [Structs](#structs "Structs")
* [Enums](#enums "Enums")
* [Traits](#traits "Traits")
* [Type Aliases](#types "Type Aliases")

# Crate clap Copy item path

[Source](../src/clap/lib.rs.html#6-110)

Expand description

> **Command Line Argument Parser for Rust**

Quick Links:

* Derive [tutorial](_derive/_tutorial/index.html "mod clap::_derive::_tutorial") and [reference](_derive/index.html "mod clap::_derive")
* Builder [tutorial](_tutorial/index.html "mod clap::_tutorial") and [reference](struct.Command.html "struct clap::Command")
* [Cookbook](_cookbook/index.html "mod clap::_cookbook")
* [CLI Concepts](_concepts/index.html "mod clap::_concepts")
* [FAQ](_faq/index.html "mod clap::_faq")
* [Discussions](https://github.com/clap-rs/clap/discussions)
* [CHANGELOG](https://github.com/clap-rs/clap/blob/v4.5.52/CHANGELOG.md) (includes major version migration
  guides)

### [§](#aspirations)Aspirations

* Out of the box, users get a polished CLI experience
  + Including common argument behavior, help generation, suggested fixes for users, colored output, [shell completions](https://github.com/clap-rs/clap/tree/master/clap_complete), etc
* Flexible enough to port your existing CLI interface
  + However, we won’t necessarily streamline support for each use case
* Reasonable parse performance
* Resilient maintainership, including
  + Willing to break compatibility rather than batching up breaking changes in large releases
  + Leverage feature flags to keep to one active branch
  + Being under [WG-CLI](https://github.com/rust-cli/team/) to increase the bus factor
* We follow semver and will wait about 6-9 months between major breaking changes
* We will support the last two minor Rust releases (MSRV, currently 1.74)

While these aspirations can be at odds with fast build times and low binary
size, we will still strive to keep these reasonable for the flexibility you
get. Check out the
[argparse-benchmarks](https://github.com/rust-cli/argparse-benchmarks-rs) for
CLI parsers optimized for other use cases.

### [§](#example)Example

Run

```
$ cargo add clap --features derive
```

*(See also [feature flag reference](_features/index.html "mod clap::_features"))*

Then define your CLI in `main.rs`:

```
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

fn main() {
    let args = Args::parse();

    for _ in 0..args.count {
        println!("Hello {}!", args.name);
    }
}
```

And try it out:

```
$ demo --help
A simple to use, efficient, and full-featured Command Line Argument Parser

Usage: demo[EXE] [OPTIONS] --name <NAME>

Options:
  -n, --name <NAME>    Name of the person to greet
  -c, --count <COUNT>  Number of times to greet [default: 1]
  -h, --help           Print help
  -V, --version        Print version

$ demo --name Me
Hello Me!
```

*(version number and `.exe` extension on windows replaced by placeholders)*

See also the derive [tutorial](_derive/_tutorial/index.html "mod clap::_derive::_tutorial") and [reference](_derive/index.html "mod clap::_derive")

#### [§](#related-projects)Related Projects

Augment clap:

* [wild](https://crates.io/crates/wild) for supporting wildcards (`*`) on Windows like you do Linux
* [argfile](https://crates.io/crates/argfile) for loading additional arguments from a file (aka response files)
* [shadow-rs](https://crates.io/crates/shadow-rs) for generating `Command::long_version`
* [clap\_mangen](https://crates.io/crates/clap_mangen) for generating man page source (roff)
* [clap\_complete](https://crates.io/crates/clap_complete) for shell completion support
* [clap-i18n-richformatter](https://crates.io/crates/clap-i18n-richformatter) for i18n support with `clap::error::RichFormatter`

CLI Helpers

* [clio](https://crates.io/crates/clio) for reading/writing to files specified as arguments
* [clap-verbosity-flag](https://crates.io/crates/clap-verbosity-flag)
* [clap-cargo](https://crates.io/crates/clap-cargo)
* [colorchoice-clap](https://crates.io/crates/colorchoice-clap)

Testing

* [`trycmd`](https://crates.io/crates/trycmd): Bulk snapshot testing
* [`snapbox`](https://crates.io/crates/snapbox): Specialized snapshot testing
* [`assert_cmd`](https://crates.io/crates/assert_cmd) and [`assert_fs`](https://crates.io/crates/assert_fs): Customized testing

Documentation:

* [Command-line Apps for Rust](https://rust-cli.github.io/book/index.html) book

## Modules[§](#modules)

[\_concepts](_concepts/index.html "mod clap::_concepts")`unstable-doc`
:   CLI Concepts

[\_cookbook](_cookbook/index.html "mod clap::_cookbook")`unstable-doc`
:   Documentation: Cookbook

[\_derive](_derive/index.html "mod clap::_derive")`unstable-doc`
:   Documentation: Derive Reference

[\_faq](_faq/index.html "mod clap::_faq")`unstable-doc`
:   Documentation: FAQ

[\_features](_features/index.html "mod clap::_features")`unstable-doc`
:   Documentation: Feature Flags

[\_tutorial](_tutorial/index.html "mod clap::_tutorial")`unstable-doc`
:   Tutorial for the Builder API

[builder](builder/index.html "mod clap::builder")
:   Define [`Command`](struct.Command.html "struct clap::Command") line [arguments](struct.Arg.html "struct clap::Arg")

[error](error/index.html "mod clap::error")
:   Error reporting

[parser](parser/index.html "mod clap::parser")
:   [`Command`](struct.Command.html "struct clap::Command") line argument parser

## Macros[§](#macros)

[arg](macro.arg.html "macro clap::arg")
:   Create an [`Arg`](struct.Arg.html "struct clap::Arg") from a usage string.

[command](macro.command.html "macro clap::command")`cargo`
:   Allows you to build the `Command` instance from your Cargo.toml at compile time.

[crate\_authors](macro.crate_authors.html "macro clap::crate_authors")`cargo`
:   Allows you to pull the authors for the command from your Cargo.toml at
    compile time in the form:
    `"author1 lastname <author1@example.com>:author2 lastname <author2@example.com>"`

[crate\_description](macro.crate_description.html "macro clap::crate_description")`cargo`
:   Allows you to pull the description from your Cargo.toml at compile time.

[crate\_name](macro.crate_name.html "macro clap::crate_name")`cargo`
:   Allows you to pull the name from your Cargo.toml at compile time.

[crate\_version](macro.crate_version.html "macro clap::crate_version")`cargo`
:   Allows you to pull the version from your Cargo.toml at compile time as
    `MAJOR.MINOR.PATCH_PKGVERSION_PRE`

[value\_parser](macro.value_parser.html "macro clap::value_parser")
:   Select a [`ValueParser`](builder/struct.ValueParser.html "struct clap::builder::ValueParser") implementation from the intended type

## Structs[§](#structs)

[Arg](struct.Arg.html "struct clap::Arg")
:   The abstract representation of a command line argument. Used to set all the options and
    relationships that define a valid argument for the program.

[ArgGroup](struct.ArgGroup.html "struct clap::ArgGroup")
:   Specifies a logical group of [arguments](struct.Arg.html "struct clap::Arg")

[ArgMatches](struct.ArgMatches.html "struct clap::ArgMatches")
:   Container for parse results.

[Command](struct.Command.html "struct clap::Command")
:   Build a command-line interface.

[Id](struct.Id.html "struct clap::Id")
:   [`Arg`](struct.Arg.html "struct clap::Arg") or [`ArgGroup`](struct.ArgGroup.html "struct clap::ArgGroup") identifier

## Enums[§](#enums)

[ArgAction](enum.ArgAction.html "enum clap::ArgAction")
:   Behavior of arguments when they are encountered while parsing

[ColorChoice](enum.ColorChoice.html "enum clap::ColorChoice")
:   Represents the color preferences for program output

[ValueHint](enum.ValueHint.html "enum clap::ValueHint")
:   Provide shell with hint on how to complete an argument.

## Traits[§](#traits)

[Args](trait.Args.html "trait clap::Args")
:   Parse a set of arguments into a user-defined container.

[CommandFactory](trait.CommandFactory.html "trait clap::CommandFactory")
:   Create a [`Command`](struct.Command.html "struct clap::Command") relevant for a user-defined container.

[FromArgMatches](trait.FromArgMatches.html "trait clap::FromArgMatches")
:   Converts an instance of [`ArgMatches`](struct.ArgMatches.html "struct clap::ArgMatches") to a user-defined container.

[Parser](trait.Parser.html "trait clap::Parser")
:   Parse command-line arguments into `Self`.

[Subcommand](trait.Subcommand.html "trait clap::Subcommand")
:   Parse a sub-command into a user-defined enum.

[ValueEnum](trait.ValueEnum.html "trait clap::ValueEnum")
:   Parse arguments into enums.

## Type Aliases[§](#types)

[Error](type.Error.html "type clap::Error")
:   Command Line Argument Parser Error