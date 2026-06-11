use chrono::{DateTime, Local, NaiveDate};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedKind {
    Being,
    Doing,
}

impl fmt::Display for SeedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SeedKind::Being => "being",
            SeedKind::Doing => "doing",
        };
        f.write_str(s)
    }
}

#[derive(Debug)]
pub struct ParseSeedKindError(String);

impl fmt::Display for ParseSeedKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid seed kind: {}", self.0)
    }
}

impl std::error::Error for ParseSeedKindError {}

impl FromStr for SeedKind {
    type Err = ParseSeedKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "being" => Ok(SeedKind::Being),
            "doing" => Ok(SeedKind::Doing),
            other => Err(ParseSeedKindError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seed {
    pub id: Uuid,
    pub text: String,
    pub kind: SeedKind,
    pub planted_on: NaiveDate,
    pub until: Option<NaiveDate>, // None = ongoing
    pub archived: bool,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

impl Seed {
    pub fn new(text: String, kind: SeedKind, planted_on: NaiveDate, until: Option<NaiveDate>) -> Self {
        let now = Local::now();
        Self {
            id: Uuid::new_v4(),
            text,
            kind,
            planted_on,
            until,
            archived: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_kind_roundtrip_string() {
        assert_eq!(SeedKind::Being.to_string(), "being");
        assert_eq!(SeedKind::Doing.to_string(), "doing");
        assert_eq!("being".parse::<SeedKind>().unwrap(), SeedKind::Being);
        assert_eq!("doing".parse::<SeedKind>().unwrap(), SeedKind::Doing);
        assert!("sprouting".parse::<SeedKind>().is_err());
    }

    #[test]
    fn test_new_seed_defaults() {
        let day = Local::now().date_naive();
        let seed = Seed::new("Be more attentive".to_string(), SeedKind::Being, day, None);
        assert_eq!(seed.kind, SeedKind::Being);
        assert!(!seed.archived);
        assert!(seed.until.is_none());
        assert_eq!(seed.created_at, seed.updated_at);
    }
}
