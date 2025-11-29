//! Receive files via iroh-blobs (based on sendme)

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use fs2::available_space;
use iroh::discovery::dns::DnsDiscovery;
use iroh::Endpoint;
use iroh::Watcher;
use iroh_blobs::api::blobs::{ExportMode, ExportOptions, ExportProgressItem};
use iroh_blobs::api::remote::GetProgressItem;
use iroh_blobs::api::Store;
use iroh_blobs::format::collection::Collection;
use iroh_blobs::get::request::get_hash_seq_and_sizes;
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::ticket::BlobTicket;
use n0_future::StreamExt;
use std::ops::Deref;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{ConflictResolution, SpeedTracker, TransferId, TransferProgress};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const MIN_FREE_SPACE: u64 = 1024 * 1024 * 1024; // 1 GB buffer

/// Each ReceiveTask creates an isolated Endpoint for privacy (fresh NodeID per receive)
pub struct ReceiveTask {
    id: TransferId,
    ticket: BlobTicket,
    output_dir: PathBuf,
    store: Arc<FsStore>,
    progress_tx: mpsc::Sender<TransferProgress>,
    resolution_rx: mpsc::Receiver<ConflictResolution>,
    cancel_token: CancellationToken,
    exported_files: Vec<PathBuf>, // For cleanup on cancel
}

pub struct ConflictResolver {
    pub tx: mpsc::Sender<ConflictResolution>,
}

impl ReceiveTask {
    pub fn new(
        id: impl Into<TransferId>,
        ticket: BlobTicket,
        output_dir: PathBuf,
        store: Arc<FsStore>,
        progress_tx: mpsc::Sender<TransferProgress>,
        cancel_token: CancellationToken,
    ) -> (Self, ConflictResolver) {
        let (resolution_tx, resolution_rx) = mpsc::channel(1);
        (
            Self {
                id: id.into(),
                ticket,
                output_dir,
                store,
                progress_tx,
                resolution_rx,
                cancel_token,
                exported_files: Vec::new(),
            },
            ConflictResolver { tx: resolution_tx },
        )
    }

