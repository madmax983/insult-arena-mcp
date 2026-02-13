use rust_mcp_sdk::mcp_server::hyper_runtime::HyperRuntime;
use rust_mcp_sdk::schema::CustomNotification;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, Semaphore, mpsc};
use tracing::{info, warn};

use crate::duel::DuelStateView;

/// Internal job for sending notifications to sessions.
struct NotificationJob {
    sessions: Vec<String>,
    notification: CustomNotification,
}

/// Manages broadcasting notifications to connected clients.
#[derive(Clone)]
pub struct NotificationManager {
    runtime: Arc<RwLock<Option<Arc<HyperRuntime>>>>,
    notification_tx: mpsc::Sender<NotificationJob>,
    // Holds the receiver until it can be spawned in set_runtime.
    // Wrapped in Arc<Mutex<Option>> so we can take it out safely and share NotificationManager.
    notification_rx: Arc<Mutex<Option<mpsc::Receiver<NotificationJob>>>>,
}

impl NotificationManager {
    /// Creates a new [`NotificationManager`].
    #[must_use]
    pub fn new() -> Self {
        // Create a bounded channel for notifications (load shedding)
        let (tx, rx) = mpsc::channel(100);

        Self {
            runtime: Arc::new(RwLock::new(None)),
            notification_tx: tx,
            notification_rx: Arc::new(Mutex::new(Some(rx))),
        }
    }

    /// Sets the `HyperRuntime` for sending notifications.
    /// Call this after `server.start_runtime()` returns.
    pub async fn set_runtime(&self, runtime: Arc<HyperRuntime>) {
        let mut guard = self.runtime.write().await;
        *guard = Some(runtime);

        // Start the notification worker if it hasn't been started yet
        let mut rx_opt = self.notification_rx.lock().await;
        if let Some(rx) = rx_opt.take() {
            let runtime_lock = self.runtime.clone();
            tokio::spawn(Self::notification_worker(rx, runtime_lock));
        }
    }

    /// Background worker that processes notifications with bounded concurrency.
    async fn notification_worker(
        mut rx: mpsc::Receiver<NotificationJob>,
        runtime_lock: Arc<RwLock<Option<Arc<HyperRuntime>>>>,
    ) {
        // Concurrency limit for notification tasks
        let semaphore = Arc::new(Semaphore::new(50));

        info!("👷 Notification worker started");

        while let Some(job) = rx.recv().await {
            let NotificationJob {
                sessions,
                notification,
            } = job;

            // Get runtime
            let runtime = {
                let guard = runtime_lock.read().await;
                guard.clone()
            };

            let Some(runtime) = runtime else {
                warn!("⚠️  Worker cannot send notification: Runtime not initialized");
                continue;
            };

            for session_id in sessions {
                let Ok(permit) = semaphore.clone().acquire_owned().await else {
                    break; // Semaphore closed
                };

                let runtime = runtime.clone();
                let notification = notification.clone();

                tokio::spawn(async move {
                    // Task holds the permit until done
                    let _permit = permit;
                    info!("   → Sending to session: {:?}", session_id);
                    if let Err(e) = runtime.notify_custom(&session_id, notification).await {
                        warn!(
                            "   ✗ Failed to send notification to {:?}: {:?}",
                            session_id, e
                        );
                    } else {
                        info!("   ✓ Notification sent to {:?}", session_id);
                    }
                });
            }
        }
        info!("👷 Notification worker stopped");
    }

    /// Broadcasts a turn notification to all connected sessions.
    pub async fn notify_turn(&self, state: &DuelStateView) {
        let runtime_guard = self.runtime.read().await;
        let Some(runtime) = runtime_guard.as_ref() else {
            warn!("⚠️  Cannot send notification: Runtime not initialized");
            return;
        };

        let sessions = runtime.sessions().await;
        info!("📡 Active sessions: {} connected", sessions.len());

        if sessions.is_empty() {
            info!("   No active sessions to notify");
            return;
        }

        let params = json!({
            "type": "turn_notification",
            "state": state,
            "message": format!(
                "It's {}'s turn!",
                state.next_to_act.as_deref().unwrap_or("unknown")
            )
        });

        let notification = CustomNotification {
            method: "notifications/turn".to_string(),
            params: params.as_object().cloned(),
        };

        info!(
            "📢 Broadcasting to {} session(s): {}'s turn",
            sessions.len(),
            state.next_to_act.as_deref().unwrap_or("unknown")
        );

        // 🛡️ HARDENING: Use bounded channel for load shedding.
        let job = NotificationJob {
            sessions,
            notification,
        };

        match self.notification_tx.try_send(job) {
            Ok(()) => info!("   ✓ Notification job queued"),
            Err(e) => warn!(
                "⚠️  Notification DROPPED due to high load (channel full): {:?}",
                e
            ),
        }
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}
