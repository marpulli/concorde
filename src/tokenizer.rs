use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Token {
    Identifier(String), // Unit names like "kg", "meter"
    Number(f64),        // For exponents: 2, 3.5
    Multiply,           // "*"
    Divide,             // "/"
    Power,              // "**" or "^"
    LeftParen,          // "("
    RightParen,         // ")"
}

#[derive(Debug)]
pub enum TokenizerError {
    UnexpectedCharacter,
}

impl std::fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenizerError::UnexpectedCharacter => write!(f, "Tokenizer error: unexpected character"),
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

pub(crate) fn tokenize(input: &str) -> Result<Vec<Token>, TokenizerError> {
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
            _ => return Err(TokenizerError::UnexpectedCharacter),
        }
    }
    Ok(tokens)
}
