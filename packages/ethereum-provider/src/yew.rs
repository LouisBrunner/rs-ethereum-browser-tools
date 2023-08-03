use crate::errors::console_error;

use super::provider::{self, ProviderError};
use std::rc::Rc;
use web_sys::{window, Window};
use yew::prelude::*;

use wasm_bindgen::prelude::wasm_bindgen;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

fn get_provider(window: &Option<Window>) -> Result<provider::Provider, ProviderError> {
    let window: &Window =
        window.as_ref().ok_or(ProviderError::Unsupported("no window available".to_owned()))?;
    let provider = provider::Provider::new(window)?;
    Ok(provider)
}

fn listen_to_provider(
    provider: provider::Provider,
    chain_id: UseStateHandle<Option<u64>>,
    accounts: UseStateHandle<Option<Vec<String>>>,
) -> Result<Box<dyn Fn()>, provider::ProviderError> {
    let connect_cb = Box::new(|info: provider::ConnectInfo| {
        console_log!("on_connect: {:?}", info);
    });

    let chain_changed_cb =
        Box::new(move |new_chain_id: String| match new_chain_id.parse::<u64>() {
            Ok(value) => {
                chain_id.set(Some(value));
            }
            Err(err) => {
                console_error!("invalid chain id: {:?}", err);
                chain_id.set(None);
            }
        });

    let message_cb = Box::new(|message: provider::Message| {
        console_log!("on_message: {:?}", message);
    });

    let accounts_changed_cb = Box::new(move |new_accounts: Vec<String>| {
        accounts.set(Some(new_accounts));
    });

    let disconnect_cb = Box::new(|err: provider::RPCError| {
        console_log!("on_disconnect: {:?}", err);
    });

    let connect_closure = provider.on_connect(connect_cb)?;
    let chain_changed_closure = provider.on_chain_changed(chain_changed_cb)?;
    let message_closure = provider.on_message(message_cb)?;
    let accounts_changed_closure = provider.on_accounts_changed(accounts_changed_cb)?;
    let disconnect_closure = provider.on_disconnect(disconnect_cb)?;

    Ok(Box::new(move || {
        // FIXME: no error checking because it's too hard (and it's just for logging)
        let _ = provider.remove_connect_listener(&connect_closure);
        let _ = provider.remove_chain_changed_listener(&chain_changed_closure);
        let _ = provider.remove_message_listener(&message_closure);
        let _ = provider.remove_accounts_changed_listener(&accounts_changed_closure);
        let _ = provider.remove_disconnect_listener(&disconnect_closure);
    }))
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProviderStatus {
    pub provider: provider::Provider,
    pub connected: Option<bool>,
    pub chain_id: Option<u64>,
    pub accounts: Option<Vec<String>>,
}

#[hook]
pub fn use_provider() -> Option<Result<ProviderStatus, ProviderError>> {
    let provider = use_state(|| None);
    let error = use_state(|| None);
    let chain_id = use_state(|| None);
    let accounts = use_state(|| None);

    {
        let provider = provider.clone();
        let error = error.clone();
        use_effect_with_deps(
            move |window| {
                match get_provider(window) {
                    Ok(p) => {
                        provider.set(Some(Rc::new(p)));
                        error.set(None);
                    }
                    Err(err) => {
                        provider.set(None);
                        error.set(Some(err));
                    }
                };
            },
            window(),
        );
    }

    {
        let provider = provider.clone();
        let error = error.clone();
        let chain_id = chain_id.clone();
        let accounts = accounts.clone();
        use_effect_with_deps(
            move |provider| -> Box<dyn Fn()> {
                match provider.as_deref() {
                    None => {
                        error.set(None);
                        Box::new(|| {})
                    }
                    Some(provider) => {
                        match listen_to_provider(provider.clone(), chain_id, accounts) {
                            Ok(cleanup) => {
                                error.set(None);
                                cleanup
                            }
                            Err(err) => {
                                error.set(Some(err));
                                Box::new(|| {})
                            }
                        }
                    }
                }
            },
            provider,
        );
    }

    if let Some(ref err) = Option::clone(&error) {
        return Some(Err(err.clone()));
    };

    provider.as_deref().map(|provider| {
        Ok(ProviderStatus {
            provider: provider.clone(),
            connected: None,
            chain_id: Option::clone(&chain_id),
            accounts: Option::clone(&accounts),
        })
    })
}
