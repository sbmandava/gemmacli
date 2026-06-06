//! Pure-logic unit tests (no model, no network, fast).
use genie::llm::{is_error_marker, is_noise};
use genie::rag::chunk_text;

#[test]
fn chunk_small_text_is_single_chunk() {
    let chunks = chunk_text("hello world", 1000, 150);
    assert_eq!(chunks, vec!["hello world".to_string()]);
}

#[test]
fn chunk_long_text_overlaps_and_bounds() {
    let text = "a".repeat(2500); // > max_chars
    let chunks = chunk_text(&text, 1000, 150);
    assert!(chunks.len() >= 3, "expected >=3 chunks, got {}", chunks.len());
    assert!(
        chunks.iter().all(|c| c.chars().count() <= 1000),
        "every chunk must be <= max_chars"
    );
}

#[test]
fn chunk_blank_text_is_empty() {
    assert!(chunk_text("   \n  ", 1000, 150).is_empty());
}

#[test]
fn noise_lines_detected() {
    assert!(is_noise("INFO: Loaded OpenCL library with dlopen."));
    assert!(is_noise(
        "Warning: maxDynamicStorageBuffersPerPipelineLayout artificially reduced"
    ));
    assert!(!is_noise("The sky is blue because of Rayleigh scattering."));
}

#[test]
fn error_marker_detected() {
    assert!(is_error_marker("An error occurred"));
    assert!(is_error_marker("   An error occurred."));
    assert!(!is_error_marker("This sentence is fine."));
}
