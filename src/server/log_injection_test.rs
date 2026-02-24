#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use crate::arena::{PlayerInput, SessionId};
use std::sync::{Arc, Mutex};

struct LogWriter(Arc<Mutex<String>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        self.0.lock().expect("mutex lock").push_str(&s);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_log_injection_throw_insult() {
    let buffer = Arc::new(Mutex::new(String::new()));
    let buffer_clone = buffer.clone();

    // Set up a subscriber that writes to our buffer
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || LogWriter(buffer_clone.clone()))
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime build");

        rt.block_on(async {
            let server = InsultServer::new();
            let _ = server.controller.start_duel().await;

            let session = SessionId::try_from("session".to_string()).unwrap();
            let _ = server.controller.register_challenger(session.clone()).await;

            // We attempt to throw an insult.
            // If the insult is unknown, it returns an error containing the input.
            // This will trigger the error path in `handle_throw_insult`.
            let malicious_input = "malicious\nINJECTED_LOG";

            // Note: PlayerInput validation ensures length, but allows newlines.
            let input = PlayerInput::try_from(malicious_input.to_string()).unwrap();

            tracing::warn!("TEST LOG");
            let _ = server.controller.throw_insult(session, input).await;
        });
    });

    let output = buffer.lock().expect("mutex lock").clone();

    // Verify that we actually captured something
    assert!(
        output.contains("TEST LOG"),
        "Failed to capture logs! Output is empty or missing expected log."
    );

    // Check if the output contains the unescaped newline.
    // If vulnerable, it will look like: ... "malicious\nINJECTED_LOG" ...
    // If fixed (Debug), it will look like: ... "malicious\\nINJECTED_LOG" ...

    // We assert that the raw newline followed by INJECTED_LOG is NOT present.
    // This assertion should FAIL if the code is vulnerable.
    assert!(
        !output.contains("malicious\nINJECTED_LOG"),
        "Log injection detected! Newline was not escaped. Output: {output}"
    );
}
