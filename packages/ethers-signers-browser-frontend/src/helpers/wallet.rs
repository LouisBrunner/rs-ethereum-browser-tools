use ethereum_provider::{provider::ProviderError, yew::ProviderStatus};
use yew::prelude::*;

use crate::console::console_log;

pub(crate) fn get_name(status: &ProviderStatus) -> String {
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
                    <pre>{ format!("Wallet: {}", get_name(&status)) }</pre>
                    <pre>{ format!("Connected: {}", match status.connected {
                      Some(status) => format!("{}", status),
                      None => "unknown".to_owned(),
                    }) }</pre>
                    <pre>{ format!("Chain ID: {}", match status.chain_id {
                      Some(chain_id) => format!("{:x}", chain_id),
                      None => "unknown".to_owned(),
                    }) }</pre>
                    <pre>{ format!("Accounts: {}", match status.accounts {
                      Some(addresses) => addresses.join(", "),
                      None => "unknown".to_owned(),
                    }) }</pre>
                  </div>
                },
                Err(e) => html! { <pre>{ format!("Error: {}", e) }</pre> },
            },
            None => html! { <pre>{ "Loading wallet provider..." }</pre> },
        }
    }
}
