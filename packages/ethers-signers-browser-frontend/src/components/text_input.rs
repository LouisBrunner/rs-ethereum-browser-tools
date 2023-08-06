use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct TextInputProps {
    pub id: String,
    pub label: String,
    pub placeholder: String,
    pub state: UseStateHandle<Option<String>>,
}

#[function_component(TextInput)]
pub(crate) fn text_input(props: &TextInputProps) -> Html {
    let callback = {
        let state = props.state.clone();
        use_callback(
            move |e: Event, _| {
                let input = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
                if let Some(input) = input {
                    state.set(match input.value().is_empty() {
                        true => None,
                        false => Some(input.value()),
                    });
                }
            },
            (),
        )
    };

    html! {
      <div style="margin: 7px 0; display: flex;">
        <label for={props.id.clone()}>
          <strong><code>{props.label.clone()}{": "}</code></strong>
        </label>
        <input
          style="font-family: monospace; margin-left: 7px; flex-grow: 1;"
          id={props.id.clone()}
          type="text"
          placeholder={props.placeholder.clone()}
          value={Option::clone(&props.state).unwrap_or("".to_string())}
          onchange={callback} />
      </div>
    }
}
