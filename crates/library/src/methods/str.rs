use heck::ToSnakeCase;
use macros::native;
use vm::{RtErr, api::Api};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    let nid = api.add_method(len);
    api.mark_intrinsic(nid, api::Intrinsic::Len);
    let nid = api.add_method(contains);
    api.mark_intrinsic(nid, api::Intrinsic::In);
    api.add_method(starts_with);
    api.add_method(ends_with);
    api.add_method(to_upper);
    api.add_method(to_lower);
    api.add_method(trim);
    api.add_method(to_snake);
    api.add_method(split);
    api.add_method(find);
    api.add_method(find_all);
    api.add_method(lines);
    api.add_method(capitalize);
    api.add_method(to_int);
    api.add_method(ord);
    api.add_method(is_empty);
    api.add_method(replace);
    api.add_method(repeat);
}

/// Returns the number of characters. This counts Unicode characters, not bytes: `"héllo".len()` is
/// `5`.
///
/// ```mimas
/// let n = "hello".len(); // 5
/// ```
#[native]
fn len(s: &str) -> usize {
    s.chars().count()
}

/// Returns whether `needle` appears anywhere in the string. This is the same check as the
/// [`in` operator](../reference/collections/in-expressions.md).
///
/// ```mimas
/// let a = "hello".contains("ell"); // true
/// let b = "ell" in "hello";        // true
/// ```
#[native]
fn contains(s: &str, needle: &str) -> bool {
    s.contains(needle)
}

/// Returns whether the string begins with `prefix`.
///
/// ```mimas
/// let a = "mimas".starts_with("mi"); // true
/// ```
#[native]
fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Returns whether the string ends with `suffix`.
///
/// ```mimas
/// let a = "readme.md".ends_with(".md"); // true
/// ```
#[native]
fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

/// Returns the string with every letter in uppercase. This follows Unicode's rules, which can turn
/// one character into several.
///
/// ```mimas
/// let a = "Hello".to_upper();  // "HELLO"
/// let b = "straße".to_upper(); // "STRASSE"
/// ```
#[native]
fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

/// Returns the string with every letter in lowercase, following Unicode's rules.
///
/// ```mimas
/// let a = "Hello".to_lower(); // "hello"
/// ```
#[native]
fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

/// Returns the string without the whitespace at its start and end. Whitespace inside the string
/// stays.
///
/// ```mimas
/// let a = "  two words \n".trim(); // "two words"
/// ```
#[native]
fn trim(s: &str) -> String {
    s.trim().to_string()
}

/// Returns the string in snake case: lowercase words joined by underscores. Spaces, punctuation,
/// and changes from lowercase to uppercase all mark where one word ends and the next begins.
///
/// ```mimas
/// let a = "PlayerHealth".to_snake(); // "player_health"
/// let b = "max hp".to_snake();       // "max_hp"
/// let c = "HTTPServer".to_snake();   // "http_server"
/// ```
#[native]
fn to_snake(s: &str) -> String {
    s.to_snake_case()
}

/// Returns the pieces of the string between each occurrence of `separator`. Two separators in a
/// row leave an empty string between them.
///
/// ```mimas
/// let a = "a,b,c".split(",");     // ["a", "b", "c"]
/// let b = "a,,b".split(",");      // ["a", "", "b"]
/// let c = "no commas".split(","); // ["no commas"]
/// ```
#[native]
fn split(s: &str, separator: &str) -> Vec<String> {
    s.split(&separator).map(|v| v.to_string()).collect()
}

/// Returns the string's lines, without their line endings. A line can end in `\n` or `\r\n`, and
/// a line ending at the very end of the string doesn't add an empty line.
///
/// ```mimas
/// let a = "one\ntwo\r\nthree\n".lines(); // ["one", "two", "three"]
/// ```
#[native]
fn lines(s: &str) -> Vec<String> {
    s.lines().map(From::from).collect()
}

// todo, the handling of option vs raisable below is bad

