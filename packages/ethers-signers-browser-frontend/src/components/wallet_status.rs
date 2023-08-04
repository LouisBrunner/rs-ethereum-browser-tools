use crate::components::label::Label;
use ethereum_provider::{yew::ProviderStatus, ProviderError};
use yew::prelude::*;

fn get_wallet_name(status: &ProviderStatus) -> String {
    if status.provider._is_coinbase_wallet.unwrap_or(false) {
        "CoinBase Wallet".to_owned()
    } else if status.provider._is_meta_mask.unwrap_or(false) {
        "MetaMask".to_owned()
    } else {
        "Unknown".to_owned()
    }
}

#[derive(Properties, PartialEq)]
pub(crate) struct WalletStatusProps {
    pub status: Option<Result<ProviderStatus, ProviderError>>,
}

#[function_component(WalletStatus)]
pub(crate) fn wallet_status(props: &WalletStatusProps) -> Html {
    match props.status.clone() {
        Some(status) => match status {
            Ok(status) => html! {
              <>
                <Label name="Wallet" value={ get_wallet_name(&status) } />
                <Label name="Chain ID" value={status.chain_id.unwrap_or("unknown".to_string())} />
                <Label name="Accounts" value={status.accounts.map_or("unknown".to_string(), |a| a.join(", "))} />
              </>
            },
            Err(e) => html! { <pre><Label name="Error" value={format!("{}", e)} /></pre> },
        },
        None => html! { <pre>{ "Loading wallet provider..." }</pre> },
    }
}
