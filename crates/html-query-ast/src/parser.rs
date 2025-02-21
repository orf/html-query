// This is the first parser I've written using nom. It's based heavily off this one from Nom:
// https://github.com/Geal/nom/blob/3645656644e3ae5074b61cc57e3f62877ada9190/tests/json.rs

use nom::bytes::complete::{take_while, take_while1};
use nom::combinator::fail;
use nom::error::{convert_error, VerboseError};
use nom::multi::many_till;
use nom::sequence::{pair, terminated};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0},
    combinator::map,
    error::ParseError,
    multi::separated_list0,
    sequence::{delimited, preceded, separated_pair},
    IResult, Parser,
};
use scraper::Selector;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum Expression {
    // .foo | @text
    Text,
    // .foo | @(href)
    Attribute(String),
    // @parent
    Parent,
    // @sibling(1)
    Sibling(usize),
    // .abc > def
    Selector(Selector, String),
}

#[derive(Debug, PartialEq)]
pub enum Action {
    // selector | [{foo: name }]
    ForEachChild(HashMap<String, Box<Action>>),
    // selector | [.name]
    ForEachChildArray(Box<Action>),
    // selector | {foo: name }
    Child(HashMap<String, Box<Action>>),
    // .foo > bar | ...
    Expression(Expression, Option<Box<Action>>),
}

fn ws<'a, O, E: ParseError<&'a str>, F: Parser<&'a str, O, E>>(f: F) -> impl Parser<&'a str, O, E> {
    delimited(multispace0, f, multispace0)
}

fn alphanum_dash_underscore1(i: &str) -> IResult<&str, &str, VerboseError<&str>> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '-')(i)
}

fn object_key(i: &str) -> IResult<&str, &str, VerboseError<&str>> {
    terminated(alphanum_dash_underscore1, char(':'))(i)
}

fn object_key_suffix(i: &str) -> IResult<&str, &str, VerboseError<&str>> {
    preceded(
        ws(tag(",")),
        terminated(alphanum_dash_underscore1, char(':')),
    )(i)
}

fn expression_rhs(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    // This code is smelly. The issue here is parsing this expression:
    // {a: .foo, b: .foo | .bar}
    //     ^ here
    // We need to work out what bit to parse next. `, b: .foo` _could_ be part of
    // our selector? i.e `(.foo, b: .foo) | .bar`
    // But obviously that's not what we want. So here, we take until we find a `,`, `}` while
    // `object_key_suffix` does not match.
    let (_, (matches, _)): (_, (Vec<&str>, _)) = many_till(
        ws(take_while(|c: char| c != ',' && c != '}')),
        alt((ws(object_key_suffix), ws(tag("}")))),
    )(i)?;
    let rhs = match matches[..] {
        [first] if first.contains('|') => {
            let (rhs, _) = first.split_once('|').unwrap();
            rhs
        }
        _ => {
            fail::<_, &str, _>(i)?;
            unreachable!()
        }
    };
    let (_, expression) = expression(rhs)?;
    let new_rest = &i[rhs.len()..];
    Ok((new_rest, expression))
}

fn selector(i: &str) -> IResult<&str, (Selector, &str), VerboseError<&str>> {
    let (rest, value) = take_while(|c| !matches!(c, ',' | '}' | ']'))(i)?;
    match Selector::parse(value) {
        Ok(v) => Ok((rest, (v, value))),
        Err(_) => {
            fail::<_, &str, _>(i)?;
            unreachable!()
        }
    }
}

fn expression(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    alt((
        map(ws(tag("@parent")), |_| Expression::Parent),
        map(
            delimited(ws(tag("@(")), take_while(|c: char| c != ')'), ws(tag(")"))),
            |v: &str| Expression::Attribute(v.to_string()),
        ),
        map(preceded(ws(tag("@")), tag("text")), |_: &str| {
            Expression::Text
        }),
        map(
            delimited(
                tag("@sibling("),
                ws(take_while1(|c: char| c.is_ascii_digit())),
                tag(")"),
            ),
            |v: &str| Expression::Sibling(v.parse::<usize>().unwrap()),
        ),
        map(selector, |(sel, val)| Expression::Selector(sel, val.into())),
    ))(i)
}

