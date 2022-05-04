# Suberra Core Contracts

Core contracts for Suberra.

![SuberraOverview](/docs/suberra-overview.jpg)

| Contract                             | Description                                                                                                            |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| `product-factory`                    | Factory that handles the instantiation and creation of the product subscriptions                                       |
| `subwallet-factory`                  | Factory contract is responsible for instantiating and storing the subwallets of users                                  |
| `jobs-registry`                      | Handles jobs creation and deletion. Entrypoint for workers to discover new contracts to perform work on                |
| `sub1-fixed-recurring-subscriptions` | Suberra module for managing fixed-period recurring payments. Stores subscriber information and enforces payment rules. |
| `sub2-p2p-recurring-transfers`       | Suberra module for managing peer-to-peer transfers                                                                     |
| `subwallet`                          | Smart contract wallet where users could deposit funds and use it to pay for any Suberra-compatible contract            |
| `admin-core`                         | Base contract that specifically handles the admin and owner separation of roles. Adapted from `cw1-subkeys` contract   |
| `token-stream`                       | A standalone contract that allow native or cw20 tokens to be streamed to a receiver                                    |

## Environment Setup

- Rust v1.44.1+
- `wasm32-unknown-unknown` target
- Docker

1. Install `rustup` via https://rustup.rs/

2. Run the following:

```sh
rustup default stable
rustup target add wasm32-unknown-unknown
```

3. Make sure [Docker](https://www.docker.com/) is installed

## Compiling and running tests

Now that you created your custom contract, make sure you can compile and run it before
making any changes. Go into the repository and do:

```sh
# builds the .wasm file from the contract
cargo build
# runs all test including integration tests
cargo test

# auto-generate json schema
cargo schema
```

## Documentation

Generate docs by running:

```
cargo doc --no-deps --open
```

## Code Coverage

```
cargo install cargo-tarpaulin
cargo tarpaulin -o html
```

## License

This repository is licensed under the Apache 2.0 license. See [LICENSE](./LICENSE) for full disclosure.

© 2021 Suberra
