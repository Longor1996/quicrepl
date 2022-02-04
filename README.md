# QUICREPL

A commandline REPL tool for the QUIC network protocol.

> TODO: Under construction! Not actually usable... for now.

## Roadmap

- [x] Implement basic QUIC client using Quinn.
- [x] Implement basic QUIC server using Quinn.
- [ ] Implement internal command parser.
  - Using [IMPRAL](https://github.com/Longor1996/impral)?
  - Need to finish parsing and cleanup semantics...
- [ ] Implement various commands.
  - [ ] Proxy TCP connections?
  - [ ] Proxy UDP connections?
  - [ ] WebTransport?
  - [ ] HTTP?
  - [ ] Telnet?
- [ ] Implement various client commands.
  - [ ] Sending and receiving files?
  - [ ] Sending text?
- [ ] Implement various server commands.
  - [ ] Pattern matching?

## Installation

> TODO: Create distribution channels.

### From Source
> e.g: Compiling from source.

- Install [git](https://git-scm.com/).
- Install [rustup](https://rustup.rs/) and necessary build-tools.
- Confirm that rust works:  
  `rustc --version`
- Clone this repository
- Jump into the directory:  
  `cd quicrepl`
- `cargo build`

## Usage

General Commandline Parameters:
```
quicrepl 0.1.0
quicrepl - The simple quic repl tool

USAGE:
    quicrepl [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -c, --cert <CERT>    TLS certificate in PEM format [env: QUICREPL_CERT=]
    -h, --host <HOST>    Hostname of the certificate/remote [env: QUICREPL_HOST=]
        --help           Print help information
    -k, --key <KEY>      TLS private key in PEM format [env: QUICREPL_KEY=]
    -V, --version        Print version information

SUBCOMMANDS:
    client    Start as a client and connect to a server
    help      Print this message or the help of the given subcommand(s)
    server    Start as a server and respond to clients
```

Client Commandline Parameters:
```
quicrepl client
Start as a client and connect to a server

USAGE:
    quicrepl client [ADDR]

ARGS:
    <ADDR>    Address to connect to [env: QUICREPL_ADDR=] [default: [::1]:4433]

OPTIONS:
    -h, --help    Print help information
```

Server Commandline Parameters:
```
quicrepl server 
Start as a server and respond to clients

USAGE:
    quicrepl.exe server [ADDR]

ARGS:
    <ADDR>    Address to listen on [env: QUICREPL_ADDR=] [default: [::1]:4433]

OPTIONS:
    -h, --help    Print help information
```

### Client Commands
The client has a read-eval-print-loop (REPL), with the following commands...

> ***TODO:*** Add commands.

### Server Commands
The server has a read-eval-print-loop (REPL), with the following commands...

> ***TODO:*** Add commands.
