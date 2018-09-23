use std::io::Write;

use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use seahash;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[derive(Serialize, Deserialize)] // Serde
#[serde(rename_all="kebab-case")]
#[derive(AsExpression, FromSqlRow)] // Diesel
#[sql_type = "Text"]
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

use self::Theme::*;

forward_display_to_serde!(Theme);
forward_from_str_to_serde!(Theme);

pub const THEMES: [Theme; 19] = [Red, Pink, Purple, DeepPurple, Indigo, Blue,
    LightBlue, Cyan, Teal, Green, LightGreen, Lime, Yellow, Amber, Orange,
    DeepOrange, Brown, Gray, BlueGray];

pub fn theme_from_str_hash(x: &str) -> Theme {
    let hash = seahash::hash(x.as_bytes()) as usize;
    let choice = hash % THEMES.len();
    THEMES[choice]
}

impl ToSql<Text, Sqlite> for Theme {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        ToSql::<Text, Sqlite>::to_sql(&self.to_string(), out)
    }
}

impl FromSql<Text, Sqlite> for Theme {
    fn from_sql(value: Option<&<Sqlite as Backend>::RawValue>) -> deserialize::Result<Self> {
        // See Diesel's documentation on how to implement FromSql for Sqlite,
        // especially with regards to the unsafe conversion below.
        // http://docs.diesel.rs/diesel/deserialize/trait.FromSql.html
        let text_ptr = <*const str as FromSql<Text, Sqlite>>::from_sql(value)?;
        let text = unsafe { &*text_ptr };
        text.parse().map_err(Into::into)
    }
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
    use std::error::Error;

    use diesel::prelude::*;
    use diesel::sql_query;
    use diesel::sql_types::Text;
    use serde_json;
    use serde_plain;
    use serde_urlencoded;

    use super::*;

    #[test]
    fn basic_serialize() {
        assert_eq!(serde_plain::to_string(&Theme::Red).unwrap(), "red");
    }

    #[test]
    fn serialize_kebab_case() {
        assert_eq!(serde_plain::to_string(&Theme::LightGreen).unwrap(), "light-green");
    }

    #[test]
    fn serialize_json() {
        #[derive(Serialize)]
        struct Test { x: Theme }
        assert_eq!(
            serde_json::to_string(&Test { x: Theme::Red }).unwrap(),
            "{\"x\":\"red\"}"
        );
    }

    #[test]
    fn deserialize_json() {
        #[derive(Deserialize, Debug, PartialEq, Eq)]
        struct Test { x: Theme }
        assert_eq!(
            serde_json::from_str::<Test>("{\"x\":\"red\"}").unwrap(),
            Test { x: Theme::Red }
        );
    }

    #[test]
    fn serialize_urlencoded() {
        #[derive(Serialize)]
        struct Test { x: Theme }
        assert_eq!(
            serde_urlencoded::to_string(&Test { x: Theme::Red }).unwrap(),
            "x=red"
        );
    }

    #[test]
    fn deserialize_urlencoded() {
        #[derive(Deserialize, Debug, PartialEq, Eq)]
        struct Test { x: Theme }
        assert_eq!(
            serde_urlencoded::from_str::<Test>("x=red").unwrap(),
            Test { x: Theme::Red }
        );
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
    fn basic_from_str() {
        let indigo: Theme = "indigo".parse().unwrap();
        assert_eq!(indigo, Theme::Indigo);
    }

    #[test]
    fn to_number() {
        assert_eq!(Theme::Red as i32, 0);
        assert_eq!(Theme::LightGreen as i32, 10);
        assert_eq!(Theme::BlueGray as i32, 18);
    }

    #[test]
    fn from_str_hash() {
        assert_eq!(theme_from_str_hash("Bartefjes"), Theme::Orange);
    }

    #[test]
    fn css_class_display() {
        assert_eq!(&Theme::Red.css_class().to_string(), "theme-red");
    }

    #[test]
    fn basic_db_roundtrip() -> Result<(), Box<Error>> {
        let conn = SqliteConnection::establish(":memory:")?;

        #[derive(QueryableByName, PartialEq, Eq, Debug)]
        struct Row { #[sql_type = "Text"] theme: Theme }

        let res = sql_query("SELECT ? as theme")
            .bind::<Text, _>(DeepPurple)
            .load::<Row>(&conn)?;

        assert_eq!(&[ Row { theme: DeepPurple } ], res.as_slice());

        Ok(())
    }

    #[test]
    fn db_invalid_value_gives_error() -> Result<(), Box<Error>> {
        let conn = SqliteConnection::establish(":memory:")?;

        #[derive(QueryableByName, PartialEq, Eq, Debug)]
        struct Row { #[sql_type = "Text"] theme: Theme }

        let res = sql_query("SELECT 'green' as theme")
            .load::<Row>(&conn);
        assert!(res.is_ok());

        let res = sql_query("SELECT 'blueish-yellow' as theme")
            .load::<Row>(&conn);
        assert!(res.is_err());

        Ok(())
    }
}
