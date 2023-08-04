use super::provider::{self, ProviderError};
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, Window};
use yew::prelude::*;

fn get_provider(window: &Option<Window>) -> Result<provider::Provider, ProviderError> {
    let window: &Window =
        window.as_ref().ok_or(ProviderError::Unsupported("no window available".to_owned()))?;
    let provider = provider::Provider::new(window)?;
    Ok(provider)
}

fn listen_to_provider(
    provider: provider::Provider,
    error: UseStateHandle<Option<ProviderError>>,
    chain_id: UseStateHandle<Option<String>>,
    accounts: UseStateHandle<Option<Vec<String>>>,
) -> Result<Box<dyn Fn()>, provider::ProviderError> {
    let chain_changed_cb = {
        let error = error.clone();
        Box::new(move |new_chain_id: Result<String, ProviderError>| match new_chain_id {
            Ok(new_chain_id) => chain_id.set(Some(new_chain_id)),
            Err(err) => error.set(Some(err)),
        })
    };

    let accounts_changed_cb =
        Box::new(move |new_accounts: Result<Vec<String>, ProviderError>| match new_accounts {
            Ok(new_accounts) => accounts.set(Some(new_accounts)),
            Err(err) => error.set(Some(err)),
        });

    let chain_changed_closure = provider.on_chain_changed(chain_changed_cb)?;
    let accounts_changed_closure = provider.on_accounts_changed(accounts_changed_cb)?;

    Ok(Box::new(move || {
        // FIXME: no error checking because it's too hard (and it's just for logging anyway)
        let _ = provider.remove_chain_changed_listener(&chain_changed_closure);
        let _ = provider.remove_accounts_changed_listener(&accounts_changed_closure);
    }))
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProviderStatus {
    pub provider: provider::Provider,
    pub chain_id: Option<String>,
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
                        match listen_to_provider(
                            provider.clone(),
                            error.clone(),
                            chain_id,
                            accounts,
                        ) {
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

    {
        let provider = provider.clone();
        let error = error.clone();
        let chain_id = chain_id.clone();
        let accounts_setter = accounts.clone();

        #[derive(PartialEq)]
        struct Deps {
            provider: Option<Rc<provider::Provider>>,
            chain_id: Option<String>,
        }
        let deps = Deps { provider: Option::clone(&provider), chain_id: Option::clone(&chain_id) };

        use_effect_with_deps(
            move |deps| {
                let Deps { provider, chain_id: _ } = deps;
                match provider {
                    None => {}
                    Some(provider) => {
                        let provider = provider.clone();
                        spawn_local(async move {
                            match provider.request_accounts().await {
                                Ok(accounts) => {
                                    accounts_setter.set(Some(accounts));
                                }
                                Err(err) => {
                                    error.set(Some(err));
                                    accounts_setter.set(None);
                                }
                            };
                        });
                    }
                }
            },
            deps,
        );
    }

    if let Some(ref err) = Option::clone(&error) {
        return Some(Err(err.clone()))
    };

    provider.as_deref().map(|provider| {
        Ok(ProviderStatus {
            provider: provider.clone(),
            chain_id: Option::clone(&chain_id),
            accounts: Option::clone(&accounts),
        })
    })
}
