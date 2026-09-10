# redist as in "Redist in Rust"
---
redist is a minimalist, console-handled Redis server, implementing most of **RESP** protocol.
---

## Commands

redist implement most of **RESP** commands as part of a compact parsing mechanism, which abstractly uses a `Decoder` trait for handling plain bytes parsing.

### Copyless parsing
The server uses a *copyless parsing model*, which quite literally designates a no-copy parser, written with the help of [this guide](https://dpbriggs.ca/blog/Implementing-A-Copyless-Redis-Protocol-in-Rust-With-Parsing-Combinators/]).
## Connectivity

**redist** uses the proprietary **RESP** protocol.

### Clients
The **redist** server handles *asynchronous clients* through spawning threads, and each client can *indefinitely* send payloads and commands to the server until an `EXIT` signal is send and the connection is broken apart.

