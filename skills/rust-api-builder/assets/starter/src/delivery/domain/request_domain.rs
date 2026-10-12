use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Options for an outbound call made through `RequestService`.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RequestOptionDomain {
    pub url: String,
    pub method: String,
    /// JSON body as a string; empty for no body
    pub payload: String,
    pub headers: HashMap<String, String>,
}
