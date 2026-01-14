#![no_main]

use libfuzzer_sys::fuzz_target;
use semantic_scholar_mcp::models::{Author, ExhaustiveSearchInput, Paper};

fuzz_target!(|data: &[u8]| {
    // First try to parse as valid JSON
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(data) {
        // Then try each model type
        let _ = serde_json::from_value::<Paper>(json.clone());
        let _ = serde_json::from_value::<Author>(json.clone());
        let _ = serde_json::from_value::<ExhaustiveSearchInput>(json);
    }
});
