<!-- Source: https://tokio.rs/tokio/tutorial/streams -->

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

# Streams

A stream is an asynchronous series of values. It is the asynchronous equivalent
to Rust's [`std::iter::Iterator`](https://doc.rust-lang.org/book/ch13-02-iterators.html) and is represented by the [`Stream`](https://docs.rs/futures-core/0.3/futures_core/stream/trait.Stream.html)
trait. Streams can be iterated in `async` functions. They can also be
transformed using adapters. Tokio provides a number of common adapters on the
[`StreamExt`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html) trait.

Tokio provides stream support in a separate crate: `tokio-stream`.

```
tokio-stream = "0.1"
```

> Currently, Tokio's Stream utilities exist in the `tokio-stream` crate.
> Once the `Stream` trait is stabilized in the Rust standard library, Tokio's
> stream utilities will be moved into the `tokio` crate.

# Iteration

Currently, the Rust programming language does not support async `for` loops.
Instead, iterating streams is done using a `while let` loop paired with
[`StreamExt::next()`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.next).

```
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let mut stream = tokio_stream::iter(&[1, 2, 3]);

    while let Some(v) = stream.next().await {
        println!("GOT = {:?}", v);
    }
}
```

Like iterators, the `next()` method returns `Option<T>` where `T` is the
stream's value type. Receiving `None` indicates that stream iteration is
terminated.

## Mini-Redis broadcast

Let's go over a slightly more complicated example using the Mini-Redis client.

