# 📝 yaml_error_context_hack

[![Crates.io](https://img.shields.io/crates/v/yaml_error_context_hack.svg)](https://crates.io/crates/yaml_error_context_hack)
[![docs.rs](https://img.shields.io/docsrs/yaml_error_context_hack)](https://docs.rs/yaml_error_context_hack)
[![CI](https://github.com/azriel91/yaml_error_context_hack/workflows/CI/badge.svg)](https://github.com/azriel91/yaml_error_context_hack/actions/workflows/ci.yml)
[![Coverage Status](https://codecov.io/gh/azriel91/yaml_error_context_hack/branch/main/graph/badge.svg)](https://codecov.io/gh/azriel91/yaml_error_context_hack)

Returns the `serde_yaml` error location and message to pass to `miette`.

The `location()` reported in the error is incorrect, due to [serde-yaml#153](https://github.com/dtolnay/serde-yaml/issues/153).

This does a best-effort to find the actual error source offsets from the `Display` string of the error.


# Usage

Add the following to `Cargo.toml`

```toml
yaml_error_context_hack = "0.1.0"
```

In code:

```rust
use serde::{Deserialize, Serialize};
use yaml_error_context_hack::{ErrorAndContext, SourceOffset};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct Config {
    outer: Outer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct Outer {
    field_1: u32,
    field_2: u32,
}

fn main() {
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
```


## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE] or <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT] or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

[LICENSE-APACHE]: LICENSE-APACHE
[LICENSE-MIT]: LICENSE-MIT
