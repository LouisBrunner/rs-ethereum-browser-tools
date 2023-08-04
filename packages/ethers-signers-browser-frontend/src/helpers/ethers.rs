use ethereum_provider::provider::Transaction;
use ethers::{
    abi::Address,
    types::{transaction::eip2718::TypedTransaction, TransactionRequest},
};

pub(crate) fn address_to_string(address: Address) -> String {
    format!("{:x}", address)
}

fn transform_legacy_transaction(
    transaction: TransactionRequest,
) -> Result<(Option<u64>, Transaction), String> {
    Ok((
        transaction.chain_id.map(|chain_id| chain_id.as_u64()),
        Transaction {
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
) -> Result<(Option<u64>, Transaction), String> {
    Ok(match transaction {
        TypedTransaction::Legacy(transaction) => transform_legacy_transaction(transaction)?,
        TypedTransaction::Eip1559(transaction) => (
            transaction.chain_id.map(|chain_id| chain_id.as_u64()),
            Transaction {
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
