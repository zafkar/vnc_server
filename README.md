# vnc_server

A small VNC server implementation in Rust.

## About

This repository contains a Rust implementation of a VNC server, including modules for capture, input handling, the VNC protocol, and server-side connection management.

## Features

- Basic VNC server skeleton and protocol handling
- Screen/frame capture utilities
- Input (keyboard) controller

## Requirements

- Rust toolchain (stable or nightly)

## Build

To build the project in release mode:

```sh
cargo build --release
```

## Run

Run the server with:

```sh
cargo run --release
```

Adjust command-line flags or configuration as implemented by `src/main.rs`.

## Project structure

- [Cargo.toml](Cargo.toml)
- [src/main.rs](src/main.rs)
- [src/lib.rs](src/lib.rs)
- [src/server/mod.rs](src/server/mod.rs)
- [src/server/client_connexion.rs](src/server/client_connexion.rs)
- [src/capture/frame.rs](src/capture/frame.rs)
- [src/input_controller/keyboard.rs](src/input_controller/keyboard.rs)
- [src/protocol/mod.rs](src/protocol/mod.rs)

## Contributing

Contributions welcome. Please open issues or pull requests for new features or fixes.

## License

This project does not include a license file. Add a `LICENSE` to indicate terms.
