//! Columns: ids, validated values and the `Column` entity.

use crate::error::{ColumnId, DomainError};

const MAX_TITLE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnTitle(String);

impl ColumnTitle {
    /// Trims the input; it must be non-empty and at most 100 chars.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        if trimmed.chars().count() > MAX_TITLE {
            return Err(DomainError::TitleTooLong);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Color(String);

impl Color {
    /// Accepts `#rrggbb` hex only; stored lower-case.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let digits = raw.strip_prefix('#').ok_or(DomainError::InvalidColor)?;
        if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DomainError::InvalidColor);
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub id: ColumnId,
    pub title: ColumnTitle,
    pub color: Color,
    pub is_close: bool,
    pub order: usize,
}

impl Column {
    pub fn new(
        id: ColumnId,
        title: ColumnTitle,
        color: Color,
        is_close: bool,
        order: usize,
    ) -> Self {
        Self {
            id,
            title,
            color,
            is_close,
            order,
        }
    }

    pub fn rename(&mut self, title: ColumnTitle) {
        self.title = title;
    }

    pub fn recolor(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_close(&mut self, is_close: bool) {
        self.is_close = is_close;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_trimmed() {
        assert_eq!(ColumnTitle::parse("  Todo ").unwrap().as_str(), "Todo");
    }

    #[test]
    fn title_rejects_empty_and_long() {
        assert_eq!(ColumnTitle::parse("   "), Err(DomainError::EmptyTitle));
        assert_eq!(
            ColumnTitle::parse(&"a".repeat(101)),
            Err(DomainError::TitleTooLong)
        );
        assert!(ColumnTitle::parse(&"a".repeat(100)).is_ok());
    }

    #[test]
    fn color_accepts_hex_only() {
        assert_eq!(Color::parse("#A1b2C3").unwrap().as_str(), "#a1b2c3");
        for bad in ["", "a1b2c3", "#a1b2c", "#a1b2c3d", "#gggggg"] {
            assert_eq!(Color::parse(bad), Err(DomainError::InvalidColor), "{bad}");
        }
    }

    #[test]
    fn column_edits() {
        let mut c = Column::new(
            ColumnId(1),
            ColumnTitle::parse("A").unwrap(),
            Color::parse("#000000").unwrap(),
            false,
            0,
        );
        c.rename(ColumnTitle::parse("B").unwrap());
        c.recolor(Color::parse("#ffffff").unwrap());
        c.set_close(true);
        assert_eq!(c.title.as_str(), "B");
        assert_eq!(c.color.as_str(), "#ffffff");
        assert!(c.is_close);
    }
}
