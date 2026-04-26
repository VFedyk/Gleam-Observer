use crate::alerts::Alert;

pub fn send_alert(alert: &Alert) {
    #[cfg(unix)]
    {
        use notify_rust::Notification;

        let summary = match alert.level {
            crate::alerts::AlertLevel::Critical => "Critical Alert",
            crate::alerts::AlertLevel::Warning => "Warning",
            crate::alerts::AlertLevel::Info => "Info",
        };

        let icon = match alert.level {
            crate::alerts::AlertLevel::Critical => "dialog-error",
            crate::alerts::AlertLevel::Warning => "dialog-warning",
            crate::alerts::AlertLevel::Info => "dialog-information",
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
                crate::alerts::AlertLevel::Critical => Urgency::Critical,
                crate::alerts::AlertLevel::Warning => Urgency::Normal,
                crate::alerts::AlertLevel::Info => Urgency::Low,
            };
            notif.urgency(urgency);
        }

        let _ = notif.show();
    }

    #[cfg(not(unix))]
    {
        log::warn!("Alert: {} - {}", 
            match alert.level {
                crate::alerts::AlertLevel::Critical => "CRITICAL",
                crate::alerts::AlertLevel::Warning => "WARNING",
                crate::alerts::AlertLevel::Info => "INFO",
            },
            alert.message
        );
    }
}

pub fn send_status_update(cpu: f32, mem: f32) {
    #[cfg(unix)]
    {
        use notify_rust::Notification;
        
        let body = format!("CPU: {:.1}% | MEM: {:.1}%", cpu, mem);
        
        let _ = Notification::new()
            .summary("GleamObserver Status")
            .body(&body)
            .icon("gleamobserver")
            .timeout(3000)
            .show();
    }

    #[cfg(not(unix))]
    {
        log::info!("Status: CPU: {:.1}% | MEM: {:.1}%", cpu, mem);
    }
}
