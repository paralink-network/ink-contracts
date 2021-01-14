## (Trusted) Oracle Requested & Consumer
This is a sample contract that makes requests to the (Trusted) Oracle and receives callbacks with
the oracle job results.

It can be used as a template for Trusted Oracle integration into your smart contract.

### Test
```
cargo +nightly test
```

### Compile to wasm

```
cargo +nightly contract build
cargo +nightly contract generate-metadata
```
