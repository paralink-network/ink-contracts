![](https://paralink.network/images/logo-sm-home.png)

### Ink! Smart Contracts
This repository contains smart contracts for a Substrate runtime.

Configure the compiler:
```
rustup component add rust-src --toolchain nightly
rustup target add wasm32-unknown-unknown --toolchain stable
```

Install dependencies:
```
cargo install canvas-node --git https://github.com/paritytech/canvas-node.git --tag v0.1.4 --force --locked
cargo install cargo-contract --vers 0.7.1 --force --locked
```

Deploy a local chain for testing:
```
canvas --dev --tmp
```
