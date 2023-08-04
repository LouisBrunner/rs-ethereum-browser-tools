use ethereum_provider::{
    provider::{self, ProviderError},
    yew::ProviderStatus,
};
use ethers::{
    abi::Address,
    types::{transaction::eip2718::TypedTransaction, TransactionRequest},
};
use yew::prelude::*;

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
                      Some(chain_id) => chain_id,
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

pub(crate) fn address_to_string(address: Address) -> String {
    format!("{:x}", address)
}

fn transform_legacy_transaction(
    transaction: TransactionRequest,
) -> Result<(Option<u64>, provider::Transaction), String> {
    Ok((
        transaction.chain_id.map(|chain_id| chain_id.as_u64()),
        provider::Transaction {
            from: transaction.from.map(address_to_string).ok_or_else(|| "missing from address")?,
            to: transaction
                .to
                .map(|v| match v {
                    ethers::types::NameOrAddress::Address(address) => address_to_string(address),
                    ethers::types::NameOrAddress::Name(name) => name,
                })
                .ok_or_else(|| "missing to address")?,
            gas: transaction.gas.map(|gas| gas.as_u128()),
            gas_price: transaction.gas_price.map(|gas_price| gas_price.as_u128()),
            value: transaction.value.map(|value| value.as_u128()),
            data: transaction.data.map_or("".to_string(), |v| v.to_string()),
            nonce: transaction.nonce.map(|nonce| nonce.as_u128()),
        },
    ))
}

pub(crate) fn transform_transaction(
    transaction: TypedTransaction,
) -> Result<(Option<u64>, provider::Transaction), String> {
    Ok(match transaction {
        TypedTransaction::Legacy(transaction) => transform_legacy_transaction(transaction)?,
        TypedTransaction::Eip1559(transaction) => (
            transaction.chain_id.map(|chain_id| chain_id.as_u64()),
            provider::Transaction {
                from: transaction
                    .from
                    .map(address_to_string)
                    .ok_or_else(|| "missing from address")?,
                to: transaction
                    .to
                    .map(|v| match v {
                        ethers::types::NameOrAddress::Address(address) => {
                            address_to_string(address)
                        }
                        ethers::types::NameOrAddress::Name(name) => name,
                    })
                    .ok_or_else(|| "missing to address")?,
                gas: transaction.gas.map(|gas| gas.as_u128()),
                gas_price: transaction
                    .max_priority_fee_per_gas
                    .or(transaction.max_fee_per_gas)
                    .map(|gas_price| gas_price.as_u128()),
                value: transaction.value.map(|value| value.as_u128()),
                data: transaction.data.map_or("".to_string(), |v| v.to_string()),
                nonce: transaction.nonce.map(|nonce| nonce.as_u128()),
            },
        ),
        TypedTransaction::Eip2930(transaction) => transform_legacy_transaction(transaction.tx)?,
    })
}
