use components::{label::Label, wallet_status::WalletStatus};
use console::console_error;
use ethereum_provider::{
    provider::{Provider, ProviderError},
    yew::{use_provider, ProviderStatus},
};
use ethers::types::H160;
use helpers::ethers::{address_to_string, transform_transaction};
use hooks::use_ws::use_ws;
use std::str::FromStr;
use yew::prelude::*;

mod components;
mod console;
mod helpers;
mod hooks;
mod ws;

async fn call_provider(
    provider: Provider,
    request: ws::messages::Request,
) -> Result<ws::messages::Response, ProviderError> {
    match request.content {
        ws::messages::RequestContent::Init { chain_id } => {
            if chain_id != 0 {
                provider.clone().request_switch_chain(format!("{:x}", chain_id)).await?;
            }
            Ok(ws::messages::Response {
                id: request.id,
                content: ws::messages::ResponseContent::Init {},
            })
        }
        ws::messages::RequestContent::Accounts {} => {
            let accounts = provider
                .request_accounts()
                .await?
                .into_iter()
                .filter_map(|v| match H160::from_str(v.as_str()) {
                    Ok(address) => Some(address),
                    Err(err) => {
                        console_error!("error parsing address: {:?}", err);
                        None
                    }
                })
                .collect();
            Ok(ws::messages::Response {
                id: request.id,
                content: ws::messages::ResponseContent::Accounts { addresses: accounts },
            })
        }
        ws::messages::RequestContent::SignTextMessage { address, message } => {
            let sig = provider.request_sign_text(address_to_string(address), message).await?;
            Ok(ws::messages::Response {
                id: request.id,
                content: ws::messages::ResponseContent::Signature { signature: sig },
            })
        }
        ws::messages::RequestContent::SignBinaryMessage { address, message } => {
            let sig =
                provider.request_sign_hash(address_to_string(address), message.to_string()).await?;
            Ok(ws::messages::Response {
                id: request.id,
                content: ws::messages::ResponseContent::Signature { signature: sig },
            })
        }
        ws::messages::RequestContent::SignTransaction { transaction } => {
            let (chain_id, transaction) = match transform_transaction(transaction) {
                Ok(transaction) => transaction,
                Err(e) => return Err(ProviderError::Unsupported(format!("transaction: {}", e))),
            };
            if let Some(chain_id) = chain_id {
                provider.clone().request_switch_chain(format!("{:x}", chain_id)).await?;
            }
            let sig = provider.request_sign_transaction(transaction).await?;
            Ok(ws::messages::Response {
                id: request.id,
                content: ws::messages::ResponseContent::Signature { signature: sig },
            })
        }
        ws::messages::RequestContent::SignTypedData { address, typed_data } => {
            let sig =
                provider.request_sign_typed_data(address_to_string(address), typed_data).await?;
            Ok(ws::messages::Response {
                id: request.id,
                content: ws::messages::ResponseContent::Signature { signature: sig },
            })
        }
    }
}

fn handle_request(
    args: hooks::use_ws::MessageCallbackArgs,
    status: &Option<Result<ProviderStatus, ProviderError>>,
) {
    let hooks::use_ws::MessageCallbackArgs { request, websocket } = args;

    let status = status.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let res = match status {
            Some(Ok(status)) => {
                let provider = status.provider.clone();
                websocket
                    .lock()
                    .expect("poisoned mutex")
                    .send(match call_provider(provider, request.clone()).await {
                        Ok(response) => response,
                        Err(e) => ws::messages::Response {
                            id: request.id,
                            content: ws::messages::ResponseContent::Error {
                                error: format!("{}", e),
                            },
                        },
                    })
                    .await
            }
            _ => {
                websocket
                    .lock()
                    .expect("poisoned mutex")
                    .send(ws::messages::Response {
                        id: request.id,
                        content: ws::messages::ResponseContent::Error {
                            error: "no wallet available in your browser".to_string(),
                        },
                    })
                    .await
            }
        };
        match res {
            Ok(_) => (),
            Err(e) => console_error!("error sending response: {:?}", e),
        }
    });
}

#[function_component]
fn App() -> Html {
    let status = use_provider();
    let callback = {
        let status = status.clone();
        use_callback(handle_request, status)
    };
    let ws = use_ws(Some(callback));

    html! {
      <>
        <header style="display: flex; align-items: center; flex-direction: column;">
          <img width=128 height=128 src="static/logo.png" alt="App Logo"/>
          <h1 style="margin-top: 0;"><pre>{ "ethers-signers-browser" }</pre></h1>
        </header>
        <section style="max-width: 600px; margin: auto;">
          <Label name="Server connection" value={helpers::utils::get_ws_status(ws)} />
          <WalletStatus status={status} />
        </section>
      </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
