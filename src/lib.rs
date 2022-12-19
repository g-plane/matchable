#![no_std]

//! `matchable` provides a convenient enum for checking if a piece of text is
//! matching a string or a regex.
//!
//! ```
//! use matchable::Matchable;
//!
//! assert!(Matchable::Str("Abc".into()).is_match("Abc"));
//! assert!(!Matchable::Str("Abc".into()).is_match("abc"));
//! assert!(Matchable::Regex(regex::Regex::new("abc.").unwrap()).is_match("abcd"));
//! ```
//!
//! More detail about the usage, please refer to doc of [`Matchable`].
//!
//! ## Deserialization
//!
//! One of the advantages of using this crate is deserialize into a [`Matchable`].
//! This is often used as configuration, and allows user to pass a string or a regex as a pattern.
//!
//! Here we use JSON as example:
//!
//! If a string is enclosed with slashes (`/`), with or without optional flags as suffix,
//! this will be deserialized as a regex:
//!
//! ```
//! use matchable::Matchable;
//!
//! let re_digits = serde_json::from_str::<Matchable>(r#""/\\d+/""#).unwrap();
//! assert!(re_digits.is_match("123"));
//!
//! // with regex flags
//! let re_word = serde_json::from_str::<Matchable>(r#""/matchable/i""#).unwrap();
//! assert!(re_word.is_match("Matchable"));
//! ```
//!
//! Otherwise, it will be parsed as a normal string as-is.
//!
//! ```
//! use matchable::Matchable;
//!
//! let re1 = serde_json::from_str::<Matchable>(r#""/ab""#).unwrap();
//! assert!(re1.is_match("/ab"));
//! assert!(!re1.is_match("ab"));
//!
//! let re2 = serde_json::from_str::<Matchable>(r#""ab/i""#).unwrap();
//! assert!(re2.is_match("ab/i"));
//! assert!(!re2.is_match("AB"));
//! ```

extern crate alloc;

#[cfg(feature = "serde")]
use alloc::borrow::ToOwned;
use alloc::string::String;
#[cfg(feature = "serde")]
use core::fmt;
use core::{
    hash::{Hash, Hasher},
    ops::Deref,
};
use regex::Regex;
#[cfg(feature = "serde")]
use regex::RegexBuilder;
#[cfg(feature = "serde")]
use serde::{
    de::{Error, Unexpected, Visitor},
    Deserialize, Deserializer,
};

/// `Matchable` is a wrapper for a plain string or a regex, and it's used to check matching.
///
/// When checking if a text is matching or not,
/// if it's a plain string, it will check whether two strings are equal or not;
/// if it's a regex, it will check whether that text matches the regex or not.
///
/// When deserializing by Serde,
/// if the value starts with a slash `/`, and it ends with a slash `/` with optional regex flags,
/// like `"/abcd/"` or `"/abcd/i"`, it will be deserialized as a regex;
/// otherwise, it will be deserialized as a plain string.
#[derive(Clone, Debug)]
pub enum Matchable {
    Str(String),
    Regex(Regex),
}

impl Matchable {
    /// Check whether a piece of text matches the [`Matchable`] or not.
    ///
    /// ```
    /// use matchable::Matchable;
    ///
    /// assert!(Matchable::Str("Abc".into()).is_match("Abc"));
    /// assert!(!Matchable::Str("Abc".into()).is_match("abc"));
    ///
    /// let re = regex::RegexBuilder::new("Abc")
    ///     .case_insensitive(true)
    ///     .build()
    ///     .unwrap();
    /// assert!(Matchable::Regex(re).is_match("abc"));
    /// ```
    #[inline]
    pub fn is_match(&self, text: impl AsRef<str>) -> bool {
        let text = text.as_ref();
        match self {
            Self::Str(str) => str == text,
            Self::Regex(regex) => regex.is_match(text),
        }
    }

    /// Return the string representation of the [`Matchable`].
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Str(str) => str,
            Self::Regex(regex) => regex.as_str(),
        }
    }
}

impl Default for Matchable {
    fn default() -> Self {
        Self::Str(Default::default())
    }
}

impl Hash for Matchable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Str(str) => str.hash(state),
            Self::Regex(regex) => regex.as_str().hash(state),
        };
    }
}

impl PartialEq for Matchable {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Matchable::Str(a), Matchable::Str(b)) => a == b,
            (Matchable::Regex(a), Matchable::Regex(b)) => a.as_str() == b.as_str(),
            _ => false,
        }
    }
}

impl Eq for Matchable {}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Matchable {
    fn deserialize<D>(deserializer: D) -> Result<Matchable, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(MatchableVisitor)
    }
}

