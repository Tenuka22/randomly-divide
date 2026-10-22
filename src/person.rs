use serde::{Deserialize, Serialize};

/// Schools available in the entry form's select widget.
/// Add more entries here as needed — nothing else has to change.
pub const SCHOOLS: &[&str] = &["Southlands", "Convent", "All Saints", "Vidyaloka", "Aloysius"];

pub const GROUP_COUNT: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    pub const ALL: [Gender; 2] = [Gender::Male, Gender::Female];

    pub fn label(self) -> &'static str {
        match self {
            Gender::Male => "Male",
            Gender::Female => "Female",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Gender::Male => "M",
            Gender::Female => "F",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: u64,
    pub name: String,
    /// Free-form class/section label, e.g. "8-B".
    pub class: String,
    pub school: String,
    pub gender: Gender,
    pub group: u8,
    /// Seed used to deterministically regenerate this person's hashed
    /// identicon avatar (see `crate::avatar`).
    pub avatar_seed: u64,
}
