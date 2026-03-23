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
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::TokenizerError(e) => write!(f, "{}", e),
            ParserError::ExpectedNumber(msg) => write!(f, "Expected number: {}", msg),
            ParserError::UnknownUnit => write!(f, "Unknown unit"),
            ParserError::UnexpectedToken(msg) => write!(f, "Unexpected token: {}", msg),
        }
    }
}

impl From<TokenizerError> for ParserError {
    fn from(e: TokenizerError) -> Self {
        ParserError::TokenizerError(e)
    }
}

pub(crate) struct Parser {
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

pub fn parse(registry: &UnitRegistry, input: &str) -> Result<Unit, ParserError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_expression(registry)
}
