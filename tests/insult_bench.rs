//! Tests for `InsultBank` search optimizations and correctness.
//!
//! Verifies that insult searching works correctly across ASCII, Unicode,
//! and edge cases, and that optimizations (like fast paths) don't break logic.

use insult_arena_mcp::InsultBank;

#[test]
fn test_search_insults_ascii_fast_path() {
    let bank = InsultBank::new();
    // ASCII query, ASCII insult
    let results = bank.search_insults("dairy");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].insult, "You fight like a dairy farmer!");
}

#[test]
fn test_search_insults_ascii_case_insensitive() {
    let bank = InsultBank::new();
    let results = bank.search_insults("DAIRY");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_insults_unicode_fallback() {
    let bank = InsultBank::new();
    // Unicode query - should use existing slow path
    // "farmer" has 'f', 'a', 'r', 'm', 'e', 'r'.
    // If we search for "fármer" (with accent), it shouldn't match "farmer" unless we normalize accents (which we don't, only lowercase).
    // But if we search for "ñ", it should return empty safely.
    let results = bank.search_insults("ñ");
    assert!(results.is_empty());
}

#[test]
fn test_search_insults_mixed_unicode_query() {
    let bank = InsultBank::new();
    // "dairy" + unicode char
    let results = bank.search_insults("dairy ñ");
    // Should not match "dairy farmer" because "ñ" is not in it.
    assert!(results.is_empty());
}

#[test]
fn test_search_insults_long_query() {
    let bank = InsultBank::new();
    let query = "a".repeat(200);
    // Should truncate to 128 chars and search.
    // "a" repeats -> "aaaa..."
    // Insults don't contain 128 'a's.
    let results = bank.search_insults(&query);
    assert!(results.is_empty());
}

#[test]
fn test_search_insults_empty_query() {
    let bank = InsultBank::new();
    let results = bank.search_insults("");
    // Empty query matches everything (contains empty string is always true)
    assert_eq!(results.len(), 16);
}
