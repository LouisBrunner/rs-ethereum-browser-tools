use self::console::console_log;
use ethereum_provider::{
    provider::{Provider, ProviderError},
    yew::{use_provider, ProviderStatus},
};
use serde::Serialize;
use yew::prelude::*;

mod console;
mod helpers;
mod ws;

#[derive(Serialize)]
struct SwitchEthereumChainParams {
    #[serde(rename = "chainId")]
    chain_id: String,
}

async fn call_provider(
    provider: Provider,
    request: ws::messages::Request,
) -> Result<ws::messages::Response, ProviderError> {
    match request.content {
        ws::messages::RequestContent::Init { chain_id } => {
            if chain_id != 0 {
                provider
                    .clone()
                    .request(
                        "wallet_switchEthereumChain".to_string(),
                        Some(ethereum_provider::provider::RequestMethodParams::Vec(vec![
                            SwitchEthereumChainParams { chain_id: format!("{:x}", chain_id) },
                        ])),
                    )
                    .await?;
            }
            let v = provider.clone().request::<()>("eth_requestAccounts".to_string(), None).await?;
            console_log!("eth_requestAccounts: {:?}", v);
            Ok(ws::messages::Response {
                id: request.id,
                content: ws::messages::ResponseContent::Init { addresses: vec![] },
            })
        }
        ws::messages::RequestContent::SignMessage { message } => {
            console_log!("message: {}", message);
            Err(ProviderError::Unsupported("sign message".to_string()))
        }
        ws::messages::RequestContent::SignTransaction { transaction } => {
            console_log!("transaction: {:?}", transaction);
            Err(ProviderError::Unsupported("sign transaction".to_string()))
        }
        ws::messages::RequestContent::SignTypedData { typed_data } => {
            console_log!("typed_data: {:?}", typed_data);
            Err(ProviderError::Unsupported("sign typed data".to_string()))
        }
    }
}

fn handle_request(
    args: helpers::ws::MessageCallbackArgs,
    status: &Option<Result<ProviderStatus, ProviderError>>,
) {
    let helpers::ws::MessageCallbackArgs { request, websocket } = args;

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
                                error: format!("{:?}", e),
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
            Err(e) => console_log!("error sending response: {:?}", e),
        }
    });
}

#[function_component]
fn App() -> Html {
    let status = use_provider();
    console_log!("status: {:?}", status);

    let callback = {
        let status = status.clone();
        use_callback(handle_request, status)
    };

    let ws = helpers::ws::use_ws(Some(callback));

    html! {
        <div>
            <pre style="text-align: center; font-size: 2rem;">{ "ethers-rs" }</pre>
            <pre>{ format!("Server connection: {}", helpers::ws::get_status(ws) )}</pre>
            {helpers::wallet::get_status(status)}
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
