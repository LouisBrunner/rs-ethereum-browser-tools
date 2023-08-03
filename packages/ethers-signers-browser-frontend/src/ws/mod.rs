use crate::console::console_error;
use futures_channel::mpsc::{channel, SendError, Sender};
use futures_util::{SinkExt, StreamExt};
use gloo_utils::errors::JsError;
use rand::Rng;
use reqwasm::websocket::{futures::WebSocket, Message, WebSocketError as WSError};
use std::sync::{Arc, Mutex};
use wasm_bindgen_futures::spawn_local;

pub mod messages;

#[derive(thiserror::Error, Debug)]
pub enum WebsocketError {
    #[error("js error: {0}")]
    JS(#[from] JsError),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("send error: {0}")]
    Send(#[from] SendError),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("{0}")]
    Other(String),
}

// FIXME: real one is not PartialEq
#[derive(Clone, Debug, PartialEq)]
pub struct CloseEvent {
    /// Close code
    pub code: u16,
    /// Close reason
    pub reason: String,
    /// If the websockets was closed cleanly
    pub was_clean: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum WebsocketStatus {
    Pending,
    Connected,
    Error(String),
    Disconnected(CloseEvent),
}

impl From<WebsocketError> for WebsocketStatus {
    fn from(err: WebsocketError) -> Self {
        Self::Error(err.to_string())
    }
}

#[derive(Clone)]
pub(super) enum WebsocketEvent {
    Message(messages::Request),
    Status(WebsocketStatus),
}

pub(super) type CallBack = yew::Callback<WebsocketEvent>;

pub(super) struct WebsocketService {
    id: usize,
    tx: Sender<String>,
    status: Arc<Mutex<WebsocketStatus>>,
    subscribers: Arc<Mutex<Vec<CallBack>>>,
}

impl WebsocketService {
    pub fn new(path: String, secure: bool) -> Result<Self, WebsocketError> {
        let id = rand::thread_rng().gen::<usize>();

        let scheme = if secure { "wss" } else { "ws" };
        let ws = WebSocket::open(format!("{}://{}", scheme, path).as_str())?;
        let (mut write, mut read) = ws.split();

        let (in_tx, mut in_rx) = channel::<String>(10);

        let subscribers = Arc::new(Mutex::new(Vec::<CallBack>::new()));
        let broadcast = {
            let subscribers = subscribers.clone();
            move |msg: WebsocketEvent| {
                let subs = subscribers.lock().expect("poisoned mutex");
                for sub in subs.iter() {
                    sub.emit(msg.clone());
                }
            }
        };

        let status = Arc::new(Mutex::new(WebsocketStatus::Pending));
        let set_status = {
            let broadcast = broadcast.clone();
            let status = status.clone();
            move |new_status: WebsocketStatus| {
                let mut pstatus = status.lock().expect("poisoned mutex");
                *pstatus = new_status.clone();
                broadcast(WebsocketEvent::Status(new_status));
            }
        };

        spawn_local(async move {
            while let Some(res) = in_rx.next().await {
                match write.send(Message::Text(res)).await {
                    Ok(_) => {}
                    Err(e) => {
                        console_error!("ws send error: {:?}", e);
                    }
                }
            }
        });

        {
            spawn_local(async move {
                while let Some(msg) = read.next().await {
                    set_status(WebsocketStatus::Connected);
                    match msg {
                        Ok(Message::Text(data)) => {
                            match serde_json::from_str::<messages::Request>(&data) {
                                Ok(req) => {
                                    broadcast(WebsocketEvent::Message(req));
                                }
                                Err(e) => {
                                    console_error!("ws receive error: {:?}", e)
                                }
                            }
                        }
                        Ok(_) => {
                            console_error!("ws unexpected message: {:?}", msg)
                        }
                        Err(e) => match e {
                            WSError::ConnectionClose(e) => {
                                set_status(WebsocketStatus::Disconnected(CloseEvent {
                                    code: e.code,
                                    reason: e.reason,
                                    was_clean: e.was_clean,
                                }));
                            }
                            _ => {
                                set_status(WebsocketStatus::Error(e.to_string()));
                            }
                        },
                    }
                }
            });
        }

        Ok(Self { id, tx: in_tx, status, subscribers })
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub async fn send(&mut self, msg: messages::Response) -> Result<(), WebsocketError> {
        self.tx.send(serde_json::to_string(&msg)?).await?;
        Ok(())
    }

    pub fn subscribe(&mut self, callback: CallBack) {
        self.subscribers.lock().expect("poisoned mutex").push(callback);
    }

    pub fn unsubscribe(&mut self, callback: CallBack) {
        let mut subs = self.subscribers.lock().expect("poisoned mutex");
        subs.retain(|sub| sub != &callback);
    }

    pub fn status(&self) -> WebsocketStatus {
        self.status.lock().expect("poisoned mutex").clone()
    }
}
