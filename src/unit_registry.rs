use crate::unit::{DefinedUnit, Unit};
use std::{any::Any, collections::HashMap, fmt::Error, iter::Peekable, str::Chars, sync::Arc};

// Define token types
#[derive(Debug, PartialEq, Clone)]
enum Token {
    Identifier(String), // Unit names like "kg", "meter"
    Number(f64),        // For exponents: 2, 3.5
    Multiply,           // "*"
    Divide,             // "/"
    Power,              // "**" or "^"
    LeftParen,          // "("
    RightParen,         // ")"
}

// Tokenizer state
struct Tokenizer<'a> {
    input: &'a str,
    position: usize,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

pub struct UnitRegistry {
    defined_units: HashMap<String, DefinedUnit>,
    aliases: HashMap<String, String>,
}

impl UnitRegistry {
    pub fn new(defined_units: Vec<DefinedUnit>) -> UnitRegistry {
        UnitRegistry {
            defined_units: defined_units
                .iter()
                .map(|f| (f.name.clone(), f.clone()))
                .collect::<HashMap<String, DefinedUnit>>(),
            aliases: HashMap::new(),
        }
    }
    pub fn parse_string(&self, string: String) -> Result<Unit, ParserError> {
        let tokens = tokenize(&string)?;
        let mut parser = Parser::new(tokens);
        parser.parse_expression(&self)
    }

    pub fn get(&self, string: &String) -> Option<Arc<DefinedUnit>> {
        let unit = self.defined_units.get(string);
        match unit {
            Some(u) => Some(Arc::new(u.clone())),
            None => None,
        }
    }
}

fn skip_whitespace(chars: &mut Peekable<Chars>) {
    while let Some(&next_ch) = chars.peek() {
        if next_ch.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParserError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            // Whitespace could be just formatting or could be implicit multiplication of units
            ' ' | '\t' | '\n' => {
                chars.next();

                // Check if we should insert implicit multiplication
                // Look at the last token to see if it was a unit or closing paren
                if let Some(last_token) = tokens.last() {
                    match last_token {
                        Token::Identifier(_) | Token::RightParen => {
                            skip_whitespace(&mut chars);
                            let next_ch = chars.peek();
                            if let Some(&ch) = next_ch {
                                if ch.is_alphabetic() || ch == '(' {
                                    tokens.push(Token::Multiply);
                                }
                            } else {
                                break;
                            }
                        }
                        _ => {
                            skip_whitespace(&mut chars);
                        }
                    }
                }
            }
            'a'..='z' | 'A'..='Z' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Identifier(ident));
            }

            '0'..='9' => {
                let mut num = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_numeric() || c == '.' {
                        num.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num.parse().unwrap()));
            }
            '*' => {
                chars.next();
                if chars.peek() == Some(&'*') {
                    chars.next();
                    tokens.push(Token::Power);
                } else {
                    tokens.push(Token::Multiply)
                }
            }
            '/' => {
                chars.next();
                tokens.push(Token::Divide);
            }
            '^' => {
                chars.next();
                tokens.push(Token::Power);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LeftParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RightParen);
            }
            _ => return Err(ParserError::TokenizerError),
        }
    }
    Ok(tokens)
}

