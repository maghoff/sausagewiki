use seahash;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all="kebab-case")]
pub enum Theme {
    Red,
    Pink,
    Purple,
    DeepPurple,
    Indigo,
    Blue,
    LightBlue,
    Cyan,
    Teal,
    Green,
    LightGreen,
    Lime,
    Yellow,
    Amber,
    Orange,
    DeepOrange,
    Brown,
    Gray,
    BlueGray,
}

forward_display_to_serde!(Theme);

use self::Theme::*;

pub const THEMES: [Theme; 19] = [Red, Pink, Purple, DeepPurple, Indigo, Blue,
    LightBlue, Cyan, Teal, Green, LightGreen, Lime, Yellow, Amber, Orange,
    DeepOrange, Brown, Gray, BlueGray];

pub fn theme_from_str(x: &str) -> Theme {
    let hash = seahash::hash(x.as_bytes()) as usize;
    let choice = hash % THEMES.len();
    THEMES[choice]
}

pub struct CssClass(Theme);

impl Theme {
    pub fn css_class(self) -> CssClass {
        CssClass(self)
    }
}

use std::fmt::{self, Display, Formatter};

impl Display for CssClass {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "theme-{}", self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_plain;

    #[test]
    fn basic_serialize() {
        assert_eq!(serde_plain::to_string(&Theme::Red).unwrap(), "red");
    }

    #[test]
    fn serialize_kebab_case() {
        assert_eq!(serde_plain::to_string(&Theme::LightGreen).unwrap(), "light-green");
    }

    #[test]
    fn basic_display() {
        assert_eq!(&Theme::Red.to_string(), "red");
    }

    #[test]
    fn display_kebab_case() {
        assert_eq!(&Theme::LightGreen.to_string(), "light-green");
    }

    #[test]
    fn to_number() {
        assert_eq!(Theme::Red as i32, 0);
        assert_eq!(Theme::LightGreen as i32, 10);
        assert_eq!(Theme::BlueGray as i32, 18);
    }

    #[test]
    fn from_str() {
        assert_eq!(theme_from_str("Bartefjes"), Theme::Orange);
    }

    #[test]
    fn css_class_display() {
        assert_eq!(&Theme::Red.css_class().to_string(), "theme-red");
    }
}
