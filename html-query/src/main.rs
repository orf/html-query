use clap::Parser;
use html_query_ast::parse_string;
use html_query_extractor::extract;

use std::io::Read;
use std::path::PathBuf;
use std::{fs, io};

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

fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    match parse_string(&args.query) {
        Ok(res) => {
            let input_str = match args.input_file {
                None => {
                    let mut buf = String::new();
                    io::stdin().lock().read_to_string(&mut buf)?;
                    buf
                }
                Some(path) => fs::read_to_string(path)?,
            };
            let output = extract(input_str.as_str(), &res);
            serde_json::to_writer(std::io::stdout().lock(), &output)?;
            println!();
        }
        Err(e) => {
            eprintln!(
                "Error parsing:\n{}",
                html_query_ast::format_error(args.query.as_str(), e)
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
        let expr = parse_string("{foo: h1}").unwrap();
        assert_eq!(
            extract(html, &expr),
            serde_json::json!({
                "foo": "This is some whitespace"
            })
        )
    }
}
