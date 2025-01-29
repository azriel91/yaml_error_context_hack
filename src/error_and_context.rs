use miette::SourceOffset;

/// The [`SourceOffset`]s of the error and the surrounding context based on the
/// error display string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorAndContext {
    /// The [`SourceOffset`] of the error.
    pub error_span: Option<SourceOffset>,
    /// The error message with the source offsets truncated.
    ///
    /// This is the text before the `" at "` text, because the source offsets in
    /// the error message can be noise, e.g.
    ///
    /// ```text
    /// "at line 2 column 11 at line 2 column 11 at line 2 column 3"
    /// ```
    pub error_message: String,
    /// The [`SourceOffset`] of the surrounding context.
    pub context_span: Option<SourceOffset>,
}

impl ErrorAndContext {
    /// Returns the error location and message to pass to miette.
    ///
    /// The `location()` reported in the error is incorrect, due to
    /// [serde-yaml#153](https://github.com/dtolnay/serde-yaml/issues/153).
    ///
    /// This does a best-effort to find the actual error source offsets from the
    /// `Display` string of the error.
    pub fn new(file_contents: &str, error: &serde_yaml::Error) -> Self {
        let error_string = format!("{error}");
        let error_location_line_index_column = error.location().map(|error_location| {
            (
                error_location.index(),
                error_location.line(),
                error_location.column(),
            )
        });
        let (error_span, context_span) = match error_location_line_index_column {
            // The `error_location` is not the true location. Extract it from the `Display` string.
            //
            // See:
            //
            // * <https://github.com/dtolnay/serde-yaml/blob/0.9.14/src/libyaml/error.rs#L65-L84>
            // * <https://github.com/dtolnay/serde-yaml/blob/0.9.14/src/libyaml/error.rs#L141>
            //
            // Example error strings (truncated the beginning):
            //
            // ```text
            // missing field `path` at line 2 column 12 at line 2 column 3
            // unknown variant `~`, expected one of `a`, `b` at line 2 column 11 at line 2 column 11 at line 2 column 3
            // ```
            Some((0, 1, 1)) => {
                // TODO: This may also be "at position 123", but we don't support that yet.
                let mut line_column_pairs =
                    error_string.rsplit(" at line ").filter_map(|line_column| {
                        let mut line_column_split = line_column.split(" column ");
                        let line = line_column_split
                            .next()
                            .map(str::parse::<usize>)
                            .and_then(Result::ok);
                        let column = line_column_split
                            .next()
                            .map(str::parse::<usize>)
                            .and_then(Result::ok);

                        if let (Some(line), Some(column)) = (line, column) {
                            Some((line, column))
                        } else {
                            None
                        }
                    });

                let last_mark = line_column_pairs
                    .next()
                    .map(|(line, column)| SourceOffset::from_location(file_contents, line, column));
                let second_to_last_mark = line_column_pairs
                    .next()
                    .map(|(line, column)| SourceOffset::from_location(file_contents, line, column));

                match (second_to_last_mark, last_mark) {
                    (error_span @ Some(_), context_span @ Some(_)) => (error_span, context_span),
                    (None, error_span @ Some(_)) => (error_span, None),
                    (Some(_), None) | (None, None) => (None, None),
                }
            }
            Some((_, line, column)) => (
                Some(SourceOffset::from_location(file_contents, line, column)),
                None,
            ),
            None => (None, None),
        };

        let error_message = error_string
            .split(" at ")
            .next()
            .map(str::to_string)
            .unwrap_or(error_string);

        ErrorAndContext {
            error_span,
            error_message,
            context_span,
        }
    }
}

#[cfg(test)]
mod tests {
    use miette::SourceOffset;
    use serde::{Deserialize, Serialize};

    use super::ErrorAndContext;

    #[test]
    fn returns_source_offsets_for_missing_field() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Config {
            outer: Outer,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Outer {
            field_1: u32,
            field_2: u32,
        }

        let file_contents = r#"---
outer:
  field_1: 123
# ^
# '--- field_2 missing the first character of the first type that has `#[serde(flatten)]`.
"#;
        let error = serde_yaml::from_str::<Config>(&file_contents).unwrap_err();
        let error_and_context = ErrorAndContext::new(file_contents, &error);

        let loc_line = 3;
        let loc_col = 3; // index 2 is column 3

        assert_eq!(
            "outer: missing field `field_2` at line 3 column 3",
            error.to_string()
        );
        assert_eq!(
            ErrorAndContext {
                error_span: Some(SourceOffset::from_location(
                    file_contents,
                    loc_line,
                    loc_col
                )),
                error_message: "outer: missing field `field_2`".to_string(),
                context_span: None,
            },
            error_and_context,
            "{error}"
        );
    }

