use serde::{Deserialize, Serialize};

/// Schools available in the entry form's select widget.
/// Add more entries here as needed — nothing else has to change.
pub const SCHOOLS: &[&str] = &["Southlands", "Convent", "All Saints", "Vidyaloka", "Aloysius"];

pub const GROUP_COUNT: u8 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: u64,
    pub name: String,
    pub school: String,
    pub group: u8,
}
