//! Tracing setup: a daily-rotating file log plus a console log in debug builds.
//!
//! Every byte written to disk passes through [`RedactingWriter`], which buffers
//! per formatted record so a token split across `write` calls is still caught.

use std::io::{self, Write};
use std::sync::Arc;

use tauri::Manager;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::error::AppError;
use crate::redact::Redactor;

/// Writes formatted log records, redacting secrets before they hit the file.
pub struct RedactingWriter<W: Write> {
    inner: W,
    redactor: Arc<Redactor>,
    buffer: Vec<u8>,
}

impl<W: Write> RedactingWriter<W> {
    fn new(inner: W, redactor: Arc<Redactor>) -> Self {
        Self {
            inner,
            redactor,
            buffer: Vec::with_capacity(256),
        }
    }

    fn drain(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let text = String::from_utf8_lossy(&self.buffer);
        let redacted = self.redactor.apply(&text);
        self.buffer.clear();
        self.inner.write_all(redacted.as_bytes())
    }
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.drain()?;
        self.inner.flush()
    }
}

impl<W: Write> Drop for RedactingWriter<W> {
    fn drop(&mut self) {
        // Best effort: the app is shutting down and nothing can act on an error.
        let _ = self.drain();
    }
}

pub struct RedactingMakeWriter {
    appender: RollingFileAppender,
    redactor: Arc<Redactor>,
}

impl<'a> MakeWriter<'a> for RedactingMakeWriter {
    type Writer = RedactingWriter<<RollingFileAppender as MakeWriter<'a>>::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter::new(self.appender.make_writer(), Arc::clone(&self.redactor))
    }
}

/// Installs the global subscriber. Must be called exactly once, before any
/// other component emits a span.
pub fn init(app: &tauri::AppHandle) -> Result<(), AppError> {
    let directory = app
        .path()
        .app_log_dir()
        .map_err(|error| AppError::Config(format!("cannot resolve log directory: {error}")))?;

    std::fs::create_dir_all(&directory).map_err(|error| {
        AppError::Config(format!(
            "cannot create log directory {}: {error}",
            directory.display()
        ))
    })?;

    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("pulse")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&directory)
        .map_err(|error| AppError::Config(format!("cannot open log file: {error}")))?;

    // Default to info, but keep our own crate verbose enough to debug the
    // backend during development.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,pulse_lib=debug"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(RedactingMakeWriter {
            appender,
            redactor: crate::redact::shared(),
        })
        .with_ansi(false)
        .with_target(true);

    let subscriber = tracing_subscriber::registry().with(filter).with(file_layer);

    #[cfg(debug_assertions)]
    let subscriber = subscriber.with(
        tracing_subscriber::fmt::layer()
            .with_writer(io::stdout)
            .with_ansi(true),
    );

    subscriber
        .try_init()
        .map_err(|error| AppError::Internal(format!("logging already initialised: {error}")))?;

    tracing::info!(directory = %directory.display(), "file logging enabled");
    Ok(())
}
