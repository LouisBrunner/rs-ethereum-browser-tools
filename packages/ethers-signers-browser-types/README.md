# ethers-signers-browser-types

This is a crate for [`ethers-signers-browser`](https://crates.io/crates/ethers-signers-browser) that provides the types for client/server communication.

This is done to avoid:

- a circular dependency between `ethers-signers-browser` and `ethers-signers-browser-frontend`
- publishing `ethers-signers-browser-frontend` as a crate as it's only meaningful when paired with `ethers-signers-browser`
