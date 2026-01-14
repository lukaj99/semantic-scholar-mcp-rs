#![no_main]

use libfuzzer_sys::fuzz_target;
use semantic_scholar_mcp::models::ExhaustiveSearchInput;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as ExhaustiveSearchInput
    let _ = serde_json::from_slice::<ExhaustiveSearchInput>(data);
});
