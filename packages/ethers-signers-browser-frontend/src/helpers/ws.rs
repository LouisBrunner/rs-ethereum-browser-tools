use crate::ws::{messages, WebsocketEvent, WebsocketService, WebsocketStatus};
use std::sync::{Arc, Mutex};
use web_sys::window;
use yew::prelude::*;

pub(crate) fn get_status(ws: WSState) -> String {
    match ws.status {
        None => "connecting...".to_owned(),
        Some(status) => match status {
            Ok(status) => match status {
                WebsocketStatus::Connected => "connected".to_owned(),
                WebsocketStatus::Pending => "connecting...".to_owned(),
                WebsocketStatus::Disconnected(e) => format!("disconnected ({:?})", e),
                WebsocketStatus::Error(e) => format!("error ({:?})", e),
            },
            Err(e) => format!("error ({:?})", e),
        },
    }
}

fn create_ws() -> Result<WebsocketService, String> {
    let window = window().ok_or("no window")?;
    let host = window.location().host().map_err(|e| format!("{:?}", e))?;
    let secure = match window.location().protocol() {
        Ok(protocol) => protocol == "https:",
        Err(_) => false,
    };
    match WebsocketService::new(format!("{}/ws/", host), secure) {
        Ok(ws) => Ok(ws),
        Err(e) => Err(format!("{:?}", e)),
    }
}

pub(crate) struct MessageCallbackArgs {
    pub request: messages::Request,
    pub websocket: Arc<Mutex<WebsocketService>>,
}

pub(crate) type MessageCallback = yew::Callback<MessageCallbackArgs>;

pub(crate) struct WSState {
    status: Option<Result<WebsocketStatus, String>>,
}

#[hook]
pub(crate) fn use_ws(on_message: Option<MessageCallback>) -> WSState {
    let websocket = use_state(|| None);
    let status = use_state(|| None);
    let err = use_state(|| None);

    {
        let websocket = websocket.clone();
        let status = status.clone();
        let err = err.clone();
        use_effect_with_deps(
            move |_| match create_ws() {
                Ok(ws) => {
                    let ws = Arc::new(Mutex::new(ws));
                    websocket.set(Some(ws.clone()));
                    status.set(Some(ws.lock().expect("poisoned mutex").status()));
                    err.set(None);
                }
                Err(e) => {
                    websocket.set(None);
                    status.set(None);
                    err.set(Some(e));
                }
            },
            (),
        );
    }

    {
        let on_message = on_message.clone();
        let websocket = websocket.clone();
        let status = status.clone();

        #[derive(PartialEq, Clone)]
        struct Deps {
            on_message: Option<MessageCallback>,
            websocket_id: Option<usize>,
        }
        let deps = Deps {
            on_message: on_message.clone(),
            websocket_id: Option::clone(&websocket).map(|w| w.lock().expect("poisoned mutex").id()),
        };

        use_effect_with_deps(
            move |deps| -> Box<dyn Fn() -> ()> {
                let Deps { on_message, .. } = deps.clone();

                match Option::clone(&websocket) {
                    Some(websocket) => {
                        let callback = {
                            let websocket = websocket.clone();
                            Callback::from(move |msg| {
                                match msg {
                                    WebsocketEvent::Message(msg) => {
                                        if let Some(on_message) = Option::clone(&on_message) {
                                            on_message.emit(MessageCallbackArgs {
                                                request: msg,
                                                websocket: websocket.clone(),
                                            });
                                        }
                                    }
                                    WebsocketEvent::Status(s) => {
                                        status.set(Some(s));
                                    }
                                };
                            })
                        };

                        websocket
                            .clone()
                            .lock()
                            .expect("poisoned mutex")
                            .subscribe(callback.clone());

                        Box::new(move || {
                            websocket.lock().expect("poisoned mutex").unsubscribe(callback.clone());
                        })
                    }
                    _ => Box::new(|| {}),
                }
            },
            deps,
        );
    }

    WSState {
        status: match Option::clone(&err) {
            Some(err) => Some(Err(err.clone())),
            _ => match Option::clone(&status) {
                Some(status) => Some(Ok(status)),
                _ => None,
            },
        },
    }
}
