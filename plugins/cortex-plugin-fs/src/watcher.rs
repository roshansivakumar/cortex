use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::Sender;
use notify::{Event, EventKind, RecursiveMode, Watcher};

use cortex_core::document::{Document, DocumentEvent};
use cortex_core::error::{CortexError, Result};
use cortex_core::traits::Plugin;

use crate::reader;

pub struct FsWatcherPlugin {
    watch_paths: Vec<PathBuf>,
    extensions: Vec<String>,
    ignore_patterns: Vec<String>,
    running: Arc<AtomicBool>,
    watcher: Option<notify::RecommendedWatcher>,
}

impl FsWatcherPlugin {
    pub fn new(
        watch_paths: Vec<PathBuf>,
        extensions: Vec<String>,
        ignore_patterns: Vec<String>,
    ) -> Self {
        Self {
            watch_paths,
            extensions,
            ignore_patterns,
            running: Arc::new(AtomicBool::new(false)),
            watcher: None,
        }
    }

    /// Perform an initial scan of all watched directories and emit Created events.
    fn initial_scan(&self, sender: &Sender<DocumentEvent>) -> Result<()> {
        for watch_path in &self.watch_paths {
            if !watch_path.exists() {
                tracing::warn!("Watch path does not exist: {}", watch_path.display());
                continue;
            }
            self.scan_directory(watch_path, sender)?;
        }
        Ok(())
    }

    fn scan_directory(&self, dir: &Path, sender: &Sender<DocumentEvent>) -> Result<()> {
        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if reader::should_ignore(&path, &self.ignore_patterns) {
                continue;
            }

            if path.is_dir() {
                self.scan_directory(&path, sender)?;
            } else if reader::has_allowed_extension(&path, &self.extensions) {
                let _ = sender.send(DocumentEvent::Created(path));
            }
        }
        Ok(())
    }
}

impl Plugin for FsWatcherPlugin {
    fn name(&self) -> &str {
        "fs-watcher"
    }

    fn start(&mut self, sender: Sender<DocumentEvent>) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);

        // Initial scan
        tracing::info!("Starting initial scan of watched directories...");
        self.initial_scan(&sender)?;
        tracing::info!("Initial scan complete.");

        // Set up file watcher
        let extensions = self.extensions.clone();
        let ignore_patterns = self.ignore_patterns.clone();
        let running = self.running.clone();

        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            if !running.load(Ordering::SeqCst) {
                return;
            }

            match res {
                Ok(event) => {
                    for path in event.paths {
                        if reader::should_ignore(&path, &ignore_patterns) {
                            continue;
                        }
                        if !reader::has_allowed_extension(&path, &extensions) {
                            continue;
                        }

                        let doc_event = match event.kind {
                            EventKind::Create(_) => DocumentEvent::Created(path),
                            EventKind::Modify(_) => DocumentEvent::Modified(path),
                            EventKind::Remove(_) => DocumentEvent::Deleted(path),
                            _ => continue,
                        };

                        if sender.send(doc_event).is_err() {
                            return; // Channel closed, daemon is shutting down
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("File watcher error: {e}");
                }
            }
        })
        .map_err(|e| CortexError::Plugin(format!("Failed to create watcher: {e}")))?;

        // Watch all configured paths
        for path in &self.watch_paths {
            if path.exists() {
                watcher
                    .watch(path, RecursiveMode::Recursive)
                    .map_err(|e| {
                        CortexError::Plugin(format!(
                            "Failed to watch {}: {e}",
                            path.display()
                        ))
                    })?;
                tracing::info!("Watching: {}", path.display());
            }
        }

        self.watcher = Some(watcher);
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.watcher = None;
        tracing::info!("FS watcher stopped.");
        Ok(())
    }

    fn read_document(&self, path: &Path) -> Result<Document> {
        reader::read_file(path, self.name())
    }
}