    async fn cleanup_exported_files(&self) {
        for file in &self.exported_files {
            if file.exists() {
                tracing::info!("Cleaning up partial file: {}", file.display());
                if let Err(e) = tokio::fs::remove_file(file).await {
                    tracing::warn!("Failed to clean up {}: {}", file.display(), e);
                }
            }
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let start = Instant::now();

        // Phase 1: Preparing - creating endpoint
        let _ = self
            .progress_tx
            .send(TransferProgress::Preparing {
                id: self.id.clone(),
                status: "Creating endpoint...".to_string(),
            })
            .await;

        // Create isolated endpoint for privacy (fresh NodeID per receive)
        let endpoint = Endpoint::builder()
            .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
            .discovery(DnsDiscovery::n0_dns())
            .bind()
            .await
            .context("failed to create endpoint for receive")?;

        // Phase 2: Connecting - attempting to connect to sender
        let _ = self
            .progress_tx
            .send(TransferProgress::Connecting {
                id: self.id.clone(),
            })
            .await;

        let addr = self.ticket.addr().clone();
        tracing::info!(
            "Connecting to addr: {:?}, hash: {}",
            addr,
            self.ticket.hash()
        );
        let connection = tokio::time::timeout(
            CONNECT_TIMEOUT,
            endpoint.connect(addr.clone(), iroh_blobs::protocol::ALPN),
        )
        .await
        .context("connection timed out after 30 seconds")?
        .context(format!("failed to connect to sender at {:?}", addr))?;

        // Phase 3: Connected - determine connection type
        tracing::info!("Connected successfully!");
        let remote_id = connection.remote_id();
        let is_relay = if let Some(mut conn_type_watcher) = endpoint.conn_type(remote_id) {
            let conn_type = conn_type_watcher.get();
            tracing::info!("Connection type: {:?}", conn_type);
            matches!(conn_type, iroh::endpoint::ConnectionType::Relay(_))
        } else {
            tracing::warn!("Could not get connection type, assuming relay");
            true // Conservative: assume relay if unknown
        };

        let _ = self
            .progress_tx
            .send(TransferProgress::Connected {
                id: self.id.clone(),
                is_relay,
            })
            .await;

        let hash_and_format = self.ticket.hash_and_format();
        tracing::info!(
            "Getting hash_seq and sizes for hash: {}",
            hash_and_format.hash
        );
        let (_hash_seq, sizes) =
            get_hash_seq_and_sizes(&connection, &hash_and_format.hash, 1024 * 1024 * 32, None)
                .await
                .map_err(|e| anyhow::anyhow!("failed to get sizes: {}", e))?;

        tracing::info!("Got sizes: {:?}", sizes);
        let total_size: u64 = sizes.iter().copied().sum();
        let payload_size: u64 = sizes.iter().skip(1).copied().sum(); // Skip metadata
        let total_files = sizes.len().saturating_sub(1);
        tracing::info!(
            "total_size: {}, payload_size: {}, total_files: {}",
            total_size,
            payload_size,
            total_files
        );

        let free_space =
            available_space(&self.output_dir).context("failed to check available disk space")?;
        let required_space = payload_size + MIN_FREE_SPACE;
        if free_space < required_space {
            let err_msg = format!(
                "insufficient disk space: need {} (+ {} buffer) but only {} available",
                humansize::format_size(payload_size, humansize::BINARY),
                humansize::format_size(MIN_FREE_SPACE, humansize::BINARY),
                humansize::format_size(free_space, humansize::BINARY)
            );
            if let Err(e) = self
                .progress_tx
                .send(TransferProgress::Failed {
                    id: self.id.clone(),
                    error: err_msg.clone(),
                })
                .await
            {
                tracing::warn!("Failed to send Failed progress: {}", e);
            }
            anyhow::bail!(err_msg);
        }
        tracing::info!(
            "Disk space OK: {} available, {} required",
            humansize::format_size(free_space, humansize::BINARY),
            humansize::format_size(required_space, humansize::BINARY)
        );

        // Check local availability - we need the collection metadata to check conflicts
        let local = self.store.remote().local(hash_and_format).await?;

        // We need the collection to check for conflicts
        // First, ensure we have at least the metadata downloaded
        let collection = if local.is_complete() {
            // Already have everything locally
            let store: &Store = self.store.deref();
            Collection::load(hash_and_format.hash, store).await?
        } else {
            // Download just enough to get the collection metadata
            // The collection metadata is the first blob, which we got with get_hash_seq_and_sizes
            // We need to do a partial download to get the collection
            let _local_size = local.local_bytes();
            let get = self
                .store
                .remote()
                .execute_get(connection.clone(), local.missing());

            let mut stream = get.stream();
            while let Some(item) = stream.next().await {
                match item {
                    GetProgressItem::Progress(_) => {}
                    GetProgressItem::Done(_) => break,
                    GetProgressItem::Error(cause) => {
                        anyhow::bail!("download error while getting metadata: {}", cause);
                    }
                }
            }

            let store: &Store = self.store.deref();
            Collection::load(hash_and_format.hash, store).await?
        };

        // Send file list for history
        let files: Vec<(String, u64)> = collection
            .iter()
            .zip(sizes.iter().skip(1)) // Skip metadata size
            .map(|((name, _hash), &size)| {
                // Extract just the filename from path
                let filename = name.rsplit('/').next().unwrap_or(name).to_string();
                (filename, size)
            })
            .collect();

        if !files.is_empty() {
            if let Err(e) = self
                .progress_tx
                .send(TransferProgress::FileList {
                    id: self.id.clone(),
                    files,
                })
                .await
            {
                tracing::warn!("Failed to send FileList progress: {}", e);
            }
        }

        let conflicts = self.check_conflicts(&collection)?;
        let resolution = if !conflicts.is_empty() {
            tracing::info!("Found {} file conflicts", conflicts.len());
            if let Err(e) = self
                .progress_tx
                .send(TransferProgress::FileConflicts {
                    id: self.id.clone(),
                    conflicts: conflicts.clone(),
                    total_bytes: payload_size,
                })
                .await
            {
                tracing::warn!("Failed to send FileConflicts progress: {}", e);
            }

            tracing::info!("Waiting for conflict resolution...");
            match self.resolution_rx.recv().await {
                Some(res) => {
                    tracing::info!("Got resolution: {:?}", res);
                    res
                }
                None => {
                    tracing::info!("Resolution channel closed, cancelling");
                    return Ok(());
                }
            }
        } else {
            ConflictResolution::Rename
        };

        if matches!(resolution, ConflictResolution::Cancel) {
            if let Err(e) = self
                .progress_tx
                .send(TransferProgress::Cancelled {
                    id: self.id.clone(),
                })
                .await
            {
                tracing::warn!("Failed to send Cancelled progress: {}", e);
            }
            return Ok(());
        }

        let name = if total_files == 1 {
            collection
                .iter()
                .next()
                .map(|(name, _)| name.to_string())
                .unwrap_or_else(|| "file".to_string())
        } else {
            let first_name = collection
                .iter()
                .next()
                .map(|(name, _)| name.to_string())
                .unwrap_or_default();
            if let Some(folder) = first_name.split('/').next() {
                if collection.iter().all(|(name, _)| name.starts_with(folder)) {
                    folder.to_string()
                } else {
                    format!("{} files", total_files)
                }
            } else {
                format!("{} files", total_files)
            }
        };

        if let Err(e) = self
            .progress_tx
            .send(TransferProgress::Started {
                id: self.id.clone(),
                name,
                total_bytes: payload_size,
            })
            .await
        {
            tracing::warn!("Failed to send Started progress: {}", e);
        }

        let completed = self.export_collection(&collection, &resolution).await?;

        if completed {
            let duration = start.elapsed();
            if let Err(e) = self
                .progress_tx
                .send(TransferProgress::Completed {
                    id: self.id.clone(),
                    total_bytes: payload_size,
                    duration_secs: duration.as_secs_f64(),
                })
                .await
            {
                tracing::warn!("Failed to send Completed progress: {}", e);
            }
        } else {
            self.cleanup_exported_files().await;
            if let Err(e) = self
                .progress_tx
                .send(TransferProgress::Cancelled {
                    id: self.id.clone(),
                })
                .await
            {
                tracing::warn!("Failed to send Cancelled progress: {}", e);
            }
        }

        Ok(())
    }

    fn check_conflicts(&self, collection: &Collection) -> Result<Vec<(String, PathBuf)>> {
        let mut conflicts = Vec::new();

        for (name, _hash) in collection.iter() {
            let target = self.get_export_path(name)?;
            if target.exists() {
                conflicts.push((name.to_string(), target));
            }
        }

        Ok(conflicts)
    }

    /// Returns true if completed, false if cancelled
    async fn export_collection(
        &mut self,
        collection: &Collection,
        resolution: &ConflictResolution,
    ) -> Result<bool> {
        let mut speed_tracker = SpeedTracker::default_window();
        let mut current_file_base: u64 = 0;

        for (name, hash) in collection.iter() {
            if self.cancel_token.is_cancelled() {
                tracing::info!("Receive task {} cancelled during export", self.id);
                return Ok(false);
            }

            let base_target = self.get_export_path(name)?;
            if let Some(parent) = base_target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let target = if base_target.exists() {
                match resolution {
                    ConflictResolution::Rename => Self::find_available_path(&base_target),
                    ConflictResolution::Overwrite => {
                        if let Err(e) = tokio::fs::remove_file(&base_target).await {
                            tracing::warn!("Failed to remove existing file for overwrite: {}", e);
                        }
                        base_target
                    }
                    ConflictResolution::Skip => {
                        tracing::info!("Skipping existing file: {}", base_target.display());
                        continue;
                    }
                    ConflictResolution::Cancel => return Ok(false),
                }
            } else {
                base_target
            };

            self.exported_files.push(target.clone());
            let mut stream = self
                .store
                .export_with_opts(ExportOptions {
                    hash: *hash,
                    target: target.clone(),
                    mode: ExportMode::TryReference,
                })
                .stream()
                .await;

            let mut current_file_size: u64 = 0;
            let mut file_complete = false;

            while let Some(item) = stream.next().await {
                if self.cancel_token.is_cancelled() {
                    tracing::info!("Receive task {} cancelled during file export", self.id);
                    return Ok(false);
                }

                match item {
                    ExportProgressItem::Size(size) => {
                        current_file_size = size;
                    }
                    ExportProgressItem::CopyProgress(offset) => {
                        let cumulative_bytes = current_file_base + offset;
                        speed_tracker.add_sample(cumulative_bytes);
                        if let Err(e) = self.progress_tx.try_send(TransferProgress::Progress {
                            id: self.id.clone(),
                            transferred_bytes: cumulative_bytes,
                            speed_bps: speed_tracker.speed_bps(),
                        }) {
                            tracing::debug!("Progress channel full, skipping update: {}", e);
                        }
                    }
                    ExportProgressItem::Done => {
                        current_file_base += current_file_size;
                        file_complete = true;
                        break;
                    }
                    ExportProgressItem::Error(cause) => {
                        anyhow::bail!("error exporting {}: {}", name, cause);
                    }
                }
            }

            if file_complete {
                self.exported_files.pop();
            }
        }

        Ok(true)
    }

    /// Security: Validates path to prevent traversal attacks
    fn get_export_path(&self, name: &str) -> Result<PathBuf> {
        let parts: Vec<&str> = name.split('/').collect();
        let mut path = self.output_dir.clone();

        for part in parts {
            anyhow::ensure!(!part.is_empty(), "empty path component in: {}", name);
            anyhow::ensure!(
                part != ".." && part != ".",
                "path traversal attempt: {}",
                part
            );
            anyhow::ensure!(
                !part.contains('/') && !part.contains('\\'),
                "path separator in component: {}",
                part
            );
            path.push(part);
        }

        // Catch edge cases like unicode normalization attacks
        anyhow::ensure!(
            path.starts_with(&self.output_dir),
            "path escaped output directory: {}",
            path.display()
        );

        Ok(path)
    }

    /// Find available path by appending (1), (2), etc.
    fn find_available_path(base: &std::path::Path) -> PathBuf {
        if !base.exists() {
            return base.to_path_buf();
        }

        let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let ext = base.extension().and_then(|e| e.to_str());
        let parent = base.parent();

        for i in 1..1000 {
            let new_name = match ext {
                Some(e) => format!("{} ({}).{}", stem, i, e),
                None => format!("{} ({})", stem, i),
            };
            let new_path = match parent {
                Some(p) => p.join(&new_name),
                None => PathBuf::from(&new_name),
            };
            if !new_path.exists() {
                tracing::info!("Auto-renamed to: {}", new_path.display());
                return new_path;
            }
        }

        base.to_path_buf()
    }
}
