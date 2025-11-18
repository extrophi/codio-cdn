<!-- Source: https://tokio.rs/tokio/tutorial/framing -->

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

# Framing

We will now apply what we just learned about I/O and implement the Mini-Redis
framing layer. Framing is the process of taking a byte stream and converting it
to a stream of frames. A frame is a unit of data transmitted between two peers.
The Redis protocol frame is defined as follows:

```
use bytes::Bytes;

enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}
```

Note how the frame only consists of data without any semantics. The command
parsing and implementation happen at a higher level.

For HTTP, a frame might look like:

```
enum HttpFrame {
    RequestHead {
        method: Method,
        uri: Uri,
        version: Version,
        headers: HeaderMap,
    },
    ResponseHead {
        status: StatusCode,
        version: Version,
        headers: HeaderMap,
    },
    BodyChunk {
        chunk: Bytes,
    },
}
```

To implement framing for Mini-Redis, we will implement a `Connection` struct
that wraps a `TcpStream` and reads/writes `mini_redis::Frame` values.

```
use tokio::net::TcpStream;
use mini_redis::{Frame, Result};

struct Connection {
    stream: TcpStream,
    // ... other fields here
}

impl Connection {
    /// Read a frame from the connection.
    ///
    /// Returns `None` if EOF is reached
    pub async fn read_frame(&mut self)
        -> Result<Option<Frame>>
    {
        // implementation here
    }

    /// Write a frame to the connection.
    pub async fn write_frame(&mut self, frame: &Frame)
        -> Result<()>
    {
        // implementation here
    }
}
```

