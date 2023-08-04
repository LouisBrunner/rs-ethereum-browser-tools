#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

use ethers::core::{
    types::{
        transaction::{eip2718::TypedTransaction, eip712::Eip712},
        Address, Signature as EthSig, H256,
    },
    utils::hash_message,
};
pub use ethers::signers::Signer;
use http::ServerOptions;
use std::str::FromStr;
use tracing::{instrument, trace};

mod http;

/// An ethers Signer that uses keys held in a browser-based wallet (e.g. Metamask).
///
/// The Browser Signer passes signing requests to the browser through a WS API.
///
/// ```
/// use ethers::{core::types::H256, signers::Signer};
/// use ethers_signers_browser::BrowserSigner;
///
/// # async fn foo() -> Result<(), Box<dyn std::error::Error>> {
/// let chain_id = 1;
///
/// let signer = BrowserSigner::new(chain_id).await?;
/// let sig = signer.sign_message(H256::zero()).await?;
/// # Ok(())
/// # }
/// ```
pub struct BrowserSigner {
    chain_id: u64,
    server: http::Server,
    addresses: Vec<Address>,
    url: String,
}

impl std::fmt::Debug for BrowserSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserSigner").field("chain_id", &self.chain_id).finish()
    }
}

impl std::fmt::Display for BrowserSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BrowserSigner {{ chain_id: {} }}", self.chain_id)
    }
}

/// Errors produced by the BrowserSigner
#[derive(thiserror::Error, Debug)]
pub enum BrowserSignerError {
    /// Error from the browser
    #[error("browser error: {0}")]
    IO(#[from] std::io::Error),
    /// Error from the server
    #[error("server error: {0}")]
    ServerError(#[from] http::ServerError),
    #[error("{0}")]
    Other(String),
    /// Error type from Eip712Error message
    #[error("eip712 error: {0:?}")]
    Eip712Error(String),
    #[error("signature error: {0}")]
    SignatureError(#[from] ethers::core::types::SignatureError),
}

impl From<String> for BrowserSignerError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

fn prompt_user(url: String) -> Result<(), BrowserSignerError> {
    Ok(webbrowser::open(&url)?)
}

pub struct BrowserOptions {
    pub open_browser: bool,
    pub server: Option<ServerOptions>,
}

impl BrowserSigner {
    /// Instantiate a new signer from a chain id.
    ///
    /// This function retrieves the public addresses from the browser. It is therefore `async`.
    #[instrument(err, skip(chain_id))]
    pub async fn new(chain_id: u64) -> Result<BrowserSigner, BrowserSignerError> {
        Self::new_with_options(chain_id, BrowserOptions { open_browser: true, server: None }).await
    }

    pub async fn new_with_options(
        chain_id: u64,
        opts: BrowserOptions,
    ) -> Result<BrowserSigner, BrowserSignerError> {
        let server = http::Server::new(chain_id, opts.server).await?;

        let url = format!("http://localhost:{}?nonce={}", server.port(), server.nonce());
        println!("Please open your browser at {} and connect your wallet", url);
        if opts.open_browser {
            prompt_user(url.clone())?;
        }

        let addresses = server.get_user_addresses().await?;
        if addresses.is_empty() {
            return Err(BrowserSignerError::Other("no addresses found in browser".to_owned()));
        }

        Ok(Self { chain_id, server, addresses, url })
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }
}

#[async_trait::async_trait]
impl Signer for BrowserSigner {
    type Error = BrowserSignerError;

    #[instrument(err, skip(message))]
    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<EthSig, Self::Error> {
        let message = message.as_ref();
        let message_hash = hash_message(message);
        trace!("{:?}", message_hash);
        trace!("{:?}", message);
        let raw_sig = self.server.sign_message(message_hash).await?;
        let sig = EthSig::from_str(&raw_sig)?;
        Ok(sig)
    }

    #[instrument(err)]
    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<EthSig, Self::Error> {
        let mut tx_with_chain = tx.clone();
        let chain_id = tx_with_chain.chain_id().map(|id| id.as_u64()).unwrap_or(self.chain_id);
        tx_with_chain.set_chain_id(chain_id);
        let raw_sig = self.server.sign_transaction(tx_with_chain).await?;
        let sig = EthSig::from_str(&raw_sig)?;
        Ok(sig)
    }

    async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        payload: &T,
    ) -> Result<EthSig, Self::Error> {
        let digest =
            payload.encode_eip712().map_err(|e| Self::Error::Eip712Error(e.to_string()))?;
        let message_hash = H256::from(digest);
        // FIXME: we can't use the actual eth_signTypedData because we aren't passed the right type
        let raw_sig = self.server.sign_message(message_hash).await?;
        let sig = EthSig::from_str(&raw_sig)?;
        Ok(sig)
    }

    fn address(&self) -> Address {
        self.addresses[0]
    }

    /// Returns the signer's chain id
    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Sets the signer's chain id
    fn with_chain_id<T: Into<u64>>(mut self, chain_id: T) -> Self {
        self.chain_id = chain_id.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg_attr(not(feature = "browser"), ignore)]
    async fn it_signs_messages() {
        let signer = BrowserSigner::new_with_options(
            1,
            BrowserOptions {
                open_browser: false,
                server: Some(ServerOptions { port: Some(7777), nonce: Some("123".to_owned()) }),
            },
        )
        .await
        .unwrap();

        println!("url: {}", signer.url());
        println!("address: {}", signer.address());

        let message = vec![0, 1, 2, 3];

        let sig = signer.sign_message(&message).await.unwrap();
        sig.verify(message, signer.address()).expect("valid sig");
    }
}
