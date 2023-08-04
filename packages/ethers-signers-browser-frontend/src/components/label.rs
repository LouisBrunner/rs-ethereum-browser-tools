use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct LabelProps {
    pub name: String,
    pub value: String,
}

#[function_component(Label)]
pub(crate) fn label(props: &LabelProps) -> Html {
    html! {
        <pre>
          <strong>{props.name.clone()}</strong>
          {": "}
          <span style="text-wrap: wrap;">{props.value.clone()}</span>
        </pre>
    }
}
