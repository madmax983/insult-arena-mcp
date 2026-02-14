use insult_arena_mcp::InsultBank;
use insult_arena_mcp::experimental::sensei::Sensei;

#[test]
fn reproduce_sensei_nan_panic() {
    // Create Sensei with NaN skill
    let sensei = Sensei::new(f64::NAN);
    let bank = InsultBank::new();

    // Attack is fine (doesn't use random bool based on skill)
    let insult = sensei.attack(&bank);

    // Defend uses gen_bool(skill). If skill was NaN, it should have been sanitized to 0.0.
    // This should NOT panic.
    let comeback = sensei.defend(&bank, &insult);

    // Verify it defaulted to 0.0 (fail mode)
    // Note: With skill 0.0, it fails 100% of the time.
    assert_eq!(comeback, "I am rubber, you are glue!");
}