    #[test]
    fn returns_source_offsets_for_missing_field_for_flattened_struct() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Config {
            outer: Outer,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Outer {
            #[serde(flatten)]
            inner: Inner,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Inner {
            field_1: u32,
            field_2: u32,
        }

        let file_contents = r#"---
outer:
  # inner
  field_1: 123
# ^
# '-- field_2 missing on the first character of the wrapping first type that has `#[serde(flatten)]`.
"#;
        let error = serde_yaml::from_str::<Config>(&file_contents).unwrap_err();
        let error_and_context = ErrorAndContext::new(file_contents, &error);

        let loc_line = 4;
        let loc_col = 3; // index 2 is column 3

        assert_eq!(
            "outer: missing field `field_2` at line 4 column 3",
            error.to_string()
        );
        assert_eq!(
            ErrorAndContext {
                error_span: Some(SourceOffset::from_location(
                    file_contents,
                    loc_line,
                    loc_col
                )),
                error_message: "outer: missing field `field_2`".to_string(),
                context_span: None,
            },
            error_and_context,
            "{error}"
        );
    }

    #[test]
    fn returns_source_offsets_for_missing_field_for_nested_flattened_struct() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Config {
            outer: Outer,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Outer {
            #[serde(flatten)]
            inner: Inner,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Inner {
            #[serde(flatten)]
            inner_inner: InnerInner,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct InnerInner {
            field_1: u32,
            field_2: u32,
        }

        let file_contents = r#"---
outer:
  # inner, inner_inner
  field_1: 123
# ^
# '-- field_2 missing on the first character of the first type that has `#[serde(flatten)]`.
"#;
        let error = serde_yaml::from_str::<Config>(&file_contents).unwrap_err();
        let error_and_context = ErrorAndContext::new(file_contents, &error);

        let loc_line = 4;
        let loc_col = 3; // index 2 is column 3

        assert_eq!(
            "outer: missing field `field_2` at line 4 column 3",
            error.to_string()
        );
        assert_eq!(
            ErrorAndContext {
                error_span: Some(SourceOffset::from_location(
                    file_contents,
                    loc_line,
                    loc_col
                )),
                error_message: "outer: missing field `field_2`".to_string(),
                context_span: None,
            },
            error_and_context,
            "{error}"
        );
    }

    #[test]
    fn returns_source_offsets_for_missing_field_for_nested_flattened_struct_2() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Config {
            outer: Outer,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Outer {
            #[serde(flatten)]
            inner: Inner,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Inner {
            inner_outer: InnerOuter,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct InnerOuter {
            #[serde(flatten)]
            inner_inner: InnerInner,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct InnerInner {
            field_1: u32,
            field_2: u32,
        }

        let file_contents = r#"---
outer:
  # inner
  inner_outer:
    # inner_inner
    field_1: 123
# ^
# '-- field_2 is always marked as missing on the first character of the
#     first wrapping type. Ideally it would point to `field_1`'s position (line 6 col 5).
"#;
        let error = serde_yaml::from_str::<Config>(&file_contents).unwrap_err();
        let error_and_context = ErrorAndContext::new(file_contents, &error);

        let loc_line = 4;
        let loc_col = 3; // index 4 is column 3

        assert_eq!(
            "outer: missing field `field_2` at line 4 column 3",
            error.to_string()
        );
        assert_eq!(
            ErrorAndContext {
                error_span: Some(SourceOffset::from_location(
                    file_contents,
                    loc_line,
                    loc_col
                )),
                error_message: "outer: missing field `field_2`".to_string(),
                context_span: None,
            },
            error_and_context,
            "{error}"
        );
    }

    #[test]
    fn returns_source_offsets_for_null_variant() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Config {
            outer: Outer,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        struct Outer {
            inner: Inner,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
        enum Inner {
            One { value: u32 },
            Two { value: u32 },
        }

        let file_contents = r#"---
outer:
  inner: ~ # null variant
#        ^
#        '-- source offset is here.
"#;
        let error = serde_yaml::from_str::<Config>(&file_contents).unwrap_err();
        let error_and_context = ErrorAndContext::new(file_contents, &error);

        let loc_line = 3;
        let loc_col = 10; // index 9 is column 10

        assert_eq!(
            "outer.inner: unknown variant `~`, expected `One` or `Two` at line 3 column 10",
            error.to_string()
        );
        assert_eq!(
            ErrorAndContext {
                error_span: Some(SourceOffset::from_location(
                    file_contents,
                    loc_line,
                    loc_col
                )),
                error_message: "outer.inner: unknown variant `~`, expected `One` or `Two`"
                    .to_string(),
                context_span: None,
            },
            error_and_context,
            "{error}"
        );
    }
}
