// A Pratt parser - the algorithm is based on binding precedence
// See https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html for details!
use std::collections::HashMap;

use crate::tokenizer::{Token, TokenizerError, tokenize};
use crate::unit::Unit;
use crate::unit_registry::UnitRegistry;

#[derive(Debug)]
pub enum ParserError {
    TokenizerError(TokenizerError),
    ExpectedNumber(String),
    UnknownUnit,
    UnexpectedToken(String),
    UnexpectedEnd,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::TokenizerError(e) => write!(f, "{}", e),
            ParserError::ExpectedNumber(msg) => write!(f, "Expected number: {}", msg),
            ParserError::UnknownUnit => write!(f, "Unknown unit"),
            ParserError::UnexpectedToken(msg) => write!(f, "Unexpected token: {}", msg),
            ParserError::UnexpectedEnd => write!(f, "Unexpected end of input"),
        }
    }
}

impl From<TokenizerError> for ParserError {
    fn from(e: TokenizerError) -> Self {
        ParserError::TokenizerError(e)
    }
}

/// Returns (left_bp, right_bp) for infix operators.
/// `*` and `/` are left-associative at the lowest precedence.
/// `^` / `**` binds tighter and is right-associative.
fn infix_bp(token: &Token) -> Option<(u8, u8)> {
    match token {
        Token::Multiply | Token::Divide => Some((1, 2)),
        Token::Power => Some((3, 4)),
        _ => None,
    }
}

pub(crate) struct PrattParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl PrattParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn expr_bp(&mut self, registry: &UnitRegistry, min_bp: u8) -> Result<Unit, ParserError> {
        let mut lhs = match self.next() {
            Some(Token::Identifier(name)) => match registry.get(&name) {
                Some(u) => Unit::new(HashMap::from([(u, 1.0)])),
                None => return Err(ParserError::UnknownUnit),
            },
            Some(Token::LeftParen) => {
                let inner = self.expr_bp(registry, 0)?;
                match self.next() {
                    Some(Token::RightParen) => {}
                    t => {
                        return Err(ParserError::UnexpectedToken(format!(
                            "expected ')', got {:?}",
                            t
                        )));
                    }
                }
                inner
            }
            t => {
                return Err(ParserError::UnexpectedToken(format!(
                    "expected unit name or '(', got {:?}",
                    t
                )));
            }
        };

        loop {
            let op = match self.peek() {
                None | Some(Token::RightParen) => break,
                Some(t) => t.clone(),
            };

            let (l_bp, r_bp) = match infix_bp(&op) {
                Some(bp) => bp,
                None => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.next(); // consume operator

            match op {
                Token::Power => {
                    let exponent = self.parse_exponent_value()?;
                    lhs = lhs.pow(exponent);
                }
                Token::Multiply => {
                    let rhs = self.expr_bp(registry, r_bp)?;
                    lhs = lhs * rhs;
                }
                Token::Divide => {
                    let rhs = self.expr_bp(registry, r_bp)?;
                    lhs = lhs / rhs;
                }
                _ => unreachable!(),
            }
        }

        Ok(lhs)
    }

    /// Parse the value after a power operator.
    /// Accepts a plain number or a parenthesized fraction like `(2/3)`.
    fn parse_exponent_value(&mut self) -> Result<f64, ParserError> {
        match self.next() {
            Some(Token::Number(n)) => Ok(n),
            Some(Token::LeftParen) => {
                let numerator = self.expect_number()?;
                if self.peek() == Some(&Token::Divide) {
                    self.next();
                    let denominator = self.expect_number()?;
                    match self.next() {
                        Some(Token::RightParen) => Ok(numerator / denominator),
                        t => Err(ParserError::UnexpectedToken(format!(
                            "expected ')', got {:?}",
                            t
                        ))),
                    }
                } else {
                    match self.next() {
                        Some(Token::RightParen) => Ok(numerator),
                        t => Err(ParserError::UnexpectedToken(format!(
                            "expected ')' or '/', got {:?}",
                            t
                        ))),
                    }
                }
            }
            t => Err(ParserError::ExpectedNumber(format!(
                "expected number or '(' after '^', got {:?}",
                t
            ))),
        }
    }

    fn expect_number(&mut self) -> Result<f64, ParserError> {
        match self.next() {
            Some(Token::Number(n)) => Ok(n),
            t => Err(ParserError::ExpectedNumber(format!(
                "expected number, got {:?}",
                t
            ))),
        }
    }
}

pub fn parse(registry: &UnitRegistry, input: &str) -> Result<Unit, ParserError> {
    let tokens = tokenize(input)?;
    let mut parser = PrattParser::new(tokens);
    parser.expr_bp(registry, 0)
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
