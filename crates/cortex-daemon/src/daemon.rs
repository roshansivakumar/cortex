use std::sync::Arc;
use std::time::Instant;

use tokio_util::sync::CancellationToken;

use cortex_core::config::CortexConfig;
use cortex_core::traits::{Chunker, Embedder, Plugin};
use cortex_embed::embedder::OnnxEmbedder;
use cortex_embed::model::ensure_model;
use cortex_plugin_fs::chunker::ParagraphChunker;
use cortex_plugin_fs::watcher::FsWatcherPlugin;
use cortex_store::connection::SqliteStore;

use crate::ingestion;
use crate::lifecycle;

pub struct Daemon {
    config: CortexConfig,
}

impl Daemon {
    pub fn new(config: CortexConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> cortex_core::error::Result<()> {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| {
                        tracing_subscriber::EnvFilter::new(&self.config.general.log_level)
                    }),
            )
            .init();

        tracing::info!("Cortex daemon starting...");

        // Ensure data directory exists
        std::fs::create_dir_all(&self.config.storage.data_dir)?;

        // Write PID file
        lifecycle::write_pid_file(&self.config.pid_path())?;

        // Ensure model is downloaded
        let _model_paths = ensure_model(
            &self.config.model_dir(),
            &self.config.embedding.model,
        )?;
        tracing::info!("Model ready: {}", self.config.embedding.model);

        // Initialize store
        let store = Arc::new(SqliteStore::open(&self.config.db_path())?);
        tracing::info!("Database opened: {}", self.config.db_path().display());

        // Initialize embedder
        let embedder: Arc<dyn Embedder> =
            Arc::new(OnnxEmbedder::new(&self.config.model_dir())?);
        tracing::info!("Embedder loaded ({}-dim)", embedder.dimension());

        // Initialize chunker
        let chunker: Arc<dyn Chunker> = Arc::new(ParagraphChunker::new(
            self.config.chunking.max_tokens,
            self.config.chunking.overlap_tokens,
        ));

        // Set up event channel
        let (sender, receiver) = crossbeam_channel::unbounded();

        // Cancellation token for graceful shutdown
        let cancel = CancellationToken::new();

        // Build and start API server FIRST so CLI can connect immediately
        let app_state = cortex_api::server::AppState {
            store: store.clone(),
            embedder: embedder.clone(),
            config: Arc::new(self.config.clone()),
            start_time: Instant::now(),
            ingest_sender: sender.clone(),
        };

        let app = cortex_api::server::build_router(app_state);

        // Bind TCP listener (default port 9723 if none configured)
        let port = self.config.api.tcp_port.unwrap_or(9723);
        let tcp_listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .map_err(cortex_core::error::CortexError::Io)?;
        tracing::info!("API listening on: http://127.0.0.1:{port}");

        // Write socket info so CLI knows where to connect
        let socket_path = self.config.socket_path();
        std::fs::write(&socket_path, port.to_string())?;

        tracing::info!("Cortex daemon ready. Starting indexing...");

        // Start the FS plugin (does initial scan, then watches for changes)
        let mut fs_plugin = FsWatcherPlugin::new(
            self.config.watch.paths.clone(),
            self.config.watch.extensions.clone(),
            self.config.watch.ignore_patterns.clone(),
        );
        fs_plugin.start(sender.clone())?;
        let plugin: Arc<dyn Plugin> = Arc::new(FsWatcherPluginHandle {
            name: "fs-watcher".to_string(),
        });

        // Start ingestion pipeline in background
        let ingestion_cancel = cancel.clone();
        let ingestion_store = store.clone();
        let ingestion_embedder = embedder.clone();
        let ingestion_chunker = chunker.clone();
        let ingestion_plugin = plugin.clone();

        let _ingestion_handle = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(ingestion::ingestion_loop(
                receiver,
                ingestion_store,
                ingestion_embedder,
                ingestion_chunker,
                ingestion_plugin,
                ingestion_cancel,
            ));
        });

        // Wait for shutdown signal
        let shutdown_cancel = cancel.clone();
        tokio::select! {
            result = axum::serve(tcp_listener, app) => {
                if let Err(e) = result {
                    tracing::error!("Server error: {e}");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C, shutting down...");
            }
            _ = shutdown_cancel.cancelled() => {
                tracing::info!("Shutdown requested.");
            }
        }

        // Graceful shutdown
        cancel.cancel();
        fs_plugin.stop()?;

        // Clean up
        lifecycle::remove_pid_file(&self.config.pid_path());
        lifecycle::remove_socket_file(&self.config.socket_path());

        tracing::info!("Cortex daemon stopped.");
        Ok(())
    }
}

/// A lightweight handle that implements Plugin for the ingestion pipeline,
/// allowing it to read documents without owning the watcher.
struct FsWatcherPluginHandle {
    name: String,
}

impl cortex_core::traits::Plugin for FsWatcherPluginHandle {
    fn name(&self) -> &str {
        &self.name
    }

    fn start(
        &mut self,
        _sender: crossbeam_channel::Sender<cortex_core::document::DocumentEvent>,
    ) -> cortex_core::error::Result<()> {
        Ok(()) // This handle doesn't watch, it just reads
    }

    fn stop(&mut self) -> cortex_core::error::Result<()> {
        Ok(())
    }

    fn read_document(
        &self,
        path: &std::path::Path,
    ) -> cortex_core::error::Result<cortex_core::document::Document> {
        cortex_plugin_fs::reader::read_file(path, &self.name)
    }
}
