#[cfg(test)]
mod tests {
    use tracing::{info, Level};
    use tracing_subscriber::FmtSubscriber;

    #[test]
    fn test_log_injection() {
        // This test demonstrates that Display formatting allows newline injection
        // creating fake log entries.

        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            let malicious_input = "innocent insult\nINFO 2024-01-01T00:00:00Z: 🔓 User admin logged in";
            info!("🗣️  INSULT: \"{}\"", malicious_input);
            // Expected output (simulated):
            // INFO ... 🗣️  INSULT: "innocent insult
            // INFO 2024-01-01T00:00:00Z: 🔓 User admin logged in"

            // Contrast with Debug formatting:
            info!("Debug: {:?}", malicious_input);
            // INFO ... Debug: "innocent insult\nINFO 2024-01-01T00:00:00Z: 🔓 User admin logged in"
        });
    }
}