You can find the details of the Redis wire protocol [here](https://redis.io/topics/protocol). The full
`Connection` code is found [here](https://github.com/tokio-rs/mini-redis/blob/tutorial/src/connection.rs).

# Buffered reads

The `read_frame` method waits for an entire frame to be received before
returning. A single call to `TcpStream::read()` may return an arbitrary amount
of data. It could contain an entire frame, a partial frame, or multiple frames.
If a partial frame is received, the data is buffered and more data is read from
the socket. If multiple frames are received, the first frame is returned and
the rest of the data is buffered until the next call to `read_frame`.

If you haven't already, create a new file called `connection.rs`.

```
touch src/connection.rs
```

To implement this, `Connection` needs a read buffer field. Data is read from the
socket into the read buffer. When a frame is parsed, the corresponding data is
removed from the buffer.

We will use [`BytesMut`](https://docs.rs/bytes/1/bytes/struct.BytesMut.html) as the buffer type. This is a mutable version of
[`Bytes`](https://docs.rs/bytes/1/bytes/struct.Bytes.html).

```
use bytes::BytesMut;
use tokio::net::TcpStream;

pub struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream,
            // Allocate the buffer with 4kb of capacity.
            buffer: BytesMut::with_capacity(4096),
        }
    }
}
```

Next, we implement the `read_frame()` method.

```
use tokio::io::AsyncReadExt;
use bytes::Buf;
use mini_redis::Result;

pub async fn read_frame(&mut self)
    -> Result<Option<Frame>>
{
    loop {
        // Attempt to parse a frame from the buffered data. If
        // enough data has been buffered, the frame is
        // returned.
        if let Some(frame) = self.parse_frame()? {
            return Ok(Some(frame));
        }

        // There is not enough buffered data to read a frame.
        // Attempt to read more data from the socket.
        //
        // On success, the number of bytes is returned. `0`
        // indicates "end of stream".
        if 0 == self.stream.read_buf(&mut self.buffer).await? {
            // The remote closed the connection. For this to be
            // a clean shutdown, there should be no data in the
            // read buffer. If there is, this means that the
            // peer closed the socket while sending a frame.
            if self.buffer.is_empty() {
                return Ok(None);
            } else {
                return Err("connection reset by peer".into());
            }
        }
    }
}
```

Let's break this down. The `read_frame` method operates in a loop. First,
`self.parse_frame()` is called. This will attempt to parse a redis frame from
`self.buffer`. If there is enough data to parse a frame, the frame is returned
to the caller of `read_frame()`.Otherwise, we attempt to read more data from the
socket into the buffer. After reading more data, `parse_frame()` is called
again. This time, if enough data has been received, parsing may succeed.

When reading from the stream, a return value of `0` indicates that no more data
will be received from the peer. If the read buffer still has data in it, this
indicates a partial frame has been received and the connection is being
terminated abruptly. This is an error condition and `Err` is returned.

## The `Buf` trait

When reading from the stream, `read_buf` is called. This version of the read
function takes a value implementing [`BufMut`](https://docs.rs/bytes/1/bytes/buf/trait.BufMut.html) from the [`bytes`](https://docs.rs/bytes/) crate.

First, consider how we would implement the same read loop using `read()`.
`Vec<u8>` could be used instead of `BytesMut`.

```
use tokio::net::TcpStream;

pub struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
    cursor: usize,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream,
            // Allocate the buffer with 4kb of capacity.
            buffer: vec![0; 4096],
            cursor: 0,
        }
    }
}
```

And the `read_frame()` function on `Connection`:

```
use mini_redis::{Frame, Result};

pub async fn read_frame(&mut self)
    -> Result<Option<Frame>>
{
    loop {
        if let Some(frame) = self.parse_frame()? {
            return Ok(Some(frame));
        }

        // Ensure the buffer has capacity
        if self.buffer.len() == self.cursor {
            // Grow the buffer
            self.buffer.resize(self.cursor * 2, 0);
        }

        // Read into the buffer, tracking the number
        // of bytes read
        let n = self.stream.read(
            &mut self.buffer[self.cursor..]).await?;

        if 0 == n {
            if self.cursor == 0 {
                return Ok(None);
            } else {
                return Err("connection reset by peer".into());
            }
        } else {
            // Update our cursor
            self.cursor += n;
        }
    }
}
```

When working with byte arrays and `read`, we must also maintain a cursor
tracking how much data has been buffered. We must make sure to pass the empty
portion of the buffer to `read()`. Otherwise, we would overwrite buffered data.
If our buffer gets filled up, we must grow the buffer in order to keep reading.
In `parse_frame()` (not included), we would need to parse data contained by
`self.buffer[..self.cursor]`.

Because pairing a byte array with a cursor is very common, the `bytes` crate
provides an abstraction representing a byte array and cursor. The `Buf` trait is
implemented by types from which data can be read. The `BufMut` trait is
implemented by types into which data can be written. When passing a `T: BufMut`
to `read_buf()`, the buffer's internal cursor is automatically updated by
`read_buf`. Because of this, in our version of `read_frame`, we do not need to
manage our own cursor.

Additionally, when using `Vec<u8>`, the buffer must be **initialized**. `vec![0; 4096]` allocates an array of 4096 bytes and writes zero to every entry. When
resizing the buffer, the new capacity must also be initialized with zeros. The
initialization process is not free. When working with `BytesMut` and `BufMut`,
capacity is **uninitialized**. The `BytesMut` abstraction prevents us from
reading the uninitialized memory. This lets us avoid the initialization step.

# Parsing

Now, let's look at the `parse_frame()` function. Parsing is done in two steps.

1. Ensure a full frame is buffered and find the end index of the frame.
2. Parse the frame.

The `mini-redis` crate provides us with a function for both of these steps:

1. [`Frame::check`](https://docs.rs/mini-redis/0.4/mini_redis/frame/enum.Frame.html#method.check)
2. [`Frame::parse`](https://docs.rs/mini-redis/0.4/mini_redis/frame/enum.Frame.html#method.parse)

We will also reuse the `Buf` abstraction to help. A `Buf` is passed into
`Frame::check`. As the `check` function iterates the passed in buffer, the
internal cursor will be advanced. When `check` returns, the buffer's internal
cursor points to the end of the frame.

For the `Buf` type, we will use [`std::io::Cursor<&[u8]>`](https://doc.rust-lang.org/stable/std/io/struct.Cursor.html).

```
use mini_redis::{Frame, Result};
use mini_redis::frame::Error::Incomplete;
use bytes::Buf;
use std::io::Cursor;

fn parse_frame(&mut self)
    -> Result<Option<Frame>>
{
    // Create the `T: Buf` type.
    let mut buf = Cursor::new(&self.buffer[..]);

    // Check whether a full frame is available
    match Frame::check(&mut buf) {
        Ok(_) => {
            // Get the byte length of the frame
            let len = buf.position() as usize;

            // Reset the internal cursor for the
            // call to `parse`.
            buf.set_position(0);

            // Parse the frame
            let frame = Frame::parse(&mut buf)?;

            // Discard the frame from the buffer
            self.buffer.advance(len);

            // Return the frame to the caller.
            Ok(Some(frame))
        }
        // Not enough data has been buffered
        Err(Incomplete) => Ok(None),
        // An error was encountered
        Err(e) => Err(e.into()),
    }
}
```

The full [`Frame::check`](https://github.com/tokio-rs/mini-redis/blob/tutorial/src/frame.rs#L65-L103) function can be found [here](https://github.com/tokio-rs/mini-redis/blob/tutorial/src/frame.rs#L65-L103). We will
not cover it in its entirety.

The relevant thing to note is that `Buf`'s "byte iterator" style APIs are used.
These fetch data and advance the internal cursor. For example, to parse a frame,
the first byte is checked to determine the type of the frame. The function used
is [`Buf::get_u8`](https://docs.rs/bytes/1/bytes/buf/trait.Buf.html#method.get_u8). This fetches the byte at the current cursor's position and
advances the cursor by one.

There are more useful methods on the [`Buf`](https://docs.rs/bytes/1/bytes/buf/trait.Buf.html) trait. Check the [API docs](https://docs.rs/bytes/1/bytes/buf/trait.Buf.html)
for more details.

# Buffered writes

The other half of the framing API is the `write_frame(frame)` function. This
function writes an entire frame to the socket. In order to minimize `write`
syscalls, writes will be buffered. A write buffer is maintained and frames are
encoded to this buffer before being written to the socket. However, unlike
`read_frame()`, the entire frame is not always buffered to a byte array before
writing to the socket.

Consider a bulk stream frame. The value being written is `Frame::Bulk(Bytes)`.
The wire format of a bulk frame is a frame head, which consists of the `$`
character followed by the data length in bytes. The majority of the frame is the
contents of the `Bytes` value. If the data is large, copying it to an
intermediate buffer would be costly.

To implement buffered writes, we will use the [`BufWriter` struct](https://docs.rs/tokio/1/tokio/io/struct.BufWriter.html).
This struct is initialized with a `T: AsyncWrite` and implements `AsyncWrite`
itself. When `write` is called on `BufWriter`, the write does not go directly to
the inner writer, but to a buffer. When the buffer is full, the contents are
flushed to the inner writer and the inner buffer is cleared. There are also
optimizations that allow bypassing the buffer in certain cases.

We will not attempt a full implementation of `write_frame()` as part of the
tutorial. See the full implementation [here](https://github.com/tokio-rs/mini-redis/blob/tutorial/src/connection.rs#L159-L184).

First, the `Connection` struct is updated:

```
use tokio::io::BufWriter;
use tokio::net::TcpStream;
use bytes::BytesMut;

pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(4096),
        }
    }
}
```

Next, `write_frame()` is implemented.

```
use tokio::io::{self, AsyncWriteExt};
use mini_redis::Frame;

async fn write_frame(&mut self, frame: &Frame)
    -> io::Result<()>
{
    match frame {
        Frame::Simple(val) => {
            self.stream.write_u8(b'+').await?;
            self.stream.write_all(val.as_bytes()).await?;
            self.stream.write_all(b"\r\n").await?;
        }
        Frame::Error(val) => {
            self.stream.write_u8(b'-').await?;
            self.stream.write_all(val.as_bytes()).await?;
            self.stream.write_all(b"\r\n").await?;
        }
        Frame::Integer(val) => {
            self.stream.write_u8(b':').await?;
            self.write_decimal(*val).await?;
        }
        Frame::Null => {
            self.stream.write_all(b"$-1\r\n").await?;
        }
        Frame::Bulk(val) => {
            let len = val.len();

            self.stream.write_u8(b'$').await?;
            self.write_decimal(len as u64).await?;
            self.stream.write_all(val).await?;
            self.stream.write_all(b"\r\n").await?;
        }
        Frame::Array(_val) => unimplemented!(),
    }

    self.stream.flush().await;

    Ok(())
}
```

The functions used here are provided by [`AsyncWriteExt`](https://docs.rs/tokio/1/tokio/io/trait.AsyncWriteExt.html). They are available on
`TcpStream` as well, but it would not be advisable to issue single byte writes
without the intermediate buffer.

* [`write_u8`](https://docs.rs/tokio/1/tokio/io/trait.AsyncWriteExt.html#method.write_u8) writes a single byte to the writer.
* [`write_all`](https://docs.rs/tokio/1/tokio/io/trait.AsyncWriteExt.html#method.write_all) writes the entire slice to the writer.
* [`write_decimal`](https://github.com/tokio-rs/mini-redis/blob/tutorial/src/connection.rs#L225-L238) is implemented by mini-redis.

The function ends with a call to `self.stream.flush().await`. Because
`BufWriter` stores writes in an intermediate buffer, calls to `write` do not
guarantee that the data is written to the socket. Before returning, we want the
frame to be written to the socket. The call to `flush()` writes any data pending
in the buffer to the socket.

Another alternative would be to **not** call `flush()` in `write_frame()`.
Instead, provide a `flush()` function on `Connection`. This would allow the
caller to write queue multiple small frames in the write buffer then write them
all to the socket with one `write` syscall. Doing this complicates the
`Connection` API. Simplicity is one of Mini-Redis' goals, so we decided to
include the `flush().await` call in `fn write_frame()`.

[![](/img/arrow-left.svg)I/O](/tokio/tutorial/io)

[Async in depth![](/img/arrow-right.svg)](/tokio/tutorial/async)

Get Help:

[Edit this page](https://github.com/tokio-rs/website/edit/master/content/tokio/tutorial/framing.md)