use ethers_core::{
    abi::Address,
    types::{
        transaction::{eip2718::TypedTransaction, eip712::TypedData},
        H256,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Request {
    pub id: String,
    pub content: RequestContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "message")]
pub enum RequestContent {
    Init { chain_id: u64 },
    SignMessage { message: H256 },
    SignTransaction { transaction: TypedTransaction },
    SignTypedData { typed_data: TypedData },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub id: String,
    pub content: ResponseContent,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "message")]
pub enum ResponseContent {
    Init { addresses: Vec<Address> },
    Signature { signature: String },
    Error { error: String },
}
