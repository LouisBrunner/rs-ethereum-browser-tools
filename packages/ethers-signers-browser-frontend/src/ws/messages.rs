use ethers::core::{
    abi::Address,
    types::{
        transaction::{eip2718::TypedTransaction, eip712::TypedData},
        H256,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NativeCurrency {
    pub name: String,
    pub symbol: String,
    pub decimals: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChainInfo {
    pub chain_name: Option<String>,
    pub rpc_urls: Option<Vec<String>>,
    pub icon_urls: Option<Vec<String>>,
    pub native_currency: Option<NativeCurrency>,
    pub block_explorer_urls: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Request {
    pub id: String,
    pub content: RequestContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "message")]
pub enum RequestContent {
    Init { chain_id: u64, chains: Option<HashMap<u64, ChainInfo>> },
    Accounts {},
    SignBinaryMessage { address: Address, message: H256 },
    SignTextMessage { address: Address, message: String },
    SignTransaction { transaction: TypedTransaction },
    SignTypedData { address: Address, typed_data: TypedData },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub id: String,
    pub content: ResponseContent,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "message")]
pub enum ResponseContent {
    Init {},
    Accounts { addresses: Vec<Address> },
    MessageSignature { signature: String },
    TransactionSignature { signature: String },
    Error { error: String },
}
