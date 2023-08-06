#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use ethers::signers::Signer;
use ethers::{
    core::types::{
        transaction::{eip2718::TypedTransaction, eip712::Eip712},
        Address, Signature as EthSig,
    },
    types::transaction::{eip2718::TypedTransactionError, eip712::TypedData},
    utils::{hash_message, hex, rlp},
};
pub use ethers_signers_browser_frontend::ws::messages::ChainInfo;
use http::ServerOptions;
use log::info;
use std::{collections::HashMap, str::FromStr};
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
        f.debug_struct("BrowserSigner")
            .field("chain_id", &self.chain_id)
            .field("url", &self.url)
            .finish()
    }
}

impl std::fmt::Display for BrowserSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BrowserSigner {{ chain_id: {}, url: {} }}", self.chain_id, self.url)
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
    /// Couldn't find any addresses in the browser
    #[error("no addresses found in browser")]
    NoAddressFound,
    /// Error while parsing the signature
    #[error("signature error: {0}")]
    SignatureError(#[from] ethers::core::types::SignatureError),
    /// Some methods are no supported
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// Error while parsing the tx signature
    #[error("transaction signature error: {0}")]
    TransactionSignatureHexError(#[from] hex::FromHexError),
    /// Error while parsing the tx signature
    #[error("transaction signature error: {0}")]
    TransactionSignatureRLPError(#[from] TypedTransactionError),
}

fn prompt_user(url: String) -> Result<(), BrowserSignerError> {
    Ok(webbrowser::open(&url)?)
}

pub struct BrowserOptions {
    /// A map of chain IDs to their info, which is used to prepopulate the browser if needed
    pub chains: Option<HashMap<u64, ChainInfo>>,
    /// Whether to open the browser automatically, defaults to true
    pub open_browser: Option<bool>,
    /// The server options, defaults to randomized
    pub server: Option<ServerOptions>,
}

impl BrowserSigner {
    /// Instantiate a new signer from a chain id.
    ///
    /// This function retrieves the public addresses from the browser. It is therefore `async`.
    #[instrument(err)]
    pub async fn new(chain_id: u64) -> Result<BrowserSigner, BrowserSignerError> {
        Self::new_with_options(
            chain_id,
            BrowserOptions { chains: None, open_browser: Some(true), server: None },
        )
        .await
    }

    pub async fn new_with_options(
        chain_id: u64,
        opts: BrowserOptions,
    ) -> Result<BrowserSigner, BrowserSignerError> {
        let server = http::Server::new(chain_id, opts.chains, opts.server).await?;

        let url = format!("http://localhost:{}?nonce={}", server.port(), server.nonce());
        info!("Please open your browser at {} and connect your wallet", url);
        if opts.open_browser.unwrap_or(true) {
            prompt_user(url.clone())?;
        }

        let addresses = server.get_user_addresses().await?;
        if addresses.is_empty() {
            return Err(BrowserSignerError::NoAddressFound)
        }

        Ok(Self { chain_id, server, addresses, url })
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }
}

pub trait TypedDataBrowserCompatible {
    fn as_browser_compatible(&self) -> Option<TypedData>;
}

impl TypedDataBrowserCompatible for TypedData {
    fn as_browser_compatible(&self) -> Option<TypedData> {
        Some(self.clone())
    }
}

impl BrowserSigner {
    pub async fn sign_typed_data_raw(
        &self,
        data: &TypedData,
    ) -> Result<EthSig, BrowserSignerError> {
        let sig = self.server.sign_typed_data(self.address(), data.clone()).await?;
        Ok(EthSig::from_str(&sig)?)
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
        let sig = match String::from_utf8(message.to_vec()) {
            Ok(s) => self.server.sign_text_message(self.address(), s).await,
            Err(_) => self.server.sign_binary_message(self.address(), message_hash).await,
        }?;
        Ok(EthSig::from_str(&sig)?)
    }

    #[instrument(err)]
    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<EthSig, Self::Error> {
        let mut tx = tx.clone();
        tx.set_chain_id(tx.chain_id().unwrap_or(self.chain_id.into()));
        let sig = self.server.sign_transaction(tx).await?;
        let sig = hex::decode(sig)?;
        let signed_rlp = rlp::Rlp::new(sig.as_slice());
        let (_, decoded_sig) = TypedTransaction::decode_signed(&signed_rlp)?;
        Ok(decoded_sig)
    }

    async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        _payload: &T,
    ) -> Result<EthSig, Self::Error> {
        Err(BrowserSignerError::Unsupported(
            "sign_typed_data is not supported, use sign_typed_data_raw instead".to_owned(),
        ))
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
    use std::vec;

    use ethers::types::{transaction::eip2930::AccessList, Eip1559TransactionRequest};
    use ethers_signers_browser_frontend::ws::messages::NativeCurrency;
    use serial_test::serial;

    use super::*;

    async fn test_signer_with_goerli() -> BrowserSigner {
        test_signer_with_options(5, None).await // goerli
    }

    async fn test_signer_with_unknown_chain() -> BrowserSigner {
        test_signer_with_options(7777, None).await // goerli
    }

    async fn test_signer_with_provided_chain() -> BrowserSigner {
        let mut chains = HashMap::new();
        chains.insert(
            114,
            ChainInfo {
                chain_name: Some("Flare Coston2".to_owned()),
                rpc_urls: Some(vec!["https://coston2-api.flare.network/ext/C/rpc".to_owned()]),
                icon_urls: Some(
                    vec!["https://docs.flare.network/assets/logo-C2FLR.png".to_owned()],
                ),
                native_currency: Some(NativeCurrency {
                    name: "Coston2 FLR".to_owned(),
                    symbol: "C2FLR".to_owned(),
                    decimals: 18,
                }),
                block_explorer_urls: Some(
                    vec!["https://coston2-explorer.flare.network".to_owned()],
                ),
            },
        );
        test_signer_with_options(114, Some(chains)).await // goerli
    }

    async fn test_signer_with_options(
        chain: u64,
        chains: Option<HashMap<u64, ChainInfo>>,
    ) -> BrowserSigner {
        BrowserSigner::new_with_options(
            chain,
            BrowserOptions {
                chains,
                open_browser: Some(false),
                server: Some(ServerOptions { port: Some(7777), nonce: Some("123".to_owned()) }),
            },
        )
        .await
        .unwrap()
    }

    // #[tokio::test]
    // #[serial]
    // #[cfg_attr(not(feature = "browser"), ignore)]
    // async fn it_signs_text_messages() {
    //     let signer = test_signer_with_goerli().await;

    //     println!("address: {:#x}", signer.address());

    //     let message = "hello world".as_bytes();

    //     let sig = signer.sign_message(&message).await.unwrap();
    //     sig.verify(message, signer.address()).expect("valid sig");
    // }

    // #[tokio::test]
    // #[serial]
    // #[cfg_attr(not(feature = "browser"), ignore)]
    // async fn it_signs_binary_messages() {
    //     let signer = test_signer_with_goerli().await;

    //     println!("address: {:#x}", signer.address());

    //     let message = vec![0x01, 0x02, 0x03];

    //     let sig = signer.sign_message(&message).await.unwrap();
    //     sig.verify(message, signer.address()).expect("valid sig");
    // }

    // #[tokio::test]
    // #[serial]
    // #[cfg_attr(not(feature = "browser"), ignore)]
    // async fn it_signs_transaction() {
    //     let signer = test_signer_with_goerli().await;

    //     println!("address: {:#x}", signer.address());

    //     let transaction = TypedTransaction::Eip1559(Eip1559TransactionRequest {
    //         from: Some(signer.address()),
    //         to: Some(ethers::types::NameOrAddress::Address(signer.address())),
    //         nonce: None,
    //         gas: None,
    //         max_priority_fee_per_gas: None,
    //         max_fee_per_gas: None,
    //         value: None,
    //         data: None,
    //         chain_id: None,
    //         access_list: AccessList(vec![]),
    //     });

    //     let sig = signer.sign_transaction(&transaction).await.unwrap();
    //     // FIXME: would be nicer to have an actual verify
    //     println!("sig: {:?}", sig);
    // }

    #[tokio::test]
    #[serial]
    #[cfg_attr(not(feature = "browser"), ignore)]
    async fn it_signs_a_transaction_for_an_unknown_chain() {
        let signer = test_signer_with_unknown_chain().await;

        println!("address: {:#x}", signer.address());

        let message = "hello coston2".as_bytes();

        let sig = signer.sign_message(&message).await.unwrap();
        sig.verify(message, signer.address()).expect("valid sig");
    }

    // #[tokio::test]
    // #[serial]
    // #[cfg_attr(not(feature = "browser"), ignore)]
    // async fn it_signs_a_transaction_for_provided_chain() {
    //     let signer = test_signer_with_provided_chain().await;

    //     println!("address: {:#x}", signer.address());

    //     let message = "hello coston2".as_bytes();

    //     let sig = signer.sign_message(&message).await.unwrap();
    //     sig.verify(message, signer.address()).expect("valid sig");
    // }
}
