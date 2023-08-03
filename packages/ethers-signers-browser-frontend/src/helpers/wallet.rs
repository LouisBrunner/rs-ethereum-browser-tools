use ethereum_provider::{provider::ProviderError, yew::ProviderStatus};
use yew::prelude::*;

pub(crate) fn get_name(status: ProviderStatus) -> String {
    if status.provider._is_coinbase_wallet.unwrap_or(false) {
        "CoinBase Wallet".to_owned()
    } else if status.provider._is_meta_mask.unwrap_or(false) {
        "MetaMask".to_owned()
    } else {
        "Unknown".to_owned()
    }
}

pub(crate) fn get_status(status: Option<Result<ProviderStatus, ProviderError>>) -> Html {
    {
        match status {
            Some(status) => match status {
                Ok(status) => html! {
                  <div>
                    <pre>{ format!("Wallet: {}", get_name(status)) }</pre>
                  </div>
                },
                Err(e) => html! { <pre>{ format!("Error: {:?}", e) }</pre> },
            },
            None => html! { <pre>{ "Loading wallet provider..." }</pre> },
        }
    }
}
