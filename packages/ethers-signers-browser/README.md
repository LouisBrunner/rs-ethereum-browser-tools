<p align="center"><img width=100 src="https://raw.githubusercontent.com/LouisBrunner/rs-ethereum-browser-tools/main/packages/ethers-signers-browser-frontend/static/logo.png" /></p>
<h1 align="center">ethers-signers-browser</h1>

A [`ethers-signers`](https://github.com/gakonst/ethers-rs)-compatible `Signer` which uses the browser's `window.ethereum` object to sign transactions, allowing you to use your Coinbase Wallet, MetaMask, or other browser-based Ethereum wallet from the comfort of the CLI.

For more information about how to use a signer, please refer to the [`ethers-rs` book](https://gakonst.com/ethers-rs).

## Installation

```bash
cargo add ethers-signers-browser
```

```toml
ethers-signers-browser = "0.2.0"
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

## Screenshots

Let's say you were running the following code:

```rust,no_run
use ethers::signers::Signer;
use ethers_signers_browser::BrowserSigner;

# async fn foo() -> Result<(), Box<dyn std::error::Error>> {
let signer = BrowserSigner::new(14).await.unwrap();
let message = "hello world".as_bytes();
let sig = signer.sign_message(&message).await.unwrap();
# Ok(())
# }
```

When the `BrowserSigner` is created, your browser will open a page and prompt you to unlock your wallet. The URL will look something like this: `http://localhost:PORT/?nonce=NONCE` where `PORT` and `NONCE` are random numbers, e.g. `http://localhost:7777/?nonce=123`.

You will then see the following page:

![Homepage of the signer displaying some metadata](https://raw.githubusercontent.com/LouisBrunner/rs-ethereum-browser-tools/main/packages/ethers-signers-browser/docs/0_homepage.png)

And, probably at the same time, a popup from your wallet:

![CoinBase Wallet popup to unlock your wallet](https://raw.githubusercontent.com/LouisBrunner/rs-ethereum-browser-tools/main/packages/ethers-signers-browser/docs/1_connection.png)

Once you have unlocked your wallet, your code will continue to run until it reaches `sign_message`, after which you will be prompted to sign the message:

![CoinBase Wallet popup to sign the message](https://raw.githubusercontent.com/LouisBrunner/rs-ethereum-browser-tools/main/packages/ethers-signers-browser/docs/2_signing.png)