fn object_value(i: &str) -> IResult<&str, Action, VerboseError<&str>> {
    alt((
        map(object, |v| {
            Action::Child(v.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
        }),
        map(delimited(ws(char('[')), object, ws(char(']'))), |v| {
            Action::ForEachChild(v.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
        }),
        map(delimited(ws(char('[')), object_value, ws(char(']'))), |v| {
            Action::ForEachChildArray(v.into())
        }),
        map(
            separated_pair(expression_rhs, ws(char('|')), object_value),
            |v: (Expression, Action)| Action::Expression(v.0, Some(v.1.into())),
        ),
        map(expression, |v: Expression| Action::Expression(v, None)),
    ))(i)
}

pub fn object(input: &str) -> IResult<&str, HashMap<&str, Action>, VerboseError<&str>> {
    map(
        delimited(
            char('{'),
            ws(separated_list0(
                ws(char(',')),
                pair(object_key, ws(object_value)),
            )),
            char('}'),
        ),
        |key_values| key_values.into_iter().collect(),
    )(input)
}

pub fn format_error(input: &str, error: VerboseError<&str>) -> String {
    convert_error(input, error)
}

#[test]
fn test_attribute() {
    let expected: HashMap<&str, Action> = vec![(
        "foo",
        Action::Expression(Expression::Attribute("abc".into()), None),
    )]
    .into_iter()
    .collect();

    assert_eq!(object("{foo: @(abc)}"), Ok(("", expected)));
}

#[test]
fn test_nested_attribute() {
    let expected: HashMap<&str, Action> = vec![(
        "foo",
        Action::Expression(
            Expression::Selector(Selector::parse(".abc").unwrap(), ".abc ".into()),
            Some(Action::Expression(Expression::Attribute("abc".into()), None).into()),
        ),
    )]
    .into_iter()
    .collect();

    assert_eq!(object("{foo: .abc | @(abc)}"), Ok(("", expected)));
}

#[test]
fn test_nested() {
    let expected: HashMap<&str, Action> = vec![(
        "foo",
        Action::Expression(
            Expression::Selector(Selector::parse(".bar").unwrap(), ".bar ".into()),
            Some(
                Action::ForEachChild(
                    [(
                        "baz".to_string(),
                        Box::new(Action::Expression(
                            Expression::Attribute("abc".into()),
                            None,
                        )),
                    )]
                    .into(),
                )
                .into(),
            ),
        ),
    )]
    .into_iter()
    .collect();

    assert_eq!(object("{foo: .bar | [{baz: @(abc)}]}"), Ok(("", expected)));
}

#[test]
fn test_array_attribute() {
    let expected: HashMap<&str, Action> = vec![(
        "foo",
        Action::Expression(
            Expression::Selector(Selector::parse(".bar").unwrap(), ".bar ".into()),
            Some(
                Action::ForEachChildArray(Box::new(Action::Expression(
                    Expression::Attribute("abc".into()),
                    None,
                )))
                .into(),
            ),
        ),
    )]
    .into_iter()
    .collect();

    assert_eq!(object("{foo: .bar | [@(abc)]}"), Ok(("", expected)));
}

#[test]
fn test_array_nested() {
    let expected: HashMap<&str, Action> = vec![(
        "foo",
        Action::Expression(
            Expression::Selector(Selector::parse(".bar").unwrap(), ".bar ".into()),
            Some(
                Action::ForEachChildArray(
                    Action::Expression(
                        Expression::Selector(Selector::parse(".lol ").unwrap(), ".lol ".into()),
                        Some(
                            Action::Expression(
                                Expression::Selector(Selector::parse("bar").unwrap(), "bar".into()),
                                None,
                            )
                            .into(),
                        ),
                    )
                    .into(),
                )
                .into(),
            ),
        ),
    )]
    .into_iter()
    .collect();

    assert_eq!(object("{foo: .bar | [.lol | bar]}"), Ok(("", expected)));
}
