//! `brickie llms.txt`: the static guide for coding agents, embedded in the binary.

use anyhow::Result;

pub const GUIDE: &str = include_str!("../../docs/llms.txt");

/// Prints the guide; it never reads the project or its config.
pub fn run() -> Result<i32> {
    print!("{GUIDE}");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::GUIDE;
    use crate::rules;

    #[test]
    fn guide_documents_every_diagnostic_code() {
        for code in rules::all_codes() {
            assert!(
                GUIDE.contains(&code),
                "docs/llms.txt does not mention diagnostic code `{code}`"
            );
        }
    }
}
