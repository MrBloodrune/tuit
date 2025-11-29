//! Send files via iroh-blobs (based on sendme)

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use futures::stream;
use iroh::discovery::dns::DnsDiscovery;
use iroh::protocol::Router;
use iroh::Endpoint;
use iroh_blobs::api::blobs::{AddPathOptions, AddProgressItem, ImportMode};
use iroh_blobs::api::TempTag;
use iroh_blobs::format::collection::Collection;
use iroh_blobs::provider::events::{
    ConnectMode, EventMask, EventSender, ProviderMessage, RequestMode, RequestUpdate,
};
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::{BlobFormat, BlobsProtocol};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use walkdir::WalkDir;

use super::{SpeedTracker, TransferId, TransferProgress};

const PARALLEL_IMPORTS: usize = 4;

#[derive(Debug, Clone, Default)]
pub struct SendOptions {
    pub follow_symlinks: bool, // Default false for security
}

/// Each SendTask creates an isolated Endpoint (avoids concurrent send conflicts)
pub struct SendTask {
    id: TransferId,
    paths: Vec<PathBuf>,
    store: Arc<FsStore>,
    progress_tx: mpsc::Sender<TransferProgress>,
    options: SendOptions,
    cancel_token: CancellationToken,
}

impl SendTask {
    pub fn new(
        id: impl Into<TransferId>,
        paths: Vec<PathBuf>,
        store: Arc<FsStore>,
        progress_tx: mpsc::Sender<TransferProgress>,
        options: SendOptions,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            id: id.into(),
            paths,
            store,
            progress_tx,
            options,
            cancel_token,
        }
    }

    pub async fn run(self) -> Result<()> {
        let start = Instant::now();

        // Phase 1: Preparing - importing files
        let _ = self
            .progress_tx
            .send(TransferProgress::Preparing {
                id: self.id.clone(),
                status: "Importing files...".to_string(),
            })
            .await;

        let (temp_tag, total_size, collection, name) = self.import_files().await?;

        // Phase 2: Preparing - creating endpoint
        let _ = self
            .progress_tx
            .send(TransferProgress::Preparing {
                id: self.id.clone(),
                status: "Creating endpoint...".to_string(),
            })
            .await;

        let endpoint = Endpoint::builder()
            .alpns(vec![iroh_blobs::ALPN.to_vec()])
            .discovery(DnsDiscovery::n0_dns())
            .bind()
            .await
            .context("failed to create endpoint for send")?;

        let (event_tx, mut event_rx) = mpsc::channel::<ProviderMessage>(32);
        let blobs = BlobsProtocol::new(
            &self.store,
            Some(EventSender::new(
                event_tx,
                EventMask {
                    connected: ConnectMode::Notify,
                    get: RequestMode::NotifyLog,
                    ..EventMask::DEFAULT
                },
            )),
        );

        let router = Router::builder(endpoint)
            .accept(iroh_blobs::ALPN, blobs)
            .spawn();

        // Phase 3: Preparing - connecting to relay network
        let _ = self
            .progress_tx
            .send(TransferProgress::Preparing {
                id: self.id.clone(),
                status: "Joining relay network...".to_string(),
            })
            .await;

        let ep = router.endpoint();
        match tokio::time::timeout(std::time::Duration::from_secs(30), ep.online()).await {
            Ok(_) => tracing::info!("Endpoint is online"),
            Err(_) => tracing::warn!("Timeout waiting for endpoint to come online"),
        }

        // Phase 4: Generate ticket
        let addr = router.endpoint().addr();
        tracing::info!("Creating ticket with addr: {:?}", addr);
        let ticket = BlobTicket::new(addr, temp_tag.hash(), BlobFormat::HashSeq);
        tracing::info!("Ticket: {}", ticket);

        let _ = self
            .progress_tx
            .send(TransferProgress::TicketReady {
                id: self.id.clone(),
                ticket: ticket.to_string(),
            })
            .await;

        // Phase 5: Started - ready with file info, now waiting for peer
        let _ = self
            .progress_tx
            .send(TransferProgress::Started {
                id: self.id.clone(),
                name: name.clone(),
                total_bytes: total_size,
            })
            .await;

        // Phase 6: Connecting - waiting for peer to connect
        let _ = self
            .progress_tx
            .send(TransferProgress::Connecting {
                id: self.id.clone(),
            })
            .await;

        // (blob_index, offset, is_complete) progress channel
        let (progress_update_tx, mut progress_update_rx) = mpsc::channel::<(u64, u64, bool)>(32);
        let mut blob_progress: std::collections::HashMap<u64, u64> =
            std::collections::HashMap::new();
        let mut completed_blobs: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut had_connection = false;
        let mut connection_reported = false;
        let mut speed_tracker = SpeedTracker::default_window();
        let expected_blob_count = collection.len() + 1; // metadata + files

        let mut cancelled = false;
        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    tracing::info!("Send task {} cancelled", self.id);
                    cancelled = true;
                    break;
                }
                msg = event_rx.recv() => {
                    match msg {
                        Some(ProviderMessage::ClientConnectedNotify(msg)) => {
                            tracing::info!("Client connected: connection_id={}", msg.inner.connection_id);
                            had_connection = true;

                            // Report connected status only once
                            // Note: Sender-side can't easily determine P2P vs Relay from ClientConnectedNotify
                            // The receiver determines this more accurately. Sender reports as P2P optimistically.
                            if !connection_reported {
                                connection_reported = true;
                                let _ = self.progress_tx
                                    .send(TransferProgress::Connected {
                                        id: self.id.clone(),
                                        is_relay: false, // Sender can't determine; receiver will have accurate info
                                    })
                                    .await;
                            }
                        }
                        Some(ProviderMessage::GetRequestReceivedNotify(msg)) => {
                            tracing::info!("Get request: connection_id={}", msg.inner.connection_id);
                            let progress_tx = progress_update_tx.clone();
                            tokio::spawn(async move {
                                let mut rx = msg.rx;
                                let mut current_blob_index = 0u64;
                                let mut current_blob_size = 0u64;
                                while let Ok(Some(update)) = rx.recv().await {
                                    match update {
                                        RequestUpdate::Started(s) => {
                                            current_blob_index = s.index;
                                            current_blob_size = s.size;
                                            tracing::debug!("Blob {} transfer started: size={}", s.index, s.size);
                                        }
                                        RequestUpdate::Progress(p) => {
                                            let _ = progress_tx.send((current_blob_index, p.end_offset, false)).await;
                                        }
                                        RequestUpdate::Completed(_) => {
                                            tracing::debug!("Blob {} transfer completed", current_blob_index);
                                            let _ = progress_tx.send((current_blob_index, current_blob_size, true)).await;
                                        }
                                        RequestUpdate::Aborted(_) => {
                                            tracing::warn!("Blob {} transfer aborted", current_blob_index);
                                        }
                                    }
                                }
                            });
                        }
                        Some(ProviderMessage::ConnectionClosed(msg)) => {
                            tracing::info!("Connection closed: connection_id={}", msg.inner.connection_id);
                            if completed_blobs.len() >= expected_blob_count || had_connection {
                                break;
                            }
                        }
                        Some(_) => {
                            tracing::debug!("Provider event received");
                        }
                        None => break,
                    }
                }
                Some((blob_index, offset, is_complete)) = progress_update_rx.recv() => {
                    blob_progress.insert(blob_index, offset);
                    if is_complete {
                        completed_blobs.insert(blob_index);
                        tracing::info!("Blob {} complete ({}/{})", blob_index, completed_blobs.len(), expected_blob_count);
                        if completed_blobs.len() >= expected_blob_count {
                            tracing::info!("All {} blobs transferred successfully", expected_blob_count);
                            break;
                        }
                    }

                    let total_transferred: u64 = blob_progress.values().sum();
                    speed_tracker.add_sample(total_transferred);
                    let speed_bps = speed_tracker.speed_bps();
                    if let Err(e) = self.progress_tx.try_send(TransferProgress::Progress {
                        id: self.id.clone(),
                        transferred_bytes: total_transferred,
                        speed_bps,
                    }) {
                        tracing::debug!("Progress channel full, skipping update: {}", e);
                    }

                }
            }
        }

        drop(temp_tag);
        drop(router);

        if cancelled {
            if let Err(e) = self
                .progress_tx
                .send(TransferProgress::Cancelled {
                    id: self.id.clone(),
                })
                .await
            {
                tracing::warn!("Failed to send Cancelled progress: {}", e);
            }
        } else {
            let duration = start.elapsed();
            if let Err(e) = self
                .progress_tx
                .send(TransferProgress::Completed {
                    id: self.id.clone(),
                    total_bytes: total_size,
                    duration_secs: duration.as_secs_f64(),
                })
                .await
            {
                tracing::warn!("Failed to send Completed progress: {}", e);
            }
        }

        Ok(())
    }

    async fn import_files(&self) -> Result<(TempTag, u64, Collection, String)> {
        let mut all_files: Vec<(String, PathBuf)> = Vec::new();

        for path in &self.paths {
            let path = path.canonicalize()?;
            anyhow::ensure!(path.exists(), "path {} does not exist", path.display());
            let root = path.parent().context("cannot get parent directory")?;

            // Security: don't follow symlinks by default
            for entry in WalkDir::new(&path).follow_links(self.options.follow_symlinks) {
                let entry = entry?;
                if !self.options.follow_symlinks && entry.file_type().is_symlink() {
                    tracing::warn!("skipping symlink: {}", entry.path().display());
                    continue;
                }

                if !entry.file_type().is_file() {
                    continue;
                }
                let file_path = entry.into_path();
                let relative = file_path.strip_prefix(root)?;
                let name = canonicalized_path_to_string(relative, true)?;
                all_files.push((name, file_path));
            }
        }

        let name = if self.paths.len() == 1 {
            self.paths[0]
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "files".to_string())
        } else {
            format!("{} items", self.paths.len())
        };

        let store = self.store.clone();

        let import_results: Vec<Result<(String, TempTag, u64)>> =
            futures::stream::StreamExt::collect(futures::stream::StreamExt::buffer_unordered(
                futures::stream::StreamExt::map(
                    stream::iter(all_files),
                    |(file_name, file_path)| {
                        let store = store.clone();
                        async move { Self::import_single_file(&store, file_name, file_path).await }
                    },
                ),
                PARALLEL_IMPORTS,
            ))
            .await;

        let mut names_and_tags: Vec<(String, TempTag, u64)> =
            Vec::with_capacity(import_results.len());
        for result in import_results {
            names_and_tags.push(result?);
        }
        names_and_tags.sort_by(|(a, _, _), (b, _, _)| a.cmp(b));
        let total_size: u64 = names_and_tags.iter().map(|(_, _, size)| *size).sum();

        let (collection, tags): (Collection, Vec<_>) = names_and_tags
            .into_iter()
            .map(|(name, tag, _)| ((name, tag.hash()), tag))
            .unzip();

        let temp_tag = collection.clone().store(&store).await?;
        drop(tags); // Data now protected by collection

        Ok((temp_tag, total_size, collection, name))
    }

    async fn import_single_file(
        store: &FsStore,
        file_name: String,
        file_path: PathBuf,
    ) -> Result<(String, TempTag, u64)> {
        let import = store.add_path_with_opts(AddPathOptions {
            path: file_path.clone(),
            format: iroh_blobs::BlobFormat::Raw,
            mode: ImportMode::Copy,
        });

        let mut item_size = 0u64;
        let mut temp_tag = None;

        let mut stream = std::pin::pin!(import.stream().await);
        while let Some(item) = n0_future::StreamExt::next(&mut stream).await {
            match item {
                AddProgressItem::Size(size) => {
                    item_size = size;
                }
                AddProgressItem::Done(tag) => {
                    temp_tag = Some(tag);
                }
                AddProgressItem::CopyProgress(_)
                | AddProgressItem::OutboardProgress(_)
                | AddProgressItem::CopyDone => {}
                AddProgressItem::Error(e) => {
                    anyhow::bail!("import error for {}: {}", file_path.display(), e);
                }
            }
        }

        let temp_tag = temp_tag.context(format!("no tag for {}", file_path.display()))?;
        Ok((file_name, temp_tag, item_size))
    }
}

/// Path to string with forward slashes.
/// Security: Rejects `.`, `..`, path separators in components, root dirs (if relative required).
fn canonicalized_path_to_string(path: impl AsRef<Path>, must_be_relative: bool) -> Result<String> {
    let mut path_str = String::new();
    let mut parts: Vec<&str> = Vec::new();

    for component in path.as_ref().components() {
        match component {
            Component::Normal(x) => {
                let s = x.to_str().context("non-UTF8 path component")?;
                anyhow::ensure!(
                    !s.contains('/') && !s.contains('\\'),
                    "path separator in component: {}",
                    s
                );
                anyhow::ensure!(!s.is_empty(), "empty path component");

                parts.push(s);
            }
            Component::RootDir => {
                anyhow::ensure!(!must_be_relative, "absolute path not allowed");
                path_str.push('/');
            }
            Component::ParentDir => anyhow::bail!("parent directory (..) not allowed"),
            Component::CurDir => {} // Skip `.`
            Component::Prefix(_) => {
                anyhow::ensure!(!must_be_relative, "absolute path not allowed");
            }
        }
    }

    path_str.push_str(&parts.join("/"));
    Ok(path_str)
}
