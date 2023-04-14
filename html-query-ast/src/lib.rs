mod parser;

pub use crate::parser::{Action, Expression};
use nom::error::VerboseError;
use nom::Finish;
pub use parser::format_error;
use std::collections::HashMap;

pub fn parse_string(input: &str) -> Result<HashMap<&str, Action>, VerboseError<&str>> {
    let (_, hashmap) = crate::parser::object(input).finish()?;
    Ok(hashmap)
}
