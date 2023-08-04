use crate::{
    console::console_error,
    ws::{messages, WebsocketEvent, WebsocketService, WebsocketStatus},
};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use web_sys::window;
use yew::prelude::*;

pub(crate) fn get_status(ws: WSState) -> String {
    match ws.status {
        None => "connecting...".to_owned(),
        Some(status) => match status {
            Ok(status) => match status {
                WebsocketStatus::Connected => "connected".to_owned(),
                WebsocketStatus::Pending => "connecting...".to_owned(),
                WebsocketStatus::Disconnected(_e) => {
                    format!("disconnected, check that the command is still running")
                }
                WebsocketStatus::Error(e) => format!("error ({})", e),
            },
            Err(e) => format!("error ({})", e),
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
        Err(e) => Err(format!("{}", e)),
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
    let recreate = use_state(|| 0);
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
            recreate.clone(),
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

    {
        let recreate = recreate.clone();

        use_effect_with_deps(
            |status| {
                match status {
                    Some(status) => {
                        match status {
                            WebsocketStatus::Disconnected(_) => {
                                let callback = Closure::<dyn Fn()>::new(move || {
                                    recreate.set(*recreate + 1);
                                });
                                match window() {
                                    Some(window) => {
                                        match window
                                            .set_timeout_with_callback_and_timeout_and_arguments_0(
                                                callback.as_ref().unchecked_ref(),
                                                5000,
                                            ) {
                                            Ok(_) => {}
                                            Err(e) => {
                                                console_error!("error setting timeout: {:?}", e)
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                                callback.forget();
                            }
                            _ => {}
                        };
                    }
                    _ => {}
                };
            },
            Option::clone(&status),
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
