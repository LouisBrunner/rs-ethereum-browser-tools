# (Rust + Ethereum) Browser tools

This repository contains a collection of tools for interacting with Ethereum-based blockchains in Rust, in the browser.

## Packages

- [`ethers-signers-browser`](packages/ethers-signers-browser/): implement a [`ethers-signers`](https://github.com/gakonst/ethers-rs)-compatible `Signer` which uses the browser's `window.ethereum` object to sign transactions, allowing you to use your Coinbase Wallet, MetaMask, or other browser-based Ethereum wallet from the comfort of the CLI.
- [`ethereum-provider`](packages/rust-ethereum-provider/): implement a `Provider` which wraps the browser's `window.ethereum` for use in Rust, which is useful for wasm-based projects (e.g. front-ends).

## Credits

- The general structure of this project was inspired by [`ethers-signers`](https://github.com/gakonst/ethers-rs)
- The `ethereum-provider` package is inspired by [`EIP1193`](https://github.com/ZeroProphet/EIP1193_rs)
