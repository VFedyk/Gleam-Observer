#[cfg(unix)]
use notify_rust::Notification;

use super::{Alert, AlertLevel};

pub struct Notifier {
    enabled: bool,
}

impl Notifier {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn send_alert(&self, alert: &Alert) {
        if !self.enabled {
            return;
        }

        #[cfg(unix)]
        {
            let summary = match alert.level {
                AlertLevel::Critical => "⚠️ Critical Alert",
                AlertLevel::Warning => "⚠ Warning",
                AlertLevel::Info => "ℹ Info",
            };

            let icon = match alert.level {
                AlertLevel::Critical => "dialog-error",
                AlertLevel::Warning => "dialog-warning",
                AlertLevel::Info => "dialog-information",
            };

            let mut notif = Notification::new();
            notif
                .summary(&format!("{} - GleamObserver", summary))
                .body(&alert.message)
                .icon(icon)
                .timeout(5000);

            #[cfg(target_os = "linux")]
            {
                use notify_rust::Urgency;
                let urgency = match alert.level {
                    AlertLevel::Critical => Urgency::Critical,
                    AlertLevel::Warning => Urgency::Normal,
                    AlertLevel::Info => Urgency::Low,
                };
                notif.urgency(urgency);
            }

            let _ = notif.show();
        }

        #[cfg(not(unix))]
        {
            // For non-Unix systems, just log
            log::warn!("Alert: {} - {}", 
                match alert.level {
                    AlertLevel::Critical => "CRITICAL",
                    AlertLevel::Warning => "WARNING",
                    AlertLevel::Info => "INFO",
                },
                alert.message
            );
        }
    }
}