Full code can be found [here](https://github.com/tokio-rs/website/blob/master/tutorial-code/streams/src/main.rs).

```
use tokio_stream::StreamExt;
use mini_redis::client;

async fn publish() -> mini_redis::Result<()> {
    let mut client = client::connect("127.0.0.1:6379").await?;

    // Publish some data
    client.publish("numbers", "1".into()).await?;
    client.publish("numbers", "two".into()).await?;
    client.publish("numbers", "3".into()).await?;
    client.publish("numbers", "four".into()).await?;
    client.publish("numbers", "five".into()).await?;
    client.publish("numbers", "6".into()).await?;
    Ok(())
}

async fn subscribe() -> mini_redis::Result<()> {
    let client = client::connect("127.0.0.1:6379").await?;
    let subscriber = client.subscribe(vec!["numbers".to_string()]).await?;
    let messages = subscriber.into_stream();

    tokio::pin!(messages);

    while let Some(msg) = messages.next().await {
        println!("got = {:?}", msg);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> mini_redis::Result<()> {
    tokio::spawn(async {
        publish().await
    });

    subscribe().await?;

    println!("DONE");

    Ok(())
}
```

A task is spawned to publish messages to the Mini-Redis server on the "numbers"
channel. Then, on the main task, we subscribe to the "numbers" channel and
display received messages.

After subscribing, [`into_stream()`](https://docs.rs/mini-redis/0.4/mini_redis/client/struct.Subscriber.html#method.into_stream) is called on the returned subscriber. This
consumes the `Subscriber`, returning a stream that yields messages as they
arrive. Before we start iterating the messages, note that the stream is
[pinned](https://doc.rust-lang.org/std/pin/index.html) to the stack using [`tokio::pin!`](https://docs.rs/tokio/1/tokio/macro.pin.html). Calling `next()` on a stream
requires the stream to be [pinned](https://doc.rust-lang.org/std/pin/index.html). The `into_stream()` function returns a
stream that is *not* pinned, we must explicitly pin it in order to iterate it.

> A Rust value is "pinned" when it can no longer be moved in memory. A key
> property of a pinned value is that pointers can be taken to the pinned
> data and the caller can be confident the pointer stays valid. This feature
> is used by `async/await` to support borrowing data across `.await` points.

If we forget to pin the stream, we get an error like this:

```
error[E0277]: `from_generator::GenFuture<[static generator@Subscriber::into_stream::{closure#0} for<'r, 's, 't0, 't1, 't2, 't3, 't4, 't5, 't6> {ResumeTy, &'r mut Subscriber, Subscriber, impl Future, (), std::result::Result<Option<Message>, Box<(dyn std::error::Error + Send + Sync + 't0)>>, Box<(dyn std::error::Error + Send + Sync + 't1)>, &'t2 mut async_stream::yielder::Sender<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't3)>>>, async_stream::yielder::Sender<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't4)>>>, std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't5)>>, impl Future, Option<Message>, Message}]>` cannot be unpinned
  --> streams/src/main.rs:29:36
   |
29 |     while let Some(msg) = messages.next().await {
   |                                    ^^^^ within `tokio_stream::filter::_::__Origin<'_, impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>`, the trait `Unpin` is not implemented for `from_generator::GenFuture<[static generator@Subscriber::into_stream::{closure#0} for<'r, 's, 't0, 't1, 't2, 't3, 't4, 't5, 't6> {ResumeTy, &'r mut Subscriber, Subscriber, impl Future, (), std::result::Result<Option<Message>, Box<(dyn std::error::Error + Send + Sync + 't0)>>, Box<(dyn std::error::Error + Send + Sync + 't1)>, &'t2 mut async_stream::yielder::Sender<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't3)>>>, async_stream::yielder::Sender<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't4)>>>, std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 't5)>>, impl Future, Option<Message>, Message}]>`
   |
   = note: required because it appears within the type `impl Future`
   = note: required because it appears within the type `async_stream::async_stream::AsyncStream<std::result::Result<Message, Box<(dyn std::error::Error + Send + Sync + 'static)>>, impl Future>`
   = note: required because it appears within the type `impl Stream`
   = note: required because it appears within the type `tokio_stream::filter::_::__Origin<'_, impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>`
   = note: required because of the requirements on the impl of `Unpin` for `tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>`
   = note: required because it appears within the type `tokio_stream::map::_::__Origin<'_, tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>, [closure@streams/src/main.rs:26:14: 26:40]>`
   = note: required because of the requirements on the impl of `Unpin` for `tokio_stream::map::Map<tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>, [closure@streams/src/main.rs:26:14: 26:40]>`
   = note: required because it appears within the type `tokio_stream::take::_::__Origin<'_, tokio_stream::map::Map<tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>, [closure@streams/src/main.rs:26:14: 26:40]>>`
   = note: required because of the requirements on the impl of `Unpin` for `tokio_stream::take::Take<tokio_stream::map::Map<tokio_stream::filter::Filter<impl Stream, [closure@streams/src/main.rs:22:17: 25:10]>, [closure@streams/src/main.rs:26:14: 26:40]>>`
```

If you hit an error message like this, try pinning the value!

Before trying to run this, start the Mini-Redis server:

```
$ mini-redis-server
```

Then try running the code. We will see the messages outputted to STDOUT.

```
got = Ok(Message { channel: "numbers", content: b"1" })
got = Ok(Message { channel: "numbers", content: b"two" })
got = Ok(Message { channel: "numbers", content: b"3" })
got = Ok(Message { channel: "numbers", content: b"four" })
got = Ok(Message { channel: "numbers", content: b"five" })
got = Ok(Message { channel: "numbers", content: b"6" })
```

Some early messages may be dropped as there is a race between subscribing and
publishing. The program never exits. A subscription to a Mini-Redis channel
stays active as long as the server is active.

Let's see how we can work with streams to expand on this program.

# Adapters

Functions that take a [`Stream`](https://docs.rs/futures-core/0.3/futures_core/stream/trait.Stream.html) and return another [`Stream`](https://docs.rs/futures-core/0.3/futures_core/stream/trait.Stream.html) are often called
'stream adapters', as they're a form of the 'adapter pattern'. Common stream
adapters include [`map`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.map), [`take`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.take), and [`filter`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.filter).

Lets update the Mini-Redis so that it will exit. After receiving three messages,
stop iterating messages. This is done using [`take`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.take). This adapter limits the
stream to yield at **most** `n` messages.

```
let messages = subscriber
    .into_stream()
    .take(3);
```

Running the program again, we get:

```
got = Ok(Message { channel: "numbers", content: b"1" })
got = Ok(Message { channel: "numbers", content: b"two" })
got = Ok(Message { channel: "numbers", content: b"3" })
```

This time the program ends.

Now, let's limit the stream to single digit numbers. We will check this by
checking for the message length. We use the [`filter`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.filter) adapter to drop any
message that does not match the predicate.

```
let messages = subscriber
    .into_stream()
    .filter(|msg| match msg {
        Ok(msg) if msg.content.len() == 1 => true,
        _ => false,
    })
    .take(3);
```

Running the program again, we get:

```
got = Ok(Message { channel: "numbers", content: b"1" })
got = Ok(Message { channel: "numbers", content: b"3" })
got = Ok(Message { channel: "numbers", content: b"6" })
```

Note that the order in which adapters are applied matters. Calling `filter`
first then `take` is different than calling `take` then `filter`.

Finally, we will tidy up the output by stripping the `Ok(Message { ... })` part
of the output. This is done with [`map`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.map). Because this is applied **after**
`filter`, we know the message is `Ok`, so we can use `unwrap()`.

```
let messages = subscriber
    .into_stream()
    .filter(|msg| match msg {
        Ok(msg) if msg.content.len() == 1 => true,
        _ => false,
    })
    .map(|msg| msg.unwrap().content)
    .take(3);
```

Now, the output is:

```
got = b"1"
got = b"3"
got = b"6"
```

Another option would be to combine the [`filter`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.filter) and [`map`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.map) steps into a single call using [`filter_map`](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.filter_map).

There are more available adapters. See the list [here](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html).

# Implementing `Stream`

The [`Stream`](https://docs.rs/futures-core/0.3/futures_core/stream/trait.Stream.html) trait is very similar to the [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) trait.

```
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Stream {
    type Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}
```

The `Stream::poll_next()` function is much like `Future::poll`, except it can be called
repeatedly to receive many values from the stream. Just as we saw in [Async in
depth](async), when a stream is **not** ready to return a value, `Poll::Pending`
is returned instead. The task's waker is registered. Once the stream should be
polled again, the waker is notified.

The `size_hint()` method is used the same way as it is with [iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html).

Usually, when manually implementing a `Stream`, it is done by composing futures
and other streams. As an example, let's build off of the `Delay` future we
implemented in [Async in depth](async). We will convert it to a stream that
yields `()` three times at 10 ms intervals.

```
use tokio_stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

struct Interval {
    rem: usize,
    delay: Delay,
}

impl Interval {
    fn new() -> Self {
        Self {
            rem: 3,
            delay: Delay { when: Instant::now() }
        }
    }
}

impl Stream for Interval {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<()>>
    {
        if self.rem == 0 {
            // No more delays
            return Poll::Ready(None);
        }

        match Pin::new(&mut self.delay).poll(cx) {
            Poll::Ready(_) => {
                let when = self.delay.when + Duration::from_millis(10);
                self.delay = Delay { when };
                self.rem -= 1;
                Poll::Ready(Some(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
```

## `async-stream`

Manually implementing streams using the [`Stream`](https://docs.rs/futures-core/0.3/futures_core/stream/trait.Stream.html) trait can be tedious.
Unfortunately, the Rust programming language does not yet support `async/await`
syntax for defining streams. This is in the works, but not yet ready.

The [`async-stream`](https://docs.rs/async-stream) crate is available as a temporary solution. This crate
provides a `stream!` macro that transforms the input into a stream. Using
this crate, the above interval can be implemented like this:

```
use async_stream::stream;
use std::time::{Duration, Instant};

stream! {
    let mut when = Instant::now();
    for _ in 0..3 {
        let delay = Delay { when };
        delay.await;
        yield ();
        when += Duration::from_millis(10);
    }
}
```

[![](/img/arrow-left.svg)Select](/tokio/tutorial/select)

[Topics![](/img/arrow-right.svg)](/tokio/topics)

Get Help:

[Edit this page](https://github.com/tokio-rs/website/edit/master/content/tokio/tutorial/streams.md)