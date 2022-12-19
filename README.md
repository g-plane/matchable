# matchable

![Crates.io](https://img.shields.io/crates/v/matchable?style=flat-square)
![docs.rs](https://img.shields.io/docsrs/matchable?style=flat-square)

`matchable` provides a convenient enum for checking if a piece of text
is matching a string or a regex.

The common usage of this crate is used as configuration value type
with `serde` feature enabled (disabled by default),
then user can pass string and/or regex in just one enum.
Later, you can use that enum to check if a piece of text is matching
the string/regex or not.

## Example

```rust
use matchable::Matchable;

assert!(Matchable::Str("Abc".into()).is_match("Abc"));
assert!(!Matchable::Str("Abc".into()).is_match("abc"));
assert!(Matchable::Regex(regex::Regex::new("abc.").unwrap()).is_match("abcd"));
```

## License

MIT License

Copyright (c) 2022-present Pig Fang
