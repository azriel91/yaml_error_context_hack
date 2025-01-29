//! Returns the `serde_yaml` error location and message to pass to `miette`.
//!
//! # Examples
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use yaml_error_context_hack::{ErrorAndContext, SourceOffset};
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
//! struct Config {
//!     outer: Outer,
//! }
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
//! struct Outer {
//!     field_1: u32,
//!     field_2: u32,
//! }
//!
//! fn main() {
//!     let file_contents = r#"---
//! outer:
//!   field_1: 123
//! ## ^
//! ## '--- field_2 missing the first character of the first type that has `#[serde(flatten)]`.
//! "#;
//!     let error = serde_yaml::from_str::<Config>(&file_contents).unwrap_err();
//!     let error_and_context = ErrorAndContext::new(file_contents, &error);
//!
//!     let loc_line = 3;
//!     let loc_col = 3; // index 2 is column 3
//!
//!     assert_eq!(
//!         "outer: missing field `field_2` at line 3 column 3",
//!         error.to_string()
//!     );
//!     assert_eq!(
//!         ErrorAndContext {
//!             error_span: Some(SourceOffset::from_location(
//!                 file_contents,
//!                 loc_line,
//!                 loc_col
//!             )),
//!             error_message: "outer: missing field `field_2`".to_string(),
//!             context_span: None,
//!         },
//!         error_and_context,
//!         "{error}"
//!     );
//! }
//! ```

// Re-exports
pub use miette::{self, SourceOffset};

pub use crate::error_and_context::ErrorAndContext;

mod error_and_context;
