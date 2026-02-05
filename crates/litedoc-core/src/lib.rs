//! # LiteDoc Core
//!
//! A deterministic, AI-token-efficient document format parser.
//!
//! LiteDoc provides an alternative to Markdown with explicit fenced block syntax,
//! making it ideal for AI consumption due to reduced ambiguity and lower token counts.
//!
//! ## Quick Start
//!
//! ```rust
//! use litedoc_core::{Parser, Profile};
//!
//! let input = "# Hello World\n\nThis is a **paragraph**.";
//! let mut parser = Parser::new(Profile::Litedoc);
//! let doc = parser.parse(input).unwrap();
//!
//! println!("Parsed {} blocks", doc.blocks.len());
//! ```
//!
//! ## Error Recovery
//!
//! The parser supports graceful error recovery:
//!
//! ```rust
//! use litedoc_core::{Parser, Profile};
//!
//! let input = "::unknown\nsome content\n::";
//! let mut parser = Parser::new(Profile::Litedoc);
//! let result = parser.parse_with_recovery(input);
//!
//! // Document is still parsed, errors are collected
//! println!("Blocks: {}, Errors: {}", result.document.blocks.len(), result.errors.len());
//! ```
//!
//! ## Profiles
//!
//! - `Profile::Litedoc` - Full native syntax with explicit fencing
//! - `Profile::Md` - CommonMark + GFM subset
//! - `Profile::MdStrict` - CommonMark core only

pub mod ast;
pub mod error;
pub mod inline;
pub mod lexer;
pub mod parser;
pub mod span;

pub use ast::{Block, Document, Inline, Profile};
pub use error::{ParseError, ParseErrorKind, ParseErrors};
pub use parser::{ParseResult, Parser};
