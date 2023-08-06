use js_sys::{Function, Object};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, vec::Vec};
use wasm_bindgen::{closure::Closure, prelude::*, JsValue};
use web_sys::Window;

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum ProviderError {
    #[error("rcp error: {0}")]
    RPC(RPCError),
    #[error("deserialize error: {0}")]
    Deserialize(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

impl From<JsValue> for ProviderError {
    fn from(js: JsValue) -> Self {
        Self::Unsupported(format!("unsupported JS call: {:?}", js))
    }
}

impl From<serde_wasm_bindgen::Error> for ProviderError {
    fn from(err: serde_wasm_bindgen::Error) -> Self {
        Self::Deserialize(err.to_string())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Provider {
    this: JsValue,
    request: Function,
    // EIP-1193 uses EventEmitter instead of EventTarget for some god-forsaken reason
    on: Function,
    remove_listener: Function,
    // non-standards
    pub _providers: Option<Vec<Provider>>, // provided by CoinBase Wallet
    pub _is_coinbase_wallet: Option<bool>, // provided by CoinBase Wallet
    pub _is_meta_mask: Option<bool>,       // provided by MetaMask
}

impl Provider {
    pub fn new(win: &Window) -> Result<Self, ProviderError> {
        let provider =
            win.get("ethereum").ok_or(ProviderError::Unsupported("missing provider".to_owned()))?;
        Self::from_object(provider, true)
    }

    fn from_object(provider: Object, get_providers: bool) -> Result<Self, ProviderError> {
        let request = js_sys::Reflect::get(&provider, &JsValue::from("request"))?;
        let on = js_sys::Reflect::get(&provider, &JsValue::from("on"))?;
        let remove_listener = js_sys::Reflect::get(&provider, &JsValue::from("removeListener"))?;
        let is_coinbase_wallet =
            js_sys::Reflect::get(&provider, &JsValue::from("isCoinbaseWallet")).ok();
        let is_meta_mask = js_sys::Reflect::get(&provider, &JsValue::from("isMetaMask")).ok();
        let providers = if get_providers {
            js_sys::Reflect::get(&provider, &JsValue::from("providers"))
                .ok()
                .filter(|p| !p.is_undefined())
                .map(|p: JsValue| {
                    js_sys::Array::from(&p)
                        .to_vec()
                        .into_iter()
                        .filter_map(|v| Self::from_object(v.into(), false).ok())
                        .collect::<Vec<Provider>>()
                })
        } else {
            None
        };
        Ok(Self {
            this: provider.into(),
            request: request.into(),
            on: on.into(),
            remove_listener: remove_listener.into(),
            _providers: providers,
            _is_coinbase_wallet: is_coinbase_wallet.and_then(|v| v.as_bool()),
            _is_meta_mask: is_meta_mask.and_then(|v| v.as_bool()),
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Message {
    Generic(GenericMessage),
    Subscription(Subscription),
}

#[derive(Deserialize, Debug)]
pub struct GenericMessage {
    #[serde(rename = "type")]
    pub typ: String,
    pub data: Value,
}

#[derive(Deserialize, Debug)]
pub struct SubscriptionData {
    pub subscription: String,
    pub result: Value,
}

#[derive(Deserialize, Debug)]
pub struct Subscription {
    #[serde(rename = "type")]
    pub typ: String, // "eth_subscription",
    pub data: SubscriptionData,
}

#[derive(Deserialize, Debug)]
pub struct ConnectInfo {
    #[serde(rename = "chainId")]
    pub chain_id: String,
}

#[derive(Deserialize, Debug)]
pub enum Event {
    Message(Message),
    Subscription(Subscription),
    Connect(ConnectInfo),
    Disconnect(RPCError),
    ChainChanged(String),
    AccountsChanged(Vec<String>),
}

fn parse_js<T: for<'de> serde::Deserialize<'de>>(data: JsValue) -> Result<T, ProviderError> {
    match serde_wasm_bindgen::from_value(data.clone()) {
        Ok(event) => Ok(event),
        Err(err) => Err(ProviderError::from(err)),
    }
}

impl Provider {
    pub fn on(&self, event: String, callback: &Callback) -> Result<(), ProviderError> {
        self.on.call2(&self.this, &JsValue::from(event), callback.as_ref().unchecked_ref())?;
        Ok(())
    }

    pub fn on_message(
        &self,
        callback: Box<dyn Fn(Result<Message, ProviderError>)>,
    ) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data| callback(parse_js(data)));
        self.on(MESSAGE.to_owned(), &closure)?;
        Ok(closure)
    }

    pub fn on_connect(
        &self,
        callback: Box<dyn Fn(Result<ConnectInfo, ProviderError>)>,
    ) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data| callback(parse_js(data)));
        self.on(CONNECT.to_owned(), &closure)?;
        Ok(closure)
    }

    pub fn on_disconnect(
        &self,
        callback: Box<dyn Fn(Result<RPCError, ProviderError>)>,
    ) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data| callback(parse_js(data)));
        self.on(DISCONNECT.to_owned(), &closure)?;
        Ok(closure)
    }

    pub fn on_chain_changed(
        &self,
        callback: Box<dyn Fn(Result<String, ProviderError>)>,
    ) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data: JsValue| match parse_js(data.clone()) {
            Ok(event) => callback(Ok(event)),
            // sometimes we get the wrong type for some reason
            Err(err) => callback(parse_js::<f32>(data).map(|v| v.to_string()).map_err(|_| err)),
        });
        self.on(CHAIN_CHANGED.to_owned(), &closure)?;
        Ok(closure)
    }

    pub fn on_accounts_changed(
        &self,
        callback: Box<dyn Fn(Result<Vec<String>, ProviderError>)>,
    ) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data| callback(parse_js(data)));
        self.on(ACCOUNTS_CHANGED.to_owned(), &closure)?;
        Ok(closure)
    }
}

