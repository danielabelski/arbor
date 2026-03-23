use {
    std::{
        collections::VecDeque,
        fmt,
        fs::OpenOptions,
        io::Write,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
    },
    tracing::{Event, Subscriber, field::Visit},
    tracing_subscriber::{Layer, layer::Context, registry::LookupSpan},
};

const MAX_LOG_ENTRIES: usize = 10_000;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: tracing::Level,
    pub target: String,
    pub message: String,
    pub fields: Vec<(String, String)>,
}

struct Inner {
    entries: VecDeque<LogEntry>,
    generation: u64,
    persistent_log_path: Option<PathBuf>,
}

#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<Inner>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                entries: VecDeque::with_capacity(MAX_LOG_ENTRIES),
                generation: 0,
                persistent_log_path: None,
            })),
        }
    }

    pub fn generation(&self) -> u64 {
        self.inner.lock().map(|inner| inner.generation).unwrap_or(0)
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.inner
            .lock()
            .map(|inner| inner.entries.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn push(&self, entry: LogEntry) {
        let persistent_log_path = if let Ok(mut inner) = self.inner.lock() {
            if inner.entries.len() >= MAX_LOG_ENTRIES {
                inner.entries.pop_front();
            }
            inner.entries.push_back(entry.clone());
            inner.generation += 1;
            inner.persistent_log_path.clone()
        } else {
            None
        };

        if let Some(path) = persistent_log_path {
            append_persistent_log_entry(&path, &entry);
        }
    }

    pub fn set_persistent_log_path(&self, path: PathBuf) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.persistent_log_path = Some(path);
        }
    }
}

pub struct InMemoryLayer {
    buffer: LogBuffer,
}

impl InMemoryLayer {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for InMemoryLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: SystemTime::now(),
            level: *metadata.level(),
            target: metadata.target().to_owned(),
            message: visitor.message,
            fields: visitor.fields,
        };

        self.buffer.push(entry);
    }
}

#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields
                .push((field.name().to_owned(), format!("{value:?}")));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        } else {
            self.fields
                .push((field.name().to_owned(), value.to_owned()));
        }
    }
}

fn append_persistent_log_entry(path: &Path, entry: &LogEntry) {
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };

    let timestamp_ms = entry
        .timestamp
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);

    let fields = if entry.fields.is_empty() {
        String::new()
    } else {
        format!(
            " {}",
            entry
                .fields
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };

    let line = format!("{timestamp_ms} {} {}", entry.level, entry.target);
    let line = if entry.message.is_empty() {
        line
    } else {
        format!("{line} {}", entry.message)
    };
    let line = if fields.is_empty() {
        line
    } else {
        format!("{line} {fields}")
    };

    let _ = writeln!(file, "{line}");
}
