use std::collections::HashMap;

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, char, multispace0},
    combinator::{map, opt, recognize},
    error::{Error, ParseError},
    multi::many0,
    number::complete::double,
    sequence::{delimited, pair, preceded},
};

use crate::unit::Unit;
use crate::unit_registry::UnitRegistry;

#[derive(Debug)]
pub enum ParserError {
    NomError(String),
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::NomError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(alpha1, many0(alt((alphanumeric1, tag("_")))))).parse(input)
}

fn ws<'a, O, F>(inner: F) -> impl Parser<&'a str, Output = O, Error = Error<&'a str>>
where
    F: Parser<&'a str, Output = O, Error = Error<&'a str>>,
{
    delimited(multispace0::<&str, Error<&str>>, inner, multispace0::<&str, Error<&str>>)
}

/// Parse a number or fraction like `2`, `2.5`, or `2/3`.
fn fraction(input: &str) -> IResult<&str, f64> {
    map(
        pair(double, opt(preceded(ws(char('/')), double))),
        |(num, denom)| match denom {
            Some(d) => num / d,
            None => num,
        },
    )
    .parse(input)
}

/// Parse an exponent value: a plain number, or a parenthesized fraction like `(2/3)`.
fn exponent_value(input: &str) -> IResult<&str, f64> {
    alt((
        delimited(ws(char('(')), fraction, ws(char(')'))),
        double,
    ))
    .parse(input)
}

fn power_suffix(input: &str) -> IResult<&str, f64> {
    preceded(alt((tag("**"), tag("^"))), exponent_value).parse(input)
}

fn factor<'a>(
    input: &'a str,
    registry: &UnitRegistry,
) -> IResult<&'a str, Unit> {
    let (input, _) = multispace0::<&str, Error<&str>>.parse(input)?;

    if input.starts_with('(') {
        let (input, _) = char::<&str, Error<&str>>('(').parse(input)?;
        let (input, unit) = expression(input, registry)?;
        let (input, _) = ws(char(')')).parse(input)?;
        let (input, exp) = opt(power_suffix).parse(input)?;
        let unit = match exp {
            Some(e) => unit.pow(e),
            None => unit,
        };
        Ok((input, unit))
    } else {
        let (input, name) = identifier(input)?;
        let defined = registry.get(&name.to_string()).ok_or_else(|| {
            nom::Err::Failure(Error::new(input, nom::error::ErrorKind::Tag))
        })?;
        let (input, exp) = opt(power_suffix).parse(input)?;
        let unit = Unit::new(HashMap::from([(defined, exp.unwrap_or(1.0))]));
        Ok((input, unit))
    }
}

fn mul_div_op(input: &str) -> IResult<&str, char> {
    ws(alt((char('*'), char('/')))).parse(input)
}

fn expression<'a>(input: &'a str, registry: &UnitRegistry) -> IResult<&'a str, Unit> {
    let (mut input, mut result) = factor(input, registry)?;

    loop {
        let (remaining, _) = multispace0::<&str, Error<&str>>.parse(input)?;

        // Try explicit operator
        if let Ok((after_op, op)) = mul_div_op(remaining) {
            let (after_factor, rhs) = factor(after_op, registry)?;
            result = match op {
                '*' => result * rhs,
                '/' => result / rhs,
                _ => unreachable!(),
            };
            input = after_factor;
            continue;
        }

        // Try implicit multiplication: next char starts an identifier or '('
        if remaining
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '(')
        {
            let (after_factor, rhs) = factor(remaining, registry)?;
            result = result * rhs;
            input = after_factor;
            continue;
        }

        input = remaining;
        break;
    }

    Ok((input, result))
}

pub fn parse(registry: &UnitRegistry, input: &str) -> Result<Unit, ParserError> {
    match expression(input, registry) {
        Ok((_, unit)) => Ok(unit),
        Err(e) => Err(ParserError::NomError(format!("{}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::DefinedUnit;

    fn create_unit_registry() -> UnitRegistry {
        let kg_unit = DefinedUnit::new(
            "kg".to_string(),
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        );
        let s = DefinedUnit::new(
            "s".to_string(),
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        );
        let m = DefinedUnit::new(
            "m".to_string(),
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        );
        UnitRegistry::new(Vec::from_iter([kg_unit, s, m]))
    }

    #[test]
    fn test_parse_integer_exponent() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg**2").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(2.0));
    }

    #[test]
    fn test_parse_decimal_exponent() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg**2.5").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(2.5));
    }

    #[test]
    fn test_parse_fraction_exponent() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg**(2/5)").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(0.4));
    }

    #[test]
    fn test_parse_unit_multiplication_with_asterisk() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg * s").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(1.0));
        assert_eq!(unit.components.len(), 2);
    }

    #[test]
    fn test_parse_unit_division() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg / s").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(-1.0));
        assert_eq!(unit.components.len(), 2);
    }

    #[test]
    fn test_multiplication_and_division_precedence() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg / s * s").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), None);
        assert_eq!(unit.components.len(), 1);
    }

    #[test]
    fn test_brackets() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg / (s * s)").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(-2.0));
    }

    #[test]
    fn test_complex_precedence_mixed_operators() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg * m^2 / s^2").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"m".to_string()), Some(2.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(-2.0));
    }

    #[test]
    fn test_chained_exponentiation_with_fractions() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "m^(3/2) * kg^(1/3) / s^(2/3)").unwrap();
        assert_eq!(unit.get_exponent(&"m".to_string()), Some(1.5));
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0 / 3.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(-2.0 / 3.0));
    }

    #[test]
    fn test_same_unit_multiple_times() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg * kg * kg").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(3.0));
    }

    #[test]
    fn test_same_unit_cancellation() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg / kg").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), None);
    }

    #[test]
    fn test_implicit_multiplication() {
        let registry = create_unit_registry();
        let unit = parse(&registry, "kg s").unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(1.0));
    }
}