pub type Callback = Closure<dyn Fn(JsValue)>;

static MESSAGE: &str = "message";
static CONNECT: &str = "connect";
static DISCONNECT: &str = "disconnect";
static CHAIN_CHANGED: &str = "chainChanged";
static ACCOUNTS_CHANGED: &str = "accountsChanged";

impl Provider {
    pub fn remove_listener(&self, event: String, callback: &Callback) -> Result<(), ProviderError> {
        self.remove_listener.call2(
            &self.this,
            &JsValue::from(event),
            callback.as_ref().unchecked_ref(),
        )?;
        Ok(())
    }

    pub fn remove_message_listener(&self, callback: &Callback) -> Result<(), ProviderError> {
        self.remove_listener(MESSAGE.to_owned(), callback)
    }

    pub fn remove_connect_listener(&self, callback: &Callback) -> Result<(), ProviderError> {
        self.remove_listener(CONNECT.to_owned(), callback)
    }

    pub fn remove_disconnect_listener(&self, callback: &Callback) -> Result<(), ProviderError> {
        self.remove_listener(DISCONNECT.to_owned(), callback)
    }

    pub fn remove_chain_changed_listener(&self, callback: &Callback) -> Result<(), ProviderError> {
        self.remove_listener(CHAIN_CHANGED.to_owned(), callback)
    }