/// Searches the string for the first match of the regular expression `pattern`. Returns the whole
/// match followed by the text of each capture group, or `null` if nothing matched or `pattern`
/// isn't a valid regular expression.
///
/// A capture group that didn't take part in the match is left out of the array, which moves the
/// groups after it down a position.
///
/// The pattern syntax is that of the Rust [`regex`](https://docs.rs/regex/latest/regex/#syntax)
/// crate. Inside a mimas string, each backslash in the pattern is written twice (`"\\d+"` matches
/// a run of digits).
///
/// ```mimas
/// let date = "2024-06-01".find("(\\d+)-(\\d+)-(\\d+)");
/// // date is ["2024-06-01", "2024", "06", "01"]
/// let none = "abc".find("\\d"); // null
/// ```
#[native]
fn find(s: &str, pattern: &str) -> Option<Vec<String>> {
    let re = regex::Regex::new(pattern).ok()?;
    let caps = re.captures(s)?;
    Some(
        caps.iter()
            .flatten()
            .map(|m| m.as_str().to_string())
            .collect(),
    )
}

/// Returns every match of the regular expression `pattern`, each in the form [`find`](#find)
/// returns. Matches don't overlap. The array is empty when nothing matched, and the result is
/// `null` only when `pattern` isn't a valid regular expression.
///
/// ```mimas
/// let pairs = "x=1, y=22".find_all("(\\w)=(\\d+)");
/// // pairs is [["x=1", "x", "1"], ["y=22", "y", "22"]]
/// ```
#[native]
fn find_all(s: &str, pattern: &str) -> Option<Vec<Vec<String>>> {
    let re = regex::Regex::new(pattern).ok()?;
    Some(
        re.captures_iter(s)
            .map(|caps| {
                caps.iter()
                    .flatten()
                    .map(|m| m.as_str().to_string())
                    .collect::<Vec<String>>()
            })
            .collect(),
    )
}

/// Returns the string with its first character in uppercase. The rest of the string is
/// unchanged.
///
/// ```mimas
/// let a = "élan".capitalize();  // "Élan"
/// let b = "hELLO".capitalize(); // "HELLO"
/// ```
#[native]
fn capitalize(s: &str) -> String {
    s.chars()
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + &s[c.len_utf8()..])
        .unwrap_or_default()
}

/// Parses the string as a base-10 `int`, or returns `null` if it isn't one. Whitespace at either
/// end is ignored, and a leading `+` or `-` is allowed.
///
/// ```mimas
/// let a = " -12 ".to_int(); // -12
/// let b = "12px".to_int();  // null
/// let c = "4.2".to_int();   // null
/// ```
#[native]
fn to_int(s: &str) -> Option<i64> {
    s.trim().parse::<i64>().ok()
}

/// Returns the Unicode code point of the first character, or `null` if the string is empty.
///
/// ```mimas
/// let a = "A".ord(); // 65
/// let b = "".ord();  // null
/// ```
#[native]
fn ord(s: &str) -> Option<i64> {
    s.chars().next().map(|c| c as i64)
}

/// Returns whether the string has no characters. A string of spaces isn't empty.
///
/// ```mimas
/// let a = "".is_empty();  // true
/// let b = " ".is_empty(); // false
/// ```
#[native]
fn is_empty(s: &str) -> bool {
    s.is_empty()
}

/// Returns the string with every occurrence of `needle` replaced by `replacement`.
///
/// ```mimas
/// let a = "a-b-c".replace("-", "+"); // "a+b+c"
/// ```
#[native]
fn replace(s: &str, needle: &str, replacement: &str) -> String {
    s.replace(needle, replacement)
}

/// Returns the string repeated `count` times. A `count` of `0` gives an empty string.
///
/// A negative `count` is a runtime error.
///
/// ```mimas
/// let a = "ab".repeat(3);   // "ababab"
/// let line = "-".repeat(20);
/// ```
#[native]
fn repeat(s: &str, count: i64) -> Result<String, RtErr> {
    let count = usize::try_from(count)
        .map_err(|_| RtErr::InvalidArgument("repeat count cannot be negative".into()))?;
    if s.len()
        .checked_mul(count)
        .is_none_or(|total| total > isize::MAX as usize)
    {
        return Err(RtErr::InvalidArgument(
            "repeat result would be too large".into(),
        ));
    }
    Ok(s.repeat(count))
}
