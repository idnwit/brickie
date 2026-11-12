//! Brick selectors used by rules: a brick name, `tag:<tag>` or `*`.

use super::Brick;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Sel {
    #[default]
    Any,
    Tag(String),
    Brick(String),
}

impl TryFrom<String> for Sel {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        match s.as_str() {
            "*" => Ok(Sel::Any),
            "" | "tag:" => Err(format!("empty selector '{s}'")),
            _ => Ok(match s.strip_prefix("tag:") {
                Some(t) => Sel::Tag(t.to_string()),
                None => Sel::Brick(s),
            }),
        }
    }
}

impl<'de> Deserialize<'de> for Sel {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Sel::try_from(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl Sel {
    /// 3 for a brick name, 2 for a tag, 1 for `*`; `None` when the brick does not match.
    pub fn specificity(&self, brick: &Brick) -> Option<u32> {
        match self {
            Sel::Any => Some(1),
            Sel::Tag(t) => brick.tags.contains(t).then_some(2),
            Sel::Brick(n) => (brick.name == *n).then_some(3),
        }
    }
}

impl std::fmt::Display for Sel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sel::Any => write!(f, "*"),
            Sel::Tag(t) => write!(f, "tag:{t}"),
            Sel::Brick(n) => write!(f, "{n}"),
        }
    }
}
