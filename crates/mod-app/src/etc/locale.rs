use serde::{Deserialize, Serialize};

/// ## Application Locale
#[repr(C)]
#[derive(Deserialize, Serialize, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    #[default]
    English,
    Japanese,
    Korean,
}
