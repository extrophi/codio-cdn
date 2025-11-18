<!-- Source: https://tokio.rs/tokio/tutorial/setup -->

[We’re now accepting talk proposals for TokioConf 2026! Learn more →](/blog/2025-09-26-announcing-tokio-conf-cfp)

[![tokio-logo](/img/tokio-horizontal.svg)](/)

[Learn](/tokio/tutorial)[API Docs](https://docs.rs/tokio)[Blog](/blog/2025-09-26-announcing-tokio-conf-cfp)

---

TABLE OF CONTENTS

Tokio

* [Tutorial](/tokio/tutorial)

  - [Overview](/tokio/tutorial)
  - [Setup](/tokio/tutorial/setup)
  - [Hello Tokio](/tokio/tutorial/hello-tokio)
  - [Spawning](/tokio/tutorial/spawning)
  - [Shared state](/tokio/tutorial/shared-state)
  - [Channels](/tokio/tutorial/channels)
  - [I/O](/tokio/tutorial/io)
  - [Framing](/tokio/tutorial/framing)
  - [Async in depth](/tokio/tutorial/async)
  - [Select](/tokio/tutorial/select)
  - [Streams](/tokio/tutorial/streams)
* [Topics](/tokio/topics)

  - [Bridging with sync code](/tokio/topics/bridging)
  - [Graceful Shutdown](/tokio/topics/shutdown)
  - [Getting started with Tracing](/tokio/topics/tracing)
  - [Next steps with Tracing](/tokio/topics/tracing-next-steps)
  - [Unit Testing](/tokio/topics/testing)
* [Glossary](/tokio/glossary)
* [API documentation](https://docs.rs/tokio)

# Setup

This tutorial will take you step by step through the process of building a
[Redis](https://redis.io) client and server. We will start with the basics of asynchronous
programming with Rust and build up from there. We will implement a subset of
Redis commands but will get a comprehensive tour of Tokio.

# Mini-Redis

The project that you will build in this tutorial is available as [Mini-Redis on
GitHub](https://github.com/tokio-rs/mini-redis). Mini-Redis is designed with the primary goal of learning
Tokio, and is therefore very well commented, but this also means that Mini-Redis
is missing some features you would want in a real Redis library. You can find
production-ready Redis libraries on [crates.io](https://crates.io/).

We will use Mini-Redis directly in the tutorial. This allows us to use parts of
Mini-Redis in the tutorial before we implement them later in the tutorial.

# Getting Help

At any point, if you get stuck, you can always get help on [Discord](https://discord.gg/tokio) or [GitHub
discussions](https://github.com/tokio-rs/tokio/discussions). Don't worry about asking "beginner" questions. We all start
somewhere and are happy to help.

# Prerequisites

Readers should already be familiar with [Rust](https://rust-lang.org). The [Rust book](https://doc.rust-lang.org/book/) is an
excellent resource to get started with.

While not required, some experience with writing networking code using the [Rust
standard library](https://doc.rust-lang.org/std/) or another language can be helpful.

No pre-existing knowledge of Redis is required.

## Rust

Before getting started, you should make sure that you have the
[Rust](https://www.rust-lang.org/tools/install) toolchain installed and ready to go. If you don't have it,
the easiest way to install it is using [rustup](https://rustup.rs/).

This tutorial requires a minimum of Rust version `1.45.0`, but the most
recent stable version of Rust is recommended.

To check that Rust is installed on your computer, run the following:

```
$ rustc --version
```

You should see output like `rustc 1.46.0 (04488afe3 2020-08-24)`.

## Mini-Redis server

Next, install the Mini-Redis server. This will be used to test our client as we
build it.

```
$ cargo install mini-redis
```

Make sure that it was successfully installed by starting the server:

```
$ mini-redis-server
```

Then, in a separate terminal window, try to get the key `foo` using `mini-redis-cli`

```
$ mini-redis-cli get foo
```

You should see `(nil)`.

# Ready to go

That's it, everything is ready to go. Go to the next page to write your first
asynchronous Rust application.

[![](/img/arrow-left.svg)Tutorial](/tokio/tutorial)

[Hello Tokio![](/img/arrow-right.svg)](/tokio/tutorial/hello-tokio)

Get Help:

[Edit this page](https://github.com/tokio-rs/website/edit/master/content/tokio/tutorial/setup.md)