# ethereum-provider

This project implements a `Provider` type which wraps the browser's `window.ethereum` for use in Rust, which is useful for wasm-based projects (e.g. front-ends).

## Installation

```bash
cargo add ethereum-provider
```

```toml
ethereum-provider = "0.1.0"
```

## Features

- `yew` (optional): provides a hook, `use_provider`, which simplifies the interaction with the provider when using [`yew`](https://github.com/yewstack/yew)

## Examples

```rust,no_run
use ethereum_provider::{Provider, ProviderError};
use web_sys::window;

# async fn foo() -> Result<(), Box<dyn std::error::Error>> {
// create a provider
let provider = Provider::new(&window().unwrap())?;

// request accounts
let v = provider.request::<()>("eth_requestAccounts".to_string(), None).await?;
println!("eth_requestAccounts: {:?}", v);
# Ok(())
# }
```

### Yew examples

```rust,no_run
use ethereum_provider::yew::use_provider;
use yew::prelude::*;

#[function_component]
fn Wallet() -> Html {
    let status = use_provider();

    html! {
        <div>
            {
              match status {
                  Some(status) => match status {
                      Ok(status) => html! {
                        <div>
                          <pre>{ format!("Wallet: {:?}", status) }</pre>
                        </div>
                      },
                      Err(e) => html! { <pre>{ format!("Error: {:?}", e) }</pre> },
                  },
                  None => html! { <pre>{ "Loading wallet provider..." }</pre> },
              }
            }
        </div>
    }
}
```
