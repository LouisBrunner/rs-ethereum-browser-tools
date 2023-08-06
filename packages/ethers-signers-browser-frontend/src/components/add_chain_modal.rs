use crate::components::{label::Label, text_input::TextInput};
use ethereum_provider::yew::{ChainInfo, NativeCurrency as RNativeCurrency, ProviderStatus};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct AddChainModalProps {
    pub chain_id: u64,
    pub status: ProviderStatus,
}

#[function_component(AddChainModal)]
pub(crate) fn add_chain_modal(props: &AddChainModalProps) -> Html {
    let loading = use_state(|| false);
    let error = use_state(|| None);
    let chain_name = use_state(|| None);
    let rpc_url = use_state(|| None);
    let icon_url = use_state(|| None);
    let nc_name = use_state(|| None);
    let nc_symbol = use_state(|| None);
    let nc_decimals = use_state(|| None);
    let block_explorer_url = use_state(|| None);

    let submit = {
        let loading = loading.clone();
        let error = error.clone();
        let chain_name = chain_name.clone();
        let rpc_url = rpc_url.clone();
        let icon_url = icon_url.clone();
        let nc_name = nc_name.clone();
        let nc_symbol = nc_symbol.clone();
        let nc_decimals = nc_decimals.clone();
        let block_explorer_url = block_explorer_url.clone();

        #[derive(PartialEq, Clone)]
        struct Deps {
            status: ProviderStatus,
            chain_name: UseStateHandle<Option<String>>,
            rpc_url: UseStateHandle<Option<String>>,
            icon_url: UseStateHandle<Option<String>>,
            nc_name: UseStateHandle<Option<String>>,
            nc_symbol: UseStateHandle<Option<String>>,
            nc_decimals: UseStateHandle<Option<String>>,
            block_explorer_url: UseStateHandle<Option<String>>,
        }

        let deps = Deps {
            status: props.status.clone(),
            chain_name: chain_name.clone(),
            rpc_url: rpc_url.clone(),
            icon_url: icon_url.clone(),
            nc_name: nc_name.clone(),
            nc_symbol: nc_symbol.clone(),
            nc_decimals: nc_decimals.clone(),
            block_explorer_url: block_explorer_url.clone(),
        };

        use_callback(
            move |e: SubmitEvent, deps| {
                e.prevent_default();
                let loading = loading.clone();
                let error = error.clone();
                let deps = deps.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let Deps {
                        status,
                        chain_name,
                        rpc_url,
                        icon_url,
                        nc_name,
                        nc_symbol,
                        nc_decimals,
                        block_explorer_url,
                    } = deps;
                    loading.set(true);

                    let mut native_currency = None;
                    if let (Some(nc_name), Some(nc_symbol), Some(nc_decimals)) = (
                        Option::clone(&nc_name),
                        Option::clone(&nc_symbol),
                        Option::clone(&nc_decimals),
                    ) {
                        native_currency = Some(RNativeCurrency {
                            name: nc_name,
                            symbol: nc_symbol,
                            decimals: nc_decimals.parse().unwrap_or(0),
                        });
                    }

                    match status
                        .provide_chain_info(ChainInfo {
                            chain_name: Option::clone(&chain_name),
                            rpc_urls: Option::clone(&rpc_url).map(|v| vec![v]),
                            icon_urls: Option::clone(&icon_url).map(|v| vec![v]),
                            native_currency,
                            block_explorer_urls: Option::clone(&block_explorer_url)
                                .map(|v| vec![v]),
                        })
                        .await
                    {
                        Ok(_) => {
                            loading.set(false);
                            error.set(None)
                        }
                        Err(e) => {
                            loading.set(false);
                            error.set(Some(e));
                        }
                    }
                });
            },
            deps,
        )
    };

    // FIXME: no validation before submitting

    html! {
      <>
        <div style="position: absolute;top: 0;left: 0;opacity: 0.3;background: black;right: 0;bottom: 0;" />

        <dialog open=true style="position: absolute;height: auto;top: 10%;left: 10%;right: 10%;bottom: 10%;width: auto;overflow-y: scroll;display: flex;flex-direction: column;max-width: 700px;">
          <h3 style="text-align: center; margin-top: 0;text-wrap: wrap;">
            <pre>{format!("Chain {} is unknown, please provide details about it", props.chain_id)}</pre>
          </h3>
          <div style="min-height: 50px">
            if let Some(err) = Option::clone(&error) {
              <Label name="Error" value={format!("{}", err)} />
            }
          </div>
          <form onsubmit={submit} style="display: flex; flex-direction: column; flex-grow: 1;">
            <div style="flex-grow: 1; margin: 20px 0;">
              <TextInput id="chain_name" label="Chain name" placeholder="CoolChain" state={chain_name.clone()} />
              <TextInput id="rpc_url" label="RPC URL" placeholder="https://website/api/rpc" state={rpc_url.clone()} />
              <TextInput id="icon_url" label="Icon URL" placeholder="https://website/icon.png" state={icon_url.clone()} />
              <TextInput id="nc_name" label="Native currency name" placeholder="CoolCoin" state={nc_name.clone()} />
              <TextInput id="nc_symbol" label="Native currency symbol" placeholder="CC" state={nc_symbol.clone()} />
              <TextInput id="nc_decimals" label="Native currency decimals" placeholder="18" state={nc_decimals.clone()} />
              <TextInput id="block_explorer_url" label="Block explorer URL" placeholder="https://website/block/{block}" state={block_explorer_url.clone()} />
            </div>
            <div>
              <button type="submit" disabled={*loading}><code>{if *loading { "Loading" } else { "Add"}}</code></button>
            </div>
          </form>
        </dialog>
      </>
    }
}
