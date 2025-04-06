#![allow(non_snake_case)]

// import the prelude to get access to the `rsx!` macro and the `Scope` and `Element` types
use dioxus::prelude::*;
use html_query_ast::parse_string;
use html_query_extractor::extract;
use log::LevelFilter;
use serde::Deserialize;

fn main() {
    // launch the web app
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    console_error_panic_hook::set_once();

    log::info!("starting app");
    dioxus_web::launch(app);
}

#[derive(Deserialize)]
pub struct Example {
    expression: &'static str,
    description: &'static str,
}

static HN_CONTENT: &str = include_str!("examples/hn.html");

type ExampleTuple<'a> = (&'static str, &'static str);

#[inline_props]
fn Examples<'a>(cx: Scope<'a>, on_input: EventHandler<'a, ExampleTuple<'a>>) -> Element {
    let examples: Vec<Example> =
        serde_json::from_str(include_str!("examples/examples.json")).unwrap();

    let buttons = examples.into_iter().map(|ex| {
        rsx!(
            button {
                class: "button",
                onclick: move |_event| { on_input.call((HN_CONTENT, ex.expression)) },
                "{ex.description}"
            }
        )
    });

    cx.render(rsx! {
        div { class: "block",
            div { class: "buttons", buttons }
        }
    })
}

fn app(cx: Scope) -> Element {
    let expression = use_state(cx, || "{}".to_string());
    let parsed = parse_string(expression);
    let html = use_state(cx, || "foo".to_string());

    let output = match &parsed {
        Ok(parsed) => {
            let output = extract(html, parsed);
            serde_json::to_string_pretty(&output).unwrap()
        }
        Err(_) => "".to_string(),
    };

    cx.render(rsx! {
        p { class: "title is-1", "hq: jq, but for HTML" }
        // p {
        //     class: "subtitle is-3",
        //     "test"
        // }

        Examples {
            on_input: move |event: ExampleTuple| {
                let (example_content, example_expression) = event;
                html.set(example_content.to_owned());
                expression.set(example_expression.to_owned());
            }
        }

        div { class: "block",
            textarea {
                // we tell the component what to render
                value: "{expression}",
                class: "input is-large",
                // and what to do when the value changes
                oninput: move |evt| expression.set(evt.value.clone())
            }
        }

        div { class: "columns",
            div { class: "column", textarea {
                value: "{html}",
                class: "textarea",
                oninput: move |evt| html.set(evt.value.clone())
            } }

            div { class: "column",
                pre { style: "white-space: pre-wrap;", code { "{output}" } }
            }
        }
    })
}
