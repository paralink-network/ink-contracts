## Trusted Oracle
Delegate the oracle jobs to a single, reputable data provider. The contract can have any number of users (smart contracts).
The governance of the contract is performed by the chosen contract admin. The contract charges a
`fee` - as set by admin - for each successful oracle request. The fees are distributed to the oracle providing the service.

### Test
```
cargo +nightly test
```

### Compile to wasm

```
cargo +nightly contract build
cargo +nightly contract generate-metadata
```
