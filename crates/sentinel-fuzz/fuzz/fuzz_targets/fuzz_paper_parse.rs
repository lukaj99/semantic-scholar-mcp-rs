#![no_main]

use libfuzzer_sys::fuzz_target;
use semantic_scholar_mcp::models::Paper;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as a Paper
    // Should never panic, only return Ok or Err
    let _ = serde_json::from_slice::<Paper>(data);
});
