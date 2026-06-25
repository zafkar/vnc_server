# vnc_server

A small VNC server implementation in Rust.

## About

This repository contains a Rust implementation of a VNC server, including modules for capture, input handling, the VNC protocol, and server-side connection management.

## Features

- Basic VNC server skeleton and protocol handling
- Screen/frame capture utilities
- Input (keyboard,Mouse) controller

## Build

To build the project in release mode:

```sh
cargo build --release
```

### Dependencies

| Feature  | Required librairies        | 
|----------|----------------------------|
| auth_pam | libclang-dev, libpam0g-dev |


## Run

Run the server with:

```sh
cargo run --release
```