/// Serde visitor for parsing string as the [`Matchable`] type.
#[cfg(feature = "serde")]
struct MatchableVisitor;

#[cfg(feature = "serde")]
impl<'de> Visitor<'de> for MatchableVisitor {
    type Value = Matchable;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a normal string or a string that represents a regex"
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if let Some((regex, flags)) = extract_regex(v) {
            build_regex(regex, flags)
                .map(Matchable::Regex)
                .map_err(|_| E::invalid_value(Unexpected::Str(regex), &"a valid regex"))
        } else {
            Ok(Matchable::Str(v.to_owned()))
        }
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if let Some((regex, flags)) = extract_regex(&v) {
            build_regex(regex, flags)
                .map(Matchable::Regex)
                .map_err(|_| E::invalid_value(Unexpected::Str(regex), &"a valid regex"))
        } else {
            Ok(Matchable::Str(v))
        }
    }
}

/// This `RegexOnly` is just a wrapper of [`Regex`](regex::Regex).
/// Unlike [`Matchable`], this `RegExp` treats the whole string as a regular expression,
/// while [`Matchable`] only treats it as regular expression when it's enclosed by `/`.
#[derive(Clone, Debug)]
pub struct RegexOnly(Regex);

impl Deref for RegexOnly {
    type Target = Regex;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for RegexOnly {
    fn deserialize<D>(deserializer: D) -> Result<RegexOnly, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(RegexOnlyVisitor)
    }
}

/// Serde visitor for parsing string as the [`RegexOnly`](RegexOnly) type.
#[cfg(feature = "serde")]
struct RegexOnlyVisitor;

#[cfg(feature = "serde")]
impl<'de> Visitor<'de> for RegexOnlyVisitor {
    type Value = RegexOnly;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a string that represents a regex")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Regex::new(v)
            .map(RegexOnly)
            .map_err(|_| E::invalid_value(Unexpected::Str(v), &"a valid regex"))
    }
}

#[cfg(feature = "serde")]
fn extract_regex(s: &str) -> Option<(&str, &str)> {
    s.strip_prefix('/').and_then(|s| s.rsplit_once('/'))
}

#[cfg(feature = "serde")]
fn build_regex(regex: &str, flags: &str) -> Result<Regex, regex::Error> {
    let mut builder = RegexBuilder::new(regex);
    builder.case_insensitive(flags.contains('i'));
    builder.multi_line(flags.contains('m'));
    builder.dot_matches_new_line(flags.contains('s'));
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_matchable_is_match() {
        let matchable = Matchable::Str(String::from("abc"));
        assert!(matchable.is_match("abc"));
        assert!(!matchable.is_match("abd"));

        let matchable = Matchable::Regex(Regex::new("\\d+").unwrap());
        assert!(matchable.is_match("123"));
        assert!(!matchable.is_match("abc"));
    }

    #[test]
    fn test_str() {
        let matchable = serde_json::from_str(r#""abc""#).unwrap();
        assert!(matches!(matchable, Matchable::Str(s) if s == "abc"));
    }

    #[test]
    fn test_regex() {
        let matchable = serde_json::from_str(r#""/\\d+/""#).unwrap();
        assert!(matches!(&matchable, Matchable::Regex(regex) if regex.is_match("123")));

        let matchable = serde_json::from_str(r#""/[ab]/i""#).unwrap();
        assert!(matches!(&matchable, Matchable::Regex(regex) if regex.is_match("a")));
        assert!(matches!(&matchable, Matchable::Regex(regex) if regex.is_match("B")));

        let matchable = serde_json::from_str(r#""/./s""#).unwrap();
        assert!(matches!(&matchable, Matchable::Regex(regex) if regex.is_match("a")));
        assert!(matches!(&matchable, Matchable::Regex(regex) if regex.is_match("\n")));

        let error = serde_json::from_str::<Matchable>(r#""/(/""#).unwrap_err();
        assert!(error.to_string().contains("expected a valid regex"));
    }

    #[test]
    fn test_regex_only() {
        let regex = serde_json::from_str::<RegexOnly>(r#""\\d+""#).unwrap();
        assert!(regex.is_match("123"));
        assert!(!regex.is_match("abc"));

        let regex = serde_json::from_str::<RegexOnly>(r#""/[ab]/i""#).unwrap();
        assert!(regex.is_match("/a/i"));
        assert!(regex.is_match("/b/i"));
        assert!(!regex.is_match("A"));
        assert!(!regex.is_match("B"));

        let error = serde_json::from_str::<RegexOnly>(r#""(""#).unwrap_err();
        assert!(error.to_string().contains("expected a valid regex"));
    }
}