    pub fn remove_accounts_changed_listener(
        &self,
        callback: &Callback,
    ) -> Result<(), ProviderError> {
        self.remove_listener(ACCOUNTS_CHANGED.to_owned(), callback)
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum RequestMethodParams<T> {
    Vec(Vec<T>),
    Object(T),
}

#[derive(Serialize)]
pub struct RequestMethod<T> {
    pub method: String,
    pub params: Option<RequestMethodParams<T>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
// TODO: this is not working
#[serde(untagged)]
#[repr(i64)]
pub enum ErrorCodes {
    UserRejectedRequest = 4001,
    Unauthorized = 4100,
    UnsupportedMethod = 4200,
    Disconnected = 4900,
    ChainDisconnected = 4901,
    Other(i64),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RPCError {
    pub code: ErrorCodes,
    pub message: String,
    pub data: Option<Value>,
}

impl fmt::Display for RPCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error {:?}: {}", self.code, self.message)
    }
}

#[derive(Serialize)]
struct SwitchEthereumChainParams {
    #[serde(rename = "chainId")]
    chain_id: String,
}

#[derive(Serialize)]
enum TypedData<T: Serialize> {
    Address(String),
    Data(T),
}

#[derive(Serialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub gas: Option<u128>,
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<u128>,
    pub value: Option<u128>,
    pub data: String,
    pub nonce: Option<u128>,
}

static REQUEST_SWITCH_CHAIN_ID: &str = "wallet_switchEthereumChain";
static REQUEST_ACCOUNTS: &str = "eth_requestAccounts";
static REQUEST_PERSONAL_SIGN: &str = "personal_sign";
static REQUEST_SIGN: &str = "eth_sign";
static REQUEST_SIGN_TYPED_DATA: &str = "eth_signTypedData";
static REQUEST_SIGN_TRANSACTION: &str = "eth_signTransaction";

impl Provider {
    pub async fn request<T: Serialize>(
        &self,
        method: String,
        params: Option<RequestMethodParams<T>>,
    ) -> Result<JsValue, ProviderError> {
        let promise = self
            .request
            .call1(&self.this, &serde_wasm_bindgen::to_value(&RequestMethod { method, params })?)?;
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await.map_err(|e| {
            match serde_wasm_bindgen::from_value(e) {
                Ok(err) => ProviderError::RPC(err),
                Err(err) => ProviderError::Deserialize(err.to_string()),
            }
        })
    }

    // TODO: wallet_addEthereumChain missing
    // TODO: wallet_watchAsset missing
    // TODO: eth_sendTransaction missing
    // TODO: eth_sendRawTransaction missing
    // TODO: eth_newFilter missing
    // TODO: eth_newBlockFilter missing
    // TODO: eth_newPendingTransactionFilter missing
    // TODO: eth_getFilterChanges missing
    // TODO: eth_getFilterLogs missing
    // TODO: signTypedData_v1 missing
    // TODO: signTypedData_v3 missing
    // TODO: signTypedData_v4 missing

    pub async fn request_switch_chain(&self, chain_id: String) -> Result<(), ProviderError> {
        self.request(
            REQUEST_SWITCH_CHAIN_ID.to_owned(),
            Some(RequestMethodParams::Vec(vec![SwitchEthereumChainParams { chain_id }])),
        )
        .await?;
        Ok(())
    }

    pub async fn request_accounts(&self) -> Result<Vec<String>, ProviderError> {
        let data = self.request::<()>(REQUEST_ACCOUNTS.to_owned(), None).await?;
        parse_js(data)
    }

    pub async fn request_sign_text(
        &self,
        address: String,
        message: String,
    ) -> Result<String, ProviderError> {
        let data = self
            .request(
                REQUEST_PERSONAL_SIGN.to_owned(),
                Some(RequestMethodParams::Vec(vec![message, address])),
            )
            .await?;
        parse_js(data)
    }

    pub async fn request_sign_hash(
        &self,
        address: String,
        message_hash: String,
    ) -> Result<String, ProviderError> {
        let data = self
            .request(
                REQUEST_SIGN.to_owned(),
                Some(RequestMethodParams::Vec(vec![address, message_hash])),
            )
            .await?;
        parse_js(data)
    }

    pub async fn request_sign_typed_data<T: Serialize>(
        &self,
        address: String,
        data: T,
    ) -> Result<String, ProviderError> {
        let data = self
            .request(
                REQUEST_SIGN_TYPED_DATA.to_owned(),
                Some(RequestMethodParams::Vec(vec![
                    TypedData::Address(address),
                    TypedData::Data(data),
                ])),
            )
            .await?;
        parse_js(data)
    }

    pub async fn request_sign_transaction(
        &self,
        transaction: Transaction,
    ) -> Result<String, ProviderError> {
        let data = self
            .request(
                REQUEST_SIGN_TRANSACTION.to_owned(),
                Some(RequestMethodParams::Vec(vec![transaction])),
            )
            .await?;
        parse_js(data)
    }
}
