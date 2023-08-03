use crate::errors::console_error;
use js_sys::{Function, Object};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::vec::Vec;
use wasm_bindgen::{closure::Closure, prelude::*, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::Window;

pub type Callback = Closure<dyn Fn(JsValue)>;

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

#[derive(Deserialize, Debug)]
#[repr(i64)]
pub enum ErrorCodes {
    UserRejectedRequest = 4001,
    Unauthorized = 4100,
    UnsupportedMethod = 4200,
    Disconnected = 4900,
    ChainDisconnected = 4901,
    Other(i64),
}

#[derive(Deserialize, Debug)]
pub struct RPCError {
    pub code: ErrorCodes,
    pub data: Option<Value>,
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

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum ProviderError {
    #[error("js error: {0}")]
    JS(String),
    #[error("deserialize error: {0}")]
    Deserialize(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

impl From<JsValue> for ProviderError {
    fn from(js: JsValue) -> Self {
        Self::JS(format!("{:?}", js))
    }
}

impl From<serde_wasm_bindgen::Error> for ProviderError {
    fn from(err: serde_wasm_bindgen::Error) -> Self {
        Self::Deserialize(err.to_string())
    }
}

static MESSAGE: &str = "message";
static CONNECT: &str = "connect";
static DISCONNECT: &str = "disconnect";
static CHAIN_CHANGED: &str = "chainChanged";
static ACCOUNTS_CHANGED: &str = "accountsChanged";

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

    pub fn on(&self, event: String, callback: &Callback) -> Result<(), ProviderError> {
        self.on.call2(&self.this, &JsValue::from(event), callback.as_ref().unchecked_ref())?;
        Ok(())
    }

    pub fn on_message(&self, callback: Box<dyn Fn(Message)>) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data| match serde_wasm_bindgen::from_value(data) {
            Ok(event) => callback(event),
            // FIXME: should forward the error to the user?
            Err(err) => console_error!("callback error: {:?}", err),
        });
        self.on(MESSAGE.to_owned(), &closure)?;
        Ok(closure)
    }

    pub fn on_connect(
        &self,
        callback: Box<dyn Fn(ConnectInfo)>,
    ) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data| match serde_wasm_bindgen::from_value(data) {
            Ok(event) => callback(event),
            // FIXME: should forward the error to the user?
            Err(err) => console_error!("callback error: {:?}", err),
        });
        self.on(CONNECT.to_owned(), &closure)?;
        Ok(closure)
    }

    pub fn on_disconnect(
        &self,
        callback: Box<dyn Fn(RPCError)>,
    ) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data| match serde_wasm_bindgen::from_value(data) {
            Ok(event) => callback(event),
            // FIXME: should forward the error to the user?
            Err(err) => console_error!("callback error: {:?}", err),
        });
        self.on(DISCONNECT.to_owned(), &closure)?;
        Ok(closure)
    }

    pub fn on_chain_changed(
        &self,
        callback: Box<dyn Fn(String)>,
    ) -> Result<Callback, ProviderError> {
        let closure =
            Closure::new(move |data: JsValue| match serde_wasm_bindgen::from_value(data.clone()) {
                Ok(event) => callback(event),
                // FIXME: should forward the error to the user?
                Err(err) =>
                // sometimes we get the wrong type, a float for some reason, so handle that
                {
                    match serde_wasm_bindgen::from_value::<f32>(data) {
                        Ok(event) => callback(event.to_string()),
                        Err(_) => {
                            // no luck, show the initial error
                            console_error!("callback error: {:?}", err);
                        }
                    }
                }
            });
        self.on(CHAIN_CHANGED.to_owned(), &closure)?;
        Ok(closure)
    }

    pub fn on_accounts_changed(
        &self,
        callback: Box<dyn Fn(Vec<String>)>,
    ) -> Result<Callback, ProviderError> {
        let closure = Closure::new(move |data| match serde_wasm_bindgen::from_value(data) {
            Ok(event) => callback(event),
            // FIXME: should forward the error to the user?
            Err(err) => console_error!("callback error: {:?}", err),
        });
        self.on(ACCOUNTS_CHANGED.to_owned(), &closure)?;
        Ok(closure)
    }

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

    pub async fn request<T>(
        &self,
        method: String,
        params: Option<RequestMethodParams<T>>,
    ) -> Result<JsValue, ProviderError>
    where
        T: Serialize,
    {
        let promise = self
            .request
            .call1(&self.this, &serde_wasm_bindgen::to_value(&RequestMethod { method, params })?)?;
        Ok(wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await?)
    }

    pub fn request_sync<T>(
        self,
        method: String,
        params: Option<RequestMethodParams<T>>,
        callback: Box<dyn Fn(Result<JsValue, ProviderError>)>,
    ) where
        T: Serialize + 'static, // FIXME: no!
    {
        spawn_local(async move { callback(self.request(method, params).await) });
    }
}
