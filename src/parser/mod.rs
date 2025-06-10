// L1 Compiler
//! Parser
// Author: Miles Conn <mconn@andrew.cmu.edu>

// We rely on [lalrpop](https://github.com/lalrpop/lalrpop) for parsing.
// Lalrpop generates a LR(1) paraser the grammar can be found in c0.lalrpop
// and the generated code in c0.rs

//lalrpop_mod!(pub meerkat, "/parser/meerkat.rs");

pub mod meerkat;
pub mod lex;

pub mod parser {
  use logos::{Logos, Span};
  use std::fs;

  use super::meerkat;
  use super::lex::Token;
  use crate::ast::Prog;

  fn parse_string(input: String) -> Result<Prog, String> {
    // You'll need lexer_with_extras later trust me :)
    let lex_stream = Token::lexer_with_extras(&input, ())
      .spanned()
      .map(|(t, y): (Token, Span)| (y.start, t, y.end));

    meerkat::ProgParser::new()
      .parse(lex_stream)
      .map_err(|e| format!("Couldn't parse file. Failed with message {:?}", e))
  }
  pub fn parse(file_name: String) -> Result<Prog, String> {
    let str_file = fs::read_to_string(file_name).expect("Couldn't read file");

    parse_string(str_file)
  }
}
