# ethers-signers-browser

A [`ethers-signers`](https://github.com/gakonst/ethers-rs)-compatible `Signer` which uses the browser's `window.ethereum` object to sign transactions, allowing you to use your Coinbase Wallet, MetaMask, or other browser-based Ethereum wallet from the comfort of the CLI.

For more information about how to use a signer, please refer to the [`ethers-rs` book](https://gakonst.com/ethers-rs).

## Installation

```bash
cargo add ethers-signers-browser
```

```toml
ethers-signers-browser = "0.1.0"
```

## Examples

```rust,no_run
use ethers::{core::{k256::ecdsa::SigningKey, types::TransactionRequest}, signers::Signer};
use ethers_signers_browser::BrowserSigner;

# async fn foo() -> Result<(), Box<dyn std::error::Error>> {
// instantiate the wallet with a chain id,
// you will be prompted to unlock your wallet in the browser
let wallet = BrowserSigner::new(0).await?;

// create a transaction
let tx = TransactionRequest::new()
    .to("vitalik.eth") // this will use ENS
    .value(10000).into();

// sign it, again, you will be prompted to sign it in the browser
let signature = wallet.sign_transaction(&tx).await?;

// can also sign a message, again, you will be prompted to sign it in the browser
let signature = wallet.sign_message("hello world").await?;
signature.verify("hello world", wallet.address()).unwrap();
# Ok(())
# }
```
