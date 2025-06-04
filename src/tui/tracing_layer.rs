use crate::tui::UILogEntry;
use tokio::sync::mpsc;
use tracing_subscriber::Layer;
use tracing::{Event, Subscriber, Level};
use chrono::Local;

// Define a visitor to extract information from tracing events
struct LogEntryVisitor {
    timestamp: String,
    level: Level,
    target: String,
    message: Option<String>,
    // Potentially: file: Option<String>, line: Option<u32>
}

impl LogEntryVisitor {
    fn new(level: Level, target: String) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            level,
            target,
            message: None,
            // file: None,
            // line: None,
        }
    }
}

impl tracing::field::Visit for LogEntryVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
        // Potentially extract file and line here if they are passed as fields
        // Example:
        // if field.name() == "log.file" {
        //     self.file = Some(format!("{:?}", value));
        // }
        // if field.name() == "log.line" {
        //     if let Ok(line_val) = format!("{:?}", value).parse::<u32>() {
        //         self.line = Some(line_val);
        //     }
        // }
    }

    // Add other record_ methods if needed for different field types (e.g., record_str)
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

pub struct TuiLogCollectorLayer {
    sender: mpsc::UnboundedSender<UILogEntry>,
}

impl TuiLogCollectorLayer {
    pub fn new(sender: mpsc::UnboundedSender<UILogEntry>) -> Self {
        Self { sender }
    }
}

impl<S: Subscriber> Layer<S> for TuiLogCollectorLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let meta = event.metadata();
        let level = *meta.level();
        let target = meta.target().to_string();

        let mut visitor = LogEntryVisitor::new(level, target.clone());
        event.record(&mut visitor);

        if let Some(message) = visitor.message {
            let log_entry = UILogEntry {
                timestamp: visitor.timestamp,
                level: visitor.level,
                target: visitor.target, // Use the cloned target from visitor
                message,
                // file: visitor.file,
                // line: visitor.line,
            };

            // Send to TUI. If the receiver is dropped, this will fail silently.
            // Consider logging to stderr if sending fails for debugging.
            if let Err(e) = self.sender.send(log_entry) {
                eprintln!("Failed to send log to TUI: {}", e);
            }
        }
    }
    // We could also implement on_new_span, on_enter, on_exit, etc. if needed
    // for more advanced context, but for basic log messages, on_event is key.
} 