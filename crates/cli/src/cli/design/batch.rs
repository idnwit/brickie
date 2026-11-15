//! `design apply`: one op per stdin line, parsed with the same clap definitions as the write
//! subcommands.

use super::args::Line;
use super::ops;
use anyhow::{Result, anyhow, bail};
use brickie_change::{Design, Op};
use clap::Parser;
use std::str::Chars;

/// Split a line into words: whitespace separates, `'...'` is literal, `"..."` allows `\"` and
/// `\\`, and a backslash outside quotes escapes the next character.
pub fn words(line: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut live = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            if live {
                out.push(std::mem::take(&mut cur));
                live = false;
            }
            continue;
        }
        live = true;
        match c {
            '\'' => single_quoted(&mut chars, &mut cur)?,
            '"' => double_quoted(&mut chars, &mut cur)?,
            '\\' => cur.push(chars.next().ok_or_else(|| anyhow!("trailing backslash"))?),
            c => cur.push(c),
        }
    }
    if live {
        out.push(cur);
    }
    Ok(out)
}

/// Copy a `'...'` body (the opening quote is consumed) literally into `cur`.
fn single_quoted(chars: &mut Chars, cur: &mut String) -> Result<()> {
    for c in chars.by_ref() {
        if c == '\'' {
            return Ok(());
        }
        cur.push(c);
    }
    bail!("unterminated quote")
}

/// Copy a `"..."` body into `cur`; only `\"` and `\\` are escapes there.
fn double_quoted(chars: &mut Chars, cur: &mut String) -> Result<()> {
    while let Some(c) = chars.next() {
        match c {
            '"' => return Ok(()),
            '\\' => {
                let e = chars.next().ok_or_else(|| anyhow!("unterminated quote"))?;
                if e != '"' && e != '\\' {
                    cur.push('\\');
                }
                cur.push(e);
            }
            c => cur.push(c),
        }
    }
    bail!("unterminated quote")
}

/// The ops of one line.
fn parse(d: &Design, line: &str) -> Result<Vec<Op>> {
    let mut args = words(line)?;
    // The write arguments are required by the subcommands; a line must not carry its own.
    args.extend(["--base".into(), "0".into()]);
    let l = Line::try_parse_from(args).map_err(|e| {
        let text = e.to_string();
        let first = text.lines().next().unwrap_or("invalid line");
        anyhow::anyhow!("{}", first.trim_start_matches("error: "))
    })?;
    let (ops, w) = ops::edit(d, l.edit)?;
    if w.base != 0 || w.note.is_some() || w.dry_run || w.scaffold {
        bail!(
            "--base, --note, --dry-run and --scaffold belong to the `apply` command, not to a line"
        );
    }
    Ok(ops)
}

/// Every op of the script; the first bad line stops it, named by its number.
pub fn ops(d: &Design, script: &str) -> Result<Vec<Op>> {
    let mut all = Vec::new();
    for (i, raw) in script.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let n = i + 1;
        match parse(d, line) {
            Ok(ops) => all.extend(ops),
            Err(e) => bail!("line {n}: {e:#}; nothing was written"),
        }
    }
    if all.is_empty() {
        bail!("no ops on stdin");
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::words;

    fn w(line: &str) -> Vec<String> {
        words(line).unwrap()
    }

    fn err(line: &str) -> String {
        words(line).unwrap_err().to_string()
    }

    #[test]
    fn plain_words_and_repeated_whitespace() {
        assert_eq!(w("a b  c\t d"), ["a", "b", "c", "d"]);
        assert_eq!(w("  a  "), ["a"]);
        assert!(w("   ").is_empty());
    }

    #[test]
    fn quotes_group_words() {
        assert_eq!(w(r#"x "a b" y"#), ["x", "a b", "y"]);
        assert_eq!(w("x 'a b' y"), ["x", "a b", "y"]);
    }

    #[test]
    fn quote_adjacent_to_text_joins_one_word() {
        assert_eq!(w(r#"--reason="x y""#), ["--reason=x y"]);
        assert_eq!(w("--reason='x y' z"), ["--reason=x y", "z"]);
        assert_eq!(w(r#"a"b c"d"#), ["ab cd"]);
    }

    #[test]
    fn backslash_escapes_in_double_quotes() {
        assert_eq!(w(r#""a\"b""#), [r#"a"b"#]);
        assert_eq!(w(r#""a\\b""#), [r"a\b"]);
        // Any other escape keeps the backslash.
        assert_eq!(w(r#""a\nb""#), [r"a\nb"]);
    }

    #[test]
    fn backslash_outside_quotes_escapes_the_next_char() {
        assert_eq!(w(r"a\ b"), ["a b"]);
        assert_eq!(w(r"a\\b"), [r"a\b"]);
        assert_eq!(w(r#"\"x"#), [r#""x"#]);
    }

    #[test]
    fn backslash_in_single_quotes_is_literal() {
        assert_eq!(w(r"'a\b'"), [r"a\b"]);
        assert_eq!(w(r"'a\' b"), [r"a\", "b"]);
    }

    #[test]
    fn empty_quoted_string_is_an_empty_argument() {
        assert_eq!(w(r#"a "" b"#), ["a", "", "b"]);
        assert_eq!(w("''"), [""]);
    }

    #[test]
    fn hash_is_part_of_the_value() {
        assert_eq!(w("a#b #c d#"), ["a#b", "#c", "d#"]);
        assert_eq!(w(r#""a #b""#), ["a #b"]);
    }

    #[test]
    fn errors_for_unterminated_quote_and_trailing_backslash() {
        assert_eq!(err(r#"a "b"#), "unterminated quote");
        assert_eq!(err("a 'b"), "unterminated quote");
        assert_eq!(err(r#""b\"#), "unterminated quote");
        assert_eq!(err(r"a b\"), "trailing backslash");
    }
}
