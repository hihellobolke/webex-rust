//! Style types for Adaptive Cards including colors, spacing, weights, and alignment options.

use serde_with::{DeserializeFromStr, SerializeDisplay};

/// Color for text
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "PascalCase")]
pub enum Color {
    Default,
    Dark,
    Light,
    Accent,
    Good,
    Warning,
    Attention,
}

/// Style hint for Container.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "camelCase")]
pub enum ContainerStyle {
    Default,
    Emphasis,
    Good,
    Attention,
    Warning,
    Accent,
}

/// Controls the amount of spacing between this element and the preceding element.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "PascalCase")]
pub enum Spacing {
    Default,
    None,
    Small,
    Medium,
    Large,
    ExtraLarge,
    Padding,
}

/// Style for Input.ChoiceSet
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "camelCase")]
pub enum ChoiceInputStyle {
    Compact,
    Expanded,
}

/// Vertical content alignment
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "PascalCase")]
pub enum VerticalContentAlignment {
    Top,
    Center,
    Bottom,
}

/// Text input style
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "camelCase")]
pub enum TextInputStyle {
    Text,
    Tel,
    Url,
    Email,
}

/// Height of element
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum Height {
    Auto,
    Stretch,
}

/// Style hint for Image.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "camelCase")]
pub enum ImageStyle {
    Default,
    Person,
}

/// Weight of text
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "PascalCase")]
pub enum Weight {
    Default,
    Lighter,
    Bolder,
}

/// Font type
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "PascalCase")]
pub enum FontType {
    Default,
    Monospace,
}

/// Size of text
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "PascalCase")]
pub enum Size {
    Default,
    Small,
    Medium,
    Large,
    ExtraLarge,
}

/// Horizontal alignment
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "PascalCase")]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

/// Size of image (pixel width)
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, SerializeDisplay, DeserializeFromStr)]
#[derive(strum_macros::EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive, serialize_all = "PascalCase")]
pub enum ImageSize {
    Auto,
    Stretch,
    Small,
    Medium,
    Large,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn de<T: serde::de::DeserializeOwned>(s: &str) -> T {
        serde_json::from_str(&format!("\"{s}\"")).unwrap()
    }

    fn ser<T: serde::Serialize>(v: &T) -> String {
        serde_json::to_string(v).unwrap().trim_matches('"').to_owned()
    }

    // ── Color ────────────────────────────────────────────────────────────────

    #[test]
    fn color_lowercase_accepted() {
        assert_eq!(de::<Color>("attention"), Color::Attention);
        assert_eq!(de::<Color>("good"), Color::Good);
        assert_eq!(de::<Color>("warning"), Color::Warning);
    }

    #[test]
    fn color_mixed_case_accepted() {
        assert_eq!(de::<Color>("Attention"), Color::Attention);
        assert_eq!(de::<Color>("ATTENTION"), Color::Attention);
    }

    #[test]
    fn color_round_trip() {
        let original = Color::Attention;
        let serialized = ser(&original);
        assert_eq!(serialized, "Attention");
        let deserialized: Color = de(&serialized);
        assert_eq!(deserialized, original);
    }

    #[test]
    fn color_invalid_returns_error() {
        let result = serde_json::from_str::<Color>("\"ultraviolet\"");
        assert!(result.is_err(), "expected error for unknown color variant");
    }

    // ── Spacing ──────────────────────────────────────────────────────────────

    #[test]
    fn spacing_lowercase_accepted() {
        assert_eq!(de::<Spacing>("small"), Spacing::Small);
        assert_eq!(de::<Spacing>("large"), Spacing::Large);
        assert_eq!(de::<Spacing>("none"), Spacing::None);
    }

    #[test]
    fn spacing_mixed_case_accepted() {
        assert_eq!(de::<Spacing>("SMALL"), Spacing::Small);
        assert_eq!(de::<Spacing>("ExtraLarge"), Spacing::ExtraLarge);
    }

    #[test]
    fn spacing_round_trip() {
        let v = Spacing::ExtraLarge;
        assert_eq!(ser(&v), "ExtraLarge");
        assert_eq!(de::<Spacing>(&ser(&v)), v);
    }

    // ── Size ─────────────────────────────────────────────────────────────────

    #[test]
    fn size_lowercase_accepted() {
        assert_eq!(de::<Size>("small"), Size::Small);
        assert_eq!(de::<Size>("large"), Size::Large);
    }

    #[test]
    fn size_uppercase_accepted() {
        assert_eq!(de::<Size>("LARGE"), Size::Large);
    }

    #[test]
    fn size_round_trip() {
        let v = Size::Large;
        assert_eq!(ser(&v), "Large");
        assert_eq!(de::<Size>(&ser(&v)), v);
    }

    // ── HorizontalAlignment ──────────────────────────────────────────────────

    #[test]
    fn horizontal_alignment_lowercase_accepted() {
        assert_eq!(de::<HorizontalAlignment>("left"), HorizontalAlignment::Left);
        assert_eq!(de::<HorizontalAlignment>("right"), HorizontalAlignment::Right);
        assert_eq!(de::<HorizontalAlignment>("center"), HorizontalAlignment::Center);
    }

    #[test]
    fn horizontal_alignment_round_trip() {
        let v = HorizontalAlignment::Left;
        assert_eq!(ser(&v), "Left");
        assert_eq!(de::<HorizontalAlignment>(&ser(&v)), v);
    }
}
