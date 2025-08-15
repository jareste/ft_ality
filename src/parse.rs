use std::collections::{BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token(String);

impl Token {
    pub fn new<S: Into<String>>(s: S) -> Self {
        let t = s.into().trim().to_string();
        Self(t)
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub sequence: Vec<Token>,
    pub move_name: String,
}

#[derive(Debug, Clone)]
pub struct Grammar {
    pub rules: Vec<Rule>,
    pub alphabet: Vec<Token>,
}

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    EmptySequence { line_no: usize },
    MissingArrow { line_no: usize },
    EmptyMoveName { line_no: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Io(e) => write!(f, "I/O error: {e}"),
            ParseError::EmptySequence { line_no } =>
                write!(f, "line {line_no}: empty sequence before '->'"),
            ParseError::MissingArrow { line_no } =>
                write!(f, "line {line_no}: expected '->' in rule"),
            ParseError::EmptyMoveName { line_no } =>
                write!(f, "line {line_no}: empty move name after '->'"),
        }
    }
}

impl std::error::Error for ParseError {}

/*
 * line := <sequence> "->" <move_name>
 * sequence := token ("," token)*
 * token := non-empty string without comma/newline (trimmed)
 * comments: lines starting with '#' (ignored)
 * blank lines ignored
 */
pub fn parse_gmr(input: &str) -> Result<Grammar, ParseError> {
    let rules: Vec<Rule> = input
        .lines()
        .enumerate()
        .try_fold(Vec::new(), |mut acc, (idx, raw_line)| {
            let line_no = idx + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                return Ok(acc);
            }

            let (lhs, rhs) = line
                .split_once("->")
                .map(|(l, r)| (l.trim(), r.trim()))
                .ok_or(ParseError::MissingArrow { line_no })?;

            if rhs.is_empty() {
                return Err(ParseError::EmptyMoveName { line_no });
            }

            let sequence: Vec<Token> = lhs
                .split(',')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .map(Token::new)
                .collect();

            if sequence.is_empty() {
                return Err(ParseError::EmptySequence { line_no });
            }

            acc.push(Rule { sequence, move_name: rhs.to_string() });
            Ok(acc)
        })?;

    let alphabet: Vec<Token> = rules
        .iter()
        .flat_map(|r| r.sequence.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(Grammar { rules, alphabet })
}

pub fn parse_gmr_file(path: &str) -> Result<Grammar, ParseError> {
    std::fs::read_to_string(path)
        .map_err(ParseError::Io)
        .and_then(|s| parse_gmr(&s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_parse_test() {
        let g = r#"
            # comment
            [BP] -> Claw Slam (Freddy Krueger)
            [BP], [FP] -> Saibot Blast (Noob Saibot)
            Down, Right, [FP] -> Fireball
        "#;

        let grammar = parse_gmr(g).unwrap();
        assert_eq!(grammar.rules.len(), 3);
        let tokens: Vec<&str> = grammar.alphabet.iter().map(|t| t.as_str()).collect();
        assert!(tokens.contains(&"[BP]"));
        assert!(tokens.contains(&"[FP]"));
        assert!(tokens.contains(&"Down"));
        assert!(tokens.contains(&"Right"));
    }

    #[test]
    fn missing_arrow_line12() {
        let grammar = parse_gmr_file("grammar/errors/missing_arrow.gmr");

        match grammar {
            Ok(_) => panic!("Expected error for missing arrow"),
            Err(ParseError::MissingArrow { line_no }) => assert_eq!(line_no, 12), /* The incorrect line is the 12th */
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn empty_sequence_line3() {
        let grammar = parse_gmr_file("grammar/errors/empty_sequence.gmr");
        match grammar {
            Ok(_) => panic!("Expected error for empty sequence"),
            Err(ParseError::EmptySequence { line_no }) => assert_eq!(line_no, 3), /* The incorrect line is the 3rd */
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn invalid_file() {
        let grammar = parse_gmr_file("non_existent.gmr");
        assert!(grammar.is_err(), "Expected error for non-existent file");
    }

    #[test]
    fn empty_file() {
        let grammar = parse_gmr_file("grammar/errors/empty.gmr");
        assert!(grammar.is_ok(), "Expected empty grammar to parse without error");
        let g = grammar.unwrap();
        assert!(g.rules.is_empty(), "Expected no rules in empty grammar");
        assert!(g.alphabet.is_empty(), "Expected no tokens in empty grammar");
    }

}
