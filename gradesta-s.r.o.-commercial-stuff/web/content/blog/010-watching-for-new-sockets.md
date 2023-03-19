---
title: "Watching for new sockets"
date: 2023-03-11
featureimage: https://assets.gradesta.com/gradesta/img/dalle2-clock-and-coins.png
author: Timothy Hobbs <tim@gradesta.com>
draft: true
---

So I want the manager to watch the sockets directory for new sockets and start a listener on any sockest that are created or triggering some kind of event at least. I need this to be kcf style, so this has to be an independently testable unit of functionality. This means that emitting some kind of event is more like it. Probably it will be a function that gets called and returns a channel. Every time a new socket is created or destroyed, an event will be sent to the channel.

So perhaps the events will look like:

```rust
use std::path::Path;

enum SocketEvent {
    NewSocket(Path),
    SocketClosed(Path),
}
```

And then the function's signature will be something like:

```rust
fn watch_for_new_sockets(sockets_dir: Path, Sender<SocketEvent>) -> async Result<(), Error> {
    // ...
}
```

Now that I have that worked out I need to remember which rust lib is best for watching for file system updates. I already have notify in `Cargo.toml` so I'll use that.

```
TASK: Watching for new unix sockets
TASK_ID: a28c678f9cb4684164b62b8730b34155
CREATED: 2022-08-31 14:38
ESTIMATED_TIME: W4
MILESTONES: unix-sockets
```

{{<screencast "2023-03-07-7490e24a-566c-4bd3-b55c-fa9e1bff2f15" "a28c678f9cb4684164b62b8730b34155">}}

We are going to implement a unix socket actor which has various behaviors. This is a low level actor operating on byte buffers and does not deal with serialization/deserialization at all.

1. It passes on messages that it recieves to a rust channel
2. It sends messages it recieves on a rust channel to the unix socket
3. Tear everything down

I was a bit stuck on how actors work in Rust. Looking at actix I was unable to assertain how to actually use actix actors to listen for events external to them. AKA, they seem more like passive objects than actual stand alone actors. But I did find [an article](https://ryhl.io/blog/actors-with-tokio/) on implementing actors using tokio::spawn and it turns out that this is much more flexible and powerfull. I will be using this method, it's just so much easier to understand...

```
TASK: Unix socket actor: Recieve messages
TASK_ID: e2fc39b5773a5c785295a89bd3350db6
CREATED: 2023-03-13 09:40
ESTIMATED_TIME: W3
MILESTONES: unix-sockets

TASK: Unix socket actor: Send messages
TASK_ID: b1b00aaa9e42f329953c69ce403638de
CREATED: 2023-03-13 10:01
ESTIMATED_TIME: W2
MILESTONES: unix-sockets

TASK: Unix socket actor: Tear down
TASK_ID: b1b00aaa9e42f329953c69ce403638de
CREATED: 2023-03-13 10:01
ESTIMATED_TIME: U1 W4
MILESTONES: unix-sockets
```

```
Something watching → new socket events → spawning of message handler tasks 
```

Trying to figure out hang when watching for new sockets
-----------------------------------------------------------------

{{<screencast "2023-03-14-f434f500-b082-40d9-82bc-18d26dfaf655" "a28c678f9cb4684164b62b8730b34155">}}

Trying to figure out hang when watching for new sockets again
-----------------------------------------------------------------

{{<screencast "2023-03-19-901d9435-895a-41d6-9a8e-9e8382ee3432"  "a28c678f9cb4684164b62b8730b34155">}}

