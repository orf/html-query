mod parser;

use crate::parser::{Action, Expression};
use clap::Parser;
use markup5ever::{LocalName, Namespace, QualName};
use nom::Finish;
use scraper::{ElementRef, Html};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::{fs, io};
use thiserror::Error;

/// jq, but for HTML
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Query describing data to extract
    #[arg(index = 1)]
    query: String,

    /// Input file. Uses stdin if not given.
    #[arg(index = 2)]
    input_file: Option<PathBuf>,
}

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

fn parse_string(input: &str, actions: HashMap<&str, Action>) -> Value {
    let fragment = Html::parse_fragment(input);
    let root = fragment.root_element();
    let hashmap = actions
        .into_iter()
        .map(|(key, value)| (key.to_string(), convert_to_output(&value, &vec![root])))
        .collect();
    Value::Object(hashmap)
}

fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();
    match parser::object(&args.query).finish() {
        Ok((_, res)) => {
            let input_str = match args.input_file {
                None => {
                    let mut buf = String::new();
                    io::stdin().lock().read_to_string(&mut buf)?;
                    buf
                }
                Some(path) => fs::read_to_string(path)?,
            };
            let output = parse_string(input_str.as_str(), res);
            serde_json::to_writer(std::io::stdout().lock(), &output)?;
            println!();
        }
        Err(e) => {
            eprintln!(
                "Error parsing:\n{}",
                parser::format_error(args.query.as_str(), e)
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse_whitespace() {
        let html = include_str!("tests/whitespace.html");
        let (_, expr) = parser::object("{foo: h1}").finish().unwrap();
        assert_eq!(
            parse_string(html, expr),
            serde_json::json!({
                "foo": "This is some whitespace"
            })
        )
    }
}
