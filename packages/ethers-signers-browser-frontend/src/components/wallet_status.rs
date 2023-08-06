use crate::components::{add_chain_modal::AddChainModal, label::Label};
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
                <Label name="Chain ID" value={status.clone().chain_id.unwrap_or("unknown".to_string())} />
                <Label name="Accounts" value={status.clone().accounts.map_or("unknown".to_string(), |a| a.join(", "))} />
                if let Some(chain_id) = status.clone().requires_chain_info() {
                  <AddChainModal chain_id={chain_id} status={status} />
                }
              </>
            },
            Err(e) => html! { <pre><Label name="Error" value={format!("{}", e)} /></pre> },
        },
        None => {
            html! { <pre>{ "You do not have a browser wallet, please install one to continue" }</pre> }
        }
    }
}
