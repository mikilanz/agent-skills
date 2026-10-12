use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Values of the `status` column every table carries.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum StatusEnum {
    Active,
    Inactive,
}

impl StatusEnum {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusEnum::Active => "active",
            StatusEnum::Inactive => "inactive",
        }
    }
}
