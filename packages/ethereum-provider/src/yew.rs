pub use crate::provider::NativeCurrency;
use crate::provider::{ChainData, Provider, ProviderError};
use std::rc::Rc;
use tokio::sync::mpsc;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, Window};
use yew::prelude::*;

fn get_provider(window: &Option<Window>) -> Result<Provider, ProviderError> {
    let window: &Window =
        window.as_ref().ok_or(ProviderError::Unsupported("no window available".to_owned()))?;
    let provider = Provider::new(window)?;
    Ok(provider)
}

fn listen_to_provider(
    provider: Provider,
    error: UseStateHandle<Option<ProviderError>>,
    chain_id: UseStateHandle<Option<String>>,
    accounts: UseStateHandle<Option<Vec<String>>>,
) -> Result<Box<dyn Fn()>, ProviderError> {
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

#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub chain_name: Option<String>,
    pub rpc_urls: Option<Vec<String>>,
    pub icon_urls: Option<Vec<String>>,
    pub native_currency: Option<NativeCurrency>,
    pub block_explorer_urls: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ProviderStatus {
    /// The current provider
    pub provider: Provider,
    /// The current Chain ID
    pub chain_id: Option<String>,
    /// The accounts available on this provider with the current `chain_id`
    pub accounts: Option<Vec<String>>,

    requires_chain_info: UseStateHandle<Option<(u64, mpsc::Sender<()>)>>,
}

impl PartialEq for ProviderStatus {
    fn eq(&self, other: &Self) -> bool {
        self.provider == other.provider &&
            self.chain_id == other.chain_id &&
            self.accounts == other.accounts &&
            match (
                Option::clone(&self.requires_chain_info),
                Option::clone(&other.requires_chain_info),
            ) {
                (None, None) => true,
                (Some((a, _)), Some((b, _))) => a == b,
                _ => false,
            }
    }
}

impl ProviderStatus {
    /// Change the current `chain_id` with smart handling for missing chains, see
    /// `requires_chain_info`
    pub async fn change_chain(&self, chain_id: u64) -> Result<(), ProviderError> {
        match self.provider.request_switch_chain(format!("{:x}", chain_id)).await {
            Err(ProviderError::UnknownChain(e)) => {
                let (tx, mut rx) = mpsc::channel(1);
                self.requires_chain_info.set(Some((chain_id, tx)));
                rx.recv().await.ok_or(ProviderError::UnknownChain(e))
            }
            a => a,
        }
    }

    /// If `Some()` is returned it means you should call `provide_chain_info` to unblock the
    /// `change_chain` call
    pub fn requires_chain_info(&self) -> Option<u64> {
        self.requires_chain_info.as_ref().map(|(chain_id, _)| *chain_id)
    }

    pub async fn provide_chain_info(&self, info: ChainInfo) -> Result<(), ProviderError> {
        match Option::clone(&self.requires_chain_info) {
            None => Err(ProviderError::Unsupported("no chain info required".to_string())),
            Some((chain_id, sender)) => {
                let chain_id = format!("{:x}", chain_id);
                self.provider
                    .request_add_chain(ChainData {
                        chain_id,
                        chain_name: info.chain_name,
                        rpc_urls: info.rpc_urls,
                        icon_urls: info.icon_urls,
                        native_currency: info.native_currency,
                        block_explorer_urls: info.block_explorer_urls,
                    })
                    .await?;
                sender
                    .send(())
                    .await
                    .map_err(|_| ProviderError::Unsupported("send error".to_string()))?;
                self.requires_chain_info.set(None);
                Ok(())
            }
        }
    }
}

#[hook]
pub fn use_provider() -> Option<Result<ProviderStatus, ProviderError>> {
    let provider = use_state(|| None);
    let error = use_state(|| None);
    let chain_id = use_state(|| None);
    let requires_chain_info = use_state(|| None);
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
            provider: Option<Rc<Provider>>,
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
            requires_chain_info,
        })
    })
}
