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

#[native]
fn len(s: &str) -> usize {
    s.chars().count()
}

#[native]
fn contains(s: &str, needle: &str) -> bool {
    s.contains(needle)
}

#[native]
fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

#[native]
fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

#[native]
fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

#[native]
fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

#[native]
fn trim(s: &str) -> String {
    s.trim().to_string()
}

#[native]
fn to_snake(s: &str) -> String {
    s.to_snake_case()
}

#[native]
fn split(s: &str, delim: &str) -> Vec<String> {
    s.split(&delim).map(|v| v.to_string()).collect()
}

#[native]
fn lines(s: &str) -> Vec<String> {
    s.lines().map(From::from).collect()
}

// todo, the handling of option vs raisable below is bad

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

#[native]
fn capitalize(s: &str) -> String {
    s.chars()
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + &s[c.len_utf8()..])
        .unwrap_or_default()
}

#[native]
fn to_int(s: &str) -> Option<i64> {
    s.trim().parse::<i64>().ok()
}

#[native]
fn ord(s: &str) -> Option<i64> {
    s.chars().next().map(|c| c as i64)
}

#[native]
fn is_empty(s: &str) -> bool {
    s.is_empty()
}

#[native]
fn replace(s: &str, needle: &str, new: &str) -> String {
    s.replace(needle, new)
}

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