#[derive(Debug)]
pub enum ParserError {
    TokenizerError,
    ExpectedNumber(String),
    UnknownUnit,
    UnexpectedToken(String),
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}
impl Parser {
    fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens: tokens,
            position: 0,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) {
        self.position += 1
    }

    fn parse_exponent(&mut self) -> Result<f64, ParserError> {
        match self.peek() {
            Some(Token::LeftParen) => {
                // Parse parenthesized expression like (2/3)
                self.advance();
                self.parse_exponent_value()
            }
            Some(Token::Number(n)) => {
                let num = *n;
                self.advance();
                Ok(num)
            }
            other => Err(ParserError::ExpectedNumber(format!(
                "expected number or parenthesis, got {:?}",
                other
            ))),
        }
    }

    fn parse_exponent_value(&mut self) -> Result<f64, ParserError> {
        // Parse the numerator
        match self.peek() {
            Some(Token::Number(n)) => {
                let num = *n;
                self.advance();

                // Check if there's a division for a fraction
                if let Some(Token::Divide) = self.peek() {
                    self.advance();
                    match self.peek() {
                        Some(Token::Number(d)) => {
                            let denom = *d;
                            self.advance();

                            // Expect closing parenthesis
                            match self.peek() {
                                Some(Token::RightParen) => {
                                    self.advance();
                                    Ok(num / denom)
                                }
                                other => Err(ParserError::UnexpectedToken(format!(
                                    "expected right parenthesis, got {:?}",
                                    other
                                ))),
                            }
                        }
                        other => Err(ParserError::ExpectedNumber(format!(
                            "expected number after division, got {:?}",
                            other
                        ))),
                    }
                } else {
                    // Just a number in parentheses like (2)
                    match self.peek() {
                        Some(Token::RightParen) => {
                            self.advance();
                            Ok(num)
                        }
                        other => Err(ParserError::UnexpectedToken(format!(
                            "expected right parenthesis, got {:?}",
                            other
                        ))),
                    }
                }
            }
            other => Err(ParserError::ExpectedNumber(format!(
                "expected number in exponent, got {:?}",
                other
            ))),
        }
    }

    fn parse_factor(&mut self, registry: &UnitRegistry) -> Result<Unit, ParserError> {
        match self.peek() {
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();

                let unit = registry.get(&name);
                match unit {
                    Some(u) => Ok(Unit::new(HashMap::from([(u, 1.0)]))),
                    None => Err(ParserError::UnknownUnit),
                }
            }
            Some(Token::LeftParen) => {
                self.advance();
                let result = self.parse_expression(registry)?;
                match self.peek() {
                    Some(Token::RightParen) => {
                        self.advance();
                        Ok(result)
                    }
                    other => Err(ParserError::UnexpectedToken(format!(
                        "Expected right parenthesis, got {:?}",
                        other
                    ))),
                }
            }
            other => Err(ParserError::UnexpectedToken(format!(
                "Expected name or right parenthesis, got {:?}",
                other
            ))),
        }
    }

    fn parse_expression(&mut self, registry: &UnitRegistry) -> Result<Unit, ParserError> {
        // Handles multiplication and division
        let mut result = self.parse_term(registry)?;

        while let Some(token) = self.peek() {
            match token {
                Token::Multiply => {
                    self.advance();
                    let next_factor = self.parse_term(registry)?;
                    result = result * next_factor;
                }
                Token::Divide => {
                    self.advance();
                    let next_factor = self.parse_term(registry)?;
                    result = result / next_factor;
                }
                _ => break,
            }
        }
        Ok(result)
    }

    fn parse_term(&mut self, registry: &UnitRegistry) -> Result<Unit, ParserError> {
        // handles ^ and )
        let factor = self.parse_factor(registry)?;
        if let Some(Token::Power) = self.peek() {
            self.advance();
            let exponent = self.parse_exponent()?;
            return Ok(factor.pow(exponent));
        }
        Ok(factor)
    }
}
#[cfg(test)]
mod test {
    use std::ops::Not;

    use super::*;

    fn create_unit_registry() -> UnitRegistry {
        let kg_unit = DefinedUnit::new(
            "kg".to_string(),
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Mass dimension
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
        return UnitRegistry::new(Vec::from_iter([kg_unit, s, m]));
    }

    #[test]
    fn test_parse_integer_exponent() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg**2".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&2.0));
    }

    #[test]
    fn test_parse_decimal_exponent() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg**2.5".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&2.5));
    }

    #[test]
    fn test_parse_fraction_exponent() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg**(2/5)".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&0.4));
    }

    #[test]
    fn test_parse_unit_multiplication_with_asterisk() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg * s".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        let s_def = unit_registry.get(&"s".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&1.0));
        assert_eq!(unit.components.get(&s_def), Some(&1.0));
        assert_eq!(unit.components.len(), 2)
    }

    #[test]
    fn test_parse_unit_division() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg / s".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        let s_def = unit_registry.get(&"s".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&1.0));
        assert_eq!(unit.components.get(&s_def), Some(&-1.0));
        assert_eq!(unit.components.len(), 2)
    }

    #[test]
    fn test_multiplication_and_division_precedence() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg / s * s".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        let s_def = unit_registry.get(&"s".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&1.0));
        assert!(unit.components.contains_key(&s_def).not());
        assert_eq!(unit.components.len(), 1)
    }

    #[test]
    fn test_brackets() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg / (s * s)".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        let s_def = unit_registry.get(&"s".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&1.0));
        assert_eq!(unit.components.get(&s_def), Some(&-2.0));
    }

    #[test]
    fn test_complex_precedence_mixed_operators() {
        let registry = create_unit_registry();
        let unit = registry.parse_string("kg * m^2 / s^2".to_string()).unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"m".to_string()), Some(2.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(-2.0));
    }

    #[test]
    fn test_chained_exponentiation_with_fractions() {
        let registry = create_unit_registry();
        let unit = registry
            .parse_string("m^(3/2) * kg^(1/3) / s^(2/3)".to_string())
            .unwrap();
        assert_eq!(unit.get_exponent(&"m".to_string()), Some(1.5));
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0 / 3.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(-2.0 / 3.0));
    }

    #[test]
    fn test_same_unit_multiple_times() {
        // m^(3/2) * kg^(1/3) / s^(2/3)
        let registry = create_unit_registry();
        let unit = registry.parse_string("kg * kg * kg".to_string()).unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(3.0));
    }

    #[test]
    fn test_same_unit_cancellation() {
        let registry = create_unit_registry();
        let unit = registry.parse_string("kg / kg".to_string()).unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), None);
    }

    #[test]
    fn test_implicit_multiplication() {
        let registry = create_unit_registry();
        let unit = registry.parse_string("kg s".to_string()).unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(1.0));
    }
}
