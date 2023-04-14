use html_query_ast::Action;
use html_query_ast::Expression;
use markup5ever::{LocalName, Namespace, QualName};
use scraper::{ElementRef, Html};
use serde_json::{Map, Value};
use std::collections::HashMap;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExpressionError {
    #[error("Selector `{0} returned no results")]
    EmptySelector(String),

    #[error("Unexpected empty root node")]
    EmptyRoot,
}

fn trim_whitespace(input: String) -> String {
    input.trim().to_string()
}

fn handle_expression(
    roots: &[ElementRef],
    rhs: &Expression,
    lhs: &Option<Box<Action>>,
) -> Result<Value, ExpressionError> {
    return match rhs {
        Expression::Selector(selector, original_selector) => {
            let first_root = roots.first().ok_or(ExpressionError::EmptyRoot)?;
            let new_roots: Vec<_> = first_root.select(selector).collect();
            let first_new_root = new_roots
                .first()
                .ok_or_else(|| ExpressionError::EmptySelector(original_selector.clone()))?;
            match lhs {
                None => Ok(Value::String(trim_whitespace(
                    first_new_root.text().collect(),
                ))),
                Some(lhs) => Ok(convert_to_output(lhs, &new_roots)),
            }
        }
        Expression::Attribute(attr) => {
            let first_root = roots.first().ok_or(ExpressionError::EmptyRoot)?;
            Ok(first_root
                .value()
                .attrs
                .get(&QualName::new(
                    None,
                    Namespace::from(""),
                    LocalName::from(attr.as_str()),
                ))
                .map_or(Value::Null, |v| {
                    Value::String(trim_whitespace(v.to_string()))
                }))
        }
        Expression::Text => {
            let first_root = roots.first().ok_or(ExpressionError::EmptyRoot)?;
            Ok(Value::String(trim_whitespace(first_root.text().collect())))
        }
        Expression::Parent => {
            let first_root = roots.first().ok_or(ExpressionError::EmptyRoot)?;
            let parent_root = ElementRef::wrap(first_root.parent().unwrap()).unwrap();
            match lhs {
                None => handle_expression(&[parent_root], &Expression::Text, &None),
                Some(lhs) => Ok(convert_to_output(lhs, &vec![parent_root])),
            }
        }
        Expression::Sibling(idx) => {
            let first_root = roots.first().ok_or(ExpressionError::EmptyRoot)?;
            let mut next_sibling_elements = first_root
                .next_siblings()
                .filter(|s| s.value().is_element());
            let chosen_sibling =
                ElementRef::wrap(next_sibling_elements.nth(*idx - 1).unwrap()).unwrap();
            match lhs {
                None => handle_expression(&[chosen_sibling], &Expression::Text, &None),
                Some(lhs) => Ok(convert_to_output(lhs, &vec![chosen_sibling])),
            }
        }
    };
}

fn convert_to_output(item: &Action, roots: &Vec<ElementRef>) -> Value {
    return match item {
        Action::ForEachChild(hashmap) => Value::Array(
            roots
                .iter()
                .map(|root| {
                    let map = hashmap
                        .iter()
                        .map(|(key, value)| (key.clone(), convert_to_output(value, &vec![*root])))
                        .collect::<Map<_, _>>();
                    Value::Object(map)
                })
                .collect(),
        ),
        Action::ForEachChildArray(action) => Value::Array(
            roots
                .iter()
                .map(|root| convert_to_output(action, &vec![*root]))
                .collect(),
        ),
        Action::Child(hashmap) => {
            let map = hashmap
                .iter()
                .map(|(key, value)| (key.clone(), convert_to_output(value, roots)))
                .collect::<Map<_, _>>();
            Value::Object(map)
        }
        Action::Expression(rhs, lhs) => handle_expression(roots, rhs, lhs).unwrap_or(Value::Null),
    };
}

pub fn extract(input: &str, actions: &HashMap<&str, Action>) -> Value {
    let fragment = Html::parse_fragment(input);
    let root = fragment.root_element();
    println!("html: {}", root.html());
    let hashmap = actions
        .into_iter()
        .map(|(key, value)| (key.to_string(), convert_to_output(&value, &vec![root])))
        .collect();
    Value::Object(hashmap)
}
