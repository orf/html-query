use nom::bytes::complete::take_while1;
use nom::character::complete::alphanumeric1;
use nom::character::is_alphanumeric;
use nom::combinator::cond;
use nom::sequence::pair;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_until1, take_while},
    character::complete::{alphanumeric1 as alphanumeric, anychar, char, one_of},
    combinator::{cut, map, opt, value},
    error::{context, convert_error, ContextError, ErrorKind, ParseError, VerboseError},
    multi::separated_list0,
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated},
    Err, IResult,
};
use std::collections::HashMap;
use std::str;
use std::str::FromStr;

use nom_supreme::error::{BaseErrorKind, ErrorTree, Expectation, StackContext};
use nom_supreme::parser_ext::ParserExt;

#[derive(Debug, PartialEq)]
pub enum SelectorType {
    Selector(String),
    Attribute(String),
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Selector(SelectorType),
    ChildSelector(Option<SelectorType>, HashMap<String, Action>),
    ForEachChildSelector(SelectorType, HashMap<String, Action>),
}

fn sp<'a>(i: &'a str) -> IResult<&'a str, &'a str, VerboseError<&str>> {
    let chars = " \t\r\n";

    // nom combinators like `take_while` return a function. That function is the
    // parser,to which we can pass the input
    let res = take_while(move |c| chars.contains(c))(i);
    // dbg!(i, &res);
    res
}

fn valid_identifier<'a>(i: &'a str) -> IResult<&'a str, &'a str, VerboseError<&str>> {
    take_while1(|c: char| !matches!(c, ':' | '}' | '|'))(i)
}

fn parse_identifier<'a>(i: &'a str) -> IResult<&'a str, &'a str, VerboseError<&str>> {
    escaped(valid_identifier, '\\', one_of("\"n\\"))(i)
}

fn parse_expr(i: &str) -> IResult<&str, SelectorType, VerboseError<&str>> {
    alt((
        map(
                delimited(
                    tag("@a["),
                    take_while1(|c: char| !matches!(c, ' ' | ']')),
                    char(']'),
                ),
                |v: &str| SelectorType::Attribute(v.to_string()),
            ),
        preceded(sp, map(parse_str, |v|SelectorType::Selector(v.to_string()))),
    ))(i)
}

fn parse_str<'a>(i: &'a str) -> IResult<&'a str, &'a str, VerboseError<&str>> {
    let parse_str_result = escaped(
        // take_while1(|c: char| !matches!(c, '|' | ',' | '}') ),
        valid_identifier,
        // alphanumeric,
        '\\',
        one_of("\"n\\"),
    )(i);
    // dbg!(i, &parse_str_result);
    eprintln!("Parse str result: {} {:?}", i, parse_str_result);
    parse_str_result
}

fn parse_string<'a>(i: &'a str) -> IResult<&'a str, &'a str, VerboseError<&str>> {
    // dbg!(i);
    context(
        "string",
        preceded(char('\"'), cut(terminated(parse_str, char('\"')))),
    )(i)
}

fn split_expression(i: &str) -> IResult<&str, Action, VerboseError<&str>> {
    let sep = map(
        pair(
            preceded(sp, parse_expr),
            pair(
                preceded(sp, alt((tag("|>"), tag("|[]")))),
                preceded(sp, hash),
            ),
        ),
        |(selector, (sep, hashmap))| {
            // println!("split map result {} {:?}", a, sep);
            return match sep {
                "|[]" => Action::ForEachChildSelector(selector, hashmap),
                "|>" => Action::ChildSelector(Some(selector), hashmap),
                _ => unreachable!("weird sep value? {}", sep),
            };
        },
    )(i);
    eprintln!("Split expression: Input {}, output: {:?}", i, sep);
    sep
}

fn parse_expression<'a>(i: &'a str) -> IResult<&'a str, Action, VerboseError<&str>> {
    let first = split_expression(i);
    let second = map(parse_expr, |v| Action::Selector(v))(i);
    dbg!(&first, &second);
    match (first, second) {
        (Ok(v), _) => return Ok(v),
        (_, Ok(v)) => return Ok(v),
        (Err(e), _) => return Err(e),
    }
}

fn key_value<'a>(i: &'a str) -> IResult<&'a str, (&'a str, Action), VerboseError<&str>> {
    separated_pair(
        preceded(sp, valid_identifier),
        cut(preceded(sp, char(':'))),
        json_value,
    )(i)
}

fn hash<'a>(i: &'a str) -> IResult<&'a str, HashMap<String, Action>, VerboseError<&str>> {
    eprintln!("Hash: {}", i);
    context(
        "map",
        preceded(
            char('{'),
            cut(terminated(
                map(
                    separated_list0(preceded(sp, char(',')), key_value),
                    |tuple_vec| {
                        tuple_vec
                            .into_iter()
                            .map(|(k, v)| (String::from(k), v))
                            .collect()
                    },
                ),
                preceded(sp, char('}')),
            )),
        ),
    )(i)
}

/// here, we apply the space parser before trying to parse a value
fn json_value<'a>(i: &'a str) -> IResult<&'a str, Action, VerboseError<&str>> {
    preceded(
        sp,
        alt((
            map(hash, |h| Action::ChildSelector(None, h)),
            parse_expression,
            map(parse_expr, |v| Action::Selector(v)),
        )),
    )(i)
}

/// the root element of a JSON parser is either an object or an array
fn root<'a>(i: &'a str) -> IResult<&'a str, Action, VerboseError<&str>> {
    delimited(
        sp,
        alt((map(hash, |v| Action::ChildSelector(None, v)),)),
        opt(sp),
    )(i)
}

pub fn expression(input: &str) -> IResult<&str, Action, VerboseError<&str>> {
    root(input)
}

pub fn format_error(input: &str, error: VerboseError<&str>) -> String {
    convert_error(input, error)
}
