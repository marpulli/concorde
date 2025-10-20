use crate::unit::{DefinedUnit, Unit};
use std::{any::Any, collections::HashMap, fmt::Error, sync::Arc};

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
    Slash,              // "/" (when used in fractions like "1/2")
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
pub struct TokenizerError {}

fn tokenize(input: &str) -> Result<Vec<Token>, ParserError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            // skip whitespace
            ' ' | '\t' | '\n' => {
                chars.next();
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
                tokens.push(Token::Number(num.parse().unwrap()))
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
                tokens.push(Token::Power)
            }
            '(' => {
                chars.next();
                tokens.push(Token::LeftParen)
            }
            ')' => {
                chars.next();
                tokens.push(Token::RightParen)
            }
            _ => return Err(ParserError::TokenizerError),
        }
    }
    Ok(tokens)
}

#[derive(Debug)]
pub enum ParserError {
    TokenizerError,
    ExpectedNumber,
    UnknownUnit,
    UnexpectedToken,
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
        match self.tokens.get(self.position) {
            Some(Token::Number(n)) => {
                let num = *n;
                self.position += 1;

                if let Some(Token::Divide) = self.tokens.get(self.position) {
                    self.position += 1;
                    match self.tokens.get(self.position) {
                        Some(Token::Number(d)) => {
                            let denom = *d;
                            Ok(num / denom)
                        }
                        _ => Err(ParserError::ExpectedNumber),
                    }
                } else {
                    Ok(num)
                }
            }
            _ => Err(ParserError::ExpectedNumber),
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
                    _ => Err(ParserError::UnexpectedToken),
                }
            }
            _ => Err(ParserError::UnexpectedToken),
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
        let mut factor = self.parse_factor(registry)?;
        match self.peek() {
            Some(Token::Power) => {
                let exponent = self.parse_exponent()?;
                Ok(factor.pow(exponent))
            }
            Some(Token::RightParen) => Ok(factor),
            _ => Err(ParserError::UnexpectedToken),
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;

    fn create_unit_registry() -> UnitRegistry {
        let kg_unit = DefinedUnit::new(
            "kg".to_string(),
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Mass dimension
            1.0,
            vec![],
        );
        return UnitRegistry::new(Vec::from_iter([kg_unit]));
    }

    #[test]
    fn test_parse_simple_unit() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg**2".to_string());
        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        assert_eq!(unit.units.get(&kg_def), Some(&2.0));
    }
}
