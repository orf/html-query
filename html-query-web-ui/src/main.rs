#![allow(non_snake_case)]

use dioxus::html::{br, button, div, label, optgroup, option, pre, select, style, textarea};
use std::borrow::Cow;
use std::collections::HashMap;
// import the prelude to get access to the `rsx!` macro and the `Scope` and `Element` types
use dioxus::prelude::*;
use dioxus_web::use_eval;
use html_query_ast::{parse_string, Action};
use html_query_extractor::extract;
use serde::Deserialize;

fn main() {
    // launch the web app
    dioxus_web::launch(App);
}

#[derive(Deserialize)]
pub struct Examples {
    name: &'static str,
    url: &'static str,
    content: &'static str,
    examples: Vec<Example>,
}

#[derive(Deserialize)]
pub struct Example {
    expression: &'static str,
    description: &'static str,
}

extern crate web_sys;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

static HN_CONTENT: &'static str = include_str!("examples/hn.html");

type ExampleTuple<'a> = (&'static str, &'static str);

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn html_beautify(s: &str) -> JsValue;
}

// #[wasm_bindgen(module = "format.js")]
// extern "C" {
//     #[wasm_bindgen(js_name = "default")]
//     type Web3;
//
//     #[wasm_bindgen(constructor, js_class = "default")]
//     fn new(_: &Provider) -> Web3;
//
//     #[wasm_bindgen(static_method_of = Web3, getter, js_class = "default")]
//     fn givenProvider() -> Provider;
//
//     type Provider;
// }

#[inline_props]
fn Examples<'a>(cx: Scope<'a>, on_input: EventHandler<'a, ExampleTuple<'a>>) -> Element {
    let examples: Examples = serde_json::from_str(include_str!("examples/examples.json")).unwrap();

    let buttons = examples.examples.into_iter().map(|ex| {
        rsx!(
            button {
                class: "button",
                onclick: move |event| { on_input.call((HN_CONTENT, &ex.expression)) },
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

fn App(cx: Scope) -> Element {
    let expression = use_state(cx, || "{}".to_string());
    let parsed = parse_string(expression);
    let html = use_state(cx, || "foo".to_string());

    // log!("{:?}", html_beautify(html));

    let output = match &parsed {
        Ok(parsed) => {
            let output = extract(html, parsed);
            serde_json::to_string_pretty(&output).unwrap()
        }
        Err(_) => "".to_string(),
    };

    cx.render(rsx! {
        p {
            class: "title is-1",
            "hq: jq, but for HTML"
        }
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
            div { class: "column",
                textarea {
                    value: "{html}",
                    class: "textarea",
                    oninput: move |evt| html.set(evt.value.clone())
                },
                button {
                    class: "button",
                    onclick: move |event| { html.set(html_beautify(html.get()).as_string().unwrap()) },
                    "Format HTML"
                }
            }

            div { class: "column",
                pre { style: "white-space: pre-wrap;", code { "{output}" } }
            }
        }
    })
}
