mod parser;

use std::collections::HashMap;
use nom::error::VerboseError;
use nom::Finish;
pub use crate::parser::{Action, Expression};
pub use parser::format_error;

pub fn parse_string(input: &str) -> Result<HashMap<&str, Action>, VerboseError<&str>> {
    let (_, hashmap) = crate::parser::object(input).finish()?;
    Ok(hashmap)
}
