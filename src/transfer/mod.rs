//! Transfer management via iroh-blobs

mod receiver;
mod sender;

pub use receiver::{ConflictResolver, ReceiveTask};
pub use sender::{SendOptions, SendTask};

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub type TransferId = Arc<str>;

use anyhow::Result;
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::ticket::BlobTicket;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Rolling window speed tracker
#[derive(Debug)]
pub struct SpeedTracker {
    samples: VecDeque<(Instant, u64)>,
    window: Duration,
}

impl SpeedTracker {
    pub fn new(window: Duration) -> Self {
        Self {
            samples: VecDeque::with_capacity(64),
            window,
        }
    }

    pub fn default_window() -> Self {
        Self::new(Duration::from_secs(5))
    }

    pub fn add_sample(&mut self, bytes: u64) {
        let now = Instant::now();
        self.samples.push_back((now, bytes));
        while let Some((t, _)) = self.samples.front() {
            if now.duration_since(*t) > self.window {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn speed_bps(&self) -> u64 {
        if self.samples.len() < 2 {
            return 0;
        }

        let (t1, b1) = self.samples.front().unwrap();
        let (t2, b2) = self.samples.back().unwrap();
        let duration = t2.duration_since(*t1).as_secs_f64();

        if duration > 0.1 {
            ((b2.saturating_sub(*b1)) as f64 / duration) as u64
        } else {
            0
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransferLimits {
    pub max_concurrent_sends: usize,
    pub max_concurrent_receives: usize,
}

impl Default for TransferLimits {
    fn default() -> Self {
        Self {
            max_concurrent_sends: 50,
            max_concurrent_receives: 50,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TransferProgress {
    /// Transfer is preparing (importing files, creating endpoint)
    Preparing { id: TransferId, status: String },
    /// Transfer is waiting for peer connection
    Connecting { id: TransferId },
    /// Transfer has started with known size
    Started {
        id: TransferId,
        name: String,
        total_bytes: u64,
    },
    /// Transfer progress update
    Progress {
        id: TransferId,
        transferred_bytes: u64,
        speed_bps: u64,
    },
    /// Connection established with peer
    Connected { id: TransferId, is_relay: bool },
    /// Ticket is ready to share (sender only)
    TicketReady { id: TransferId, ticket: String },
    /// Transfer completed successfully
    Completed {
        id: TransferId,
        total_bytes: u64,
        duration_secs: f64,
    },
    /// Transfer failed with error
    Failed { id: TransferId, error: String },
    /// Transfer was cancelled
    Cancelled { id: TransferId },
    /// File conflicts detected (receiver only)
    FileConflicts {
        id: TransferId,
        conflicts: Vec<(String, PathBuf)>,
        total_bytes: u64,
    },
    /// Transfer is queued waiting for slot
    Queued { id: TransferId, position: usize },
    /// File list for received transfers (name, size)
    FileList {
        id: TransferId,
        files: Vec<(String, u64)>,
    },
}

#[derive(Debug)]
pub enum TransferCommand {
    Send {
        id: String,
        paths: Vec<PathBuf>,
        follow_symlinks: bool,
    },
    Receive {
        id: String,
        ticket: BlobTicket,
        output_dir: PathBuf,
    },
    Cancel {
        id: String,
    },
    Shutdown,
    ResolveConflict {
        id: String,
        resolution: ConflictResolution,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConflictResolution {
    Rename, // Add (1), (2), etc.
    Overwrite,
    Skip,
    Cancel,
}

pub struct TransferManager {
    cmd_tx: mpsc::Sender<TransferCommand>,
    progress_rx: mpsc::Receiver<TransferProgress>,
}

impl TransferManager {
    pub async fn with_limits(data_dir: PathBuf, limits: TransferLimits) -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (progress_tx, progress_rx) = mpsc::channel(256);
        tokio::spawn(run_manager(data_dir, cmd_rx, progress_tx, limits));

        Ok(Self {
            cmd_tx,
            progress_rx,
        })
    }

    pub async fn send_command(&self, cmd: TransferCommand) -> Result<()> {
        self.cmd_tx.send(cmd).await?;
        Ok(())
    }

    pub fn try_recv_progress(&mut self) -> Option<TransferProgress> {
        self.progress_rx.try_recv().ok()
    }
}

struct QueuedSend {
    id: String,
    paths: Vec<PathBuf>,
    follow_symlinks: bool,
}
struct QueuedReceive {
    id: String,
    ticket: BlobTicket,
    output_dir: PathBuf,
}

/// Background task managing all transfers
async fn run_manager(
    data_dir: PathBuf,
    mut cmd_rx: mpsc::Receiver<TransferCommand>,
    progress_tx: mpsc::Sender<TransferProgress>,
    limits: TransferLimits,
) {
    if let Err(e) = tokio::fs::create_dir_all(&data_dir).await {
        tracing::error!("Failed to create data directory: {}", e);
        return;
    }

    let store = match FsStore::load(&data_dir).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            tracing::error!("Failed to load store: {}", e);
            return;
        }
    };

    tracing::info!(
        "Transfer manager started, max_sends: {}, max_receives: {}",
        limits.max_concurrent_sends,
        limits.max_concurrent_receives
    );

    let mut active_sends: std::collections::HashMap<
        String,
        (tokio::task::JoinHandle<()>, CancellationToken),
    > = std::collections::HashMap::new();
    let mut active_receives: std::collections::HashMap<
        String,
        (tokio::task::JoinHandle<()>, CancellationToken),
    > = std::collections::HashMap::new();
    let mut conflict_resolvers: std::collections::HashMap<String, ConflictResolver> =
        std::collections::HashMap::new();
    let mut send_queue: Vec<QueuedSend> = Vec::new();
    let mut receive_queue: Vec<QueuedReceive> = Vec::new();
    let start_send = |id: String,
                      paths: Vec<PathBuf>,
                      follow_symlinks: bool,
                      store: Arc<FsStore>,
                      progress_tx: mpsc::Sender<TransferProgress>|
     -> (tokio::task::JoinHandle<()>, CancellationToken) {
        let cancel_token = CancellationToken::new();
        let options = SendOptions { follow_symlinks };
        let task = SendTask::new(
            id.clone(),
            paths,
            store,
            progress_tx,
            options,
            cancel_token.clone(),
        );
        let handle = tokio::spawn(async move {
            if let Err(e) = task.run().await {
                tracing::error!("Send task failed: {}", e);
            }
        });
        (handle, cancel_token)
    };

    let start_receive = |id: String,
                         ticket: BlobTicket,
                         output_dir: PathBuf,
                         store: Arc<FsStore>,
                         progress_tx: mpsc::Sender<TransferProgress>|
     -> (
        tokio::task::JoinHandle<()>,
        CancellationToken,
        ConflictResolver,
    ) {
        let cancel_token = CancellationToken::new();
        let (task, resolver) = ReceiveTask::new(
            id.clone(),
            ticket,
            output_dir,
            store,
            progress_tx,
            cancel_token.clone(),
        );
        let handle = tokio::spawn(async move {
            tracing::info!("Receive task spawned, running...");
            if let Err(e) = task.run().await {
                tracing::error!("Receive task failed: {}", e);
            }
        });
        (handle, cancel_token, resolver)
    };

    loop {
        // Clean up finished tasks
        active_sends.retain(|_, (handle, _)| !handle.is_finished());
        active_receives.retain(|id, (handle, _)| {
            if handle.is_finished() {
                conflict_resolvers.remove(id);
                false
            } else {
                true
            }
        });

        // Start queued transfers
        while active_sends.len() < limits.max_concurrent_sends && !send_queue.is_empty() {
            let queued = send_queue.remove(0);
            tracing::info!("Starting queued send: {}", queued.id);
            let (handle, cancel_token) = start_send(
                queued.id.clone(),
                queued.paths,
                queued.follow_symlinks,
                store.clone(),
                progress_tx.clone(),
            );
            active_sends.insert(queued.id, (handle, cancel_token));

            for (pos, q) in send_queue.iter().enumerate() {
                let _ = progress_tx
                    .send(TransferProgress::Queued {
                        id: q.id.clone().into(),
                        position: pos + 1,
                    })
                    .await;
            }
        }

        while active_receives.len() < limits.max_concurrent_receives && !receive_queue.is_empty() {
            let queued = receive_queue.remove(0);
            tracing::info!("Starting queued receive: {}", queued.id);
            let (handle, cancel_token, resolver) = start_receive(
                queued.id.clone(),
                queued.ticket,
                queued.output_dir,
                store.clone(),
                progress_tx.clone(),
            );
            conflict_resolvers.insert(queued.id.clone(), resolver);
            active_receives.insert(queued.id, (handle, cancel_token));

            for (pos, q) in receive_queue.iter().enumerate() {
                let _ = progress_tx
                    .send(TransferProgress::Queued {
                        id: q.id.clone().into(),
                        position: pos + 1,
                    })
                    .await;
            }
        }

        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    TransferCommand::Send { id, paths, follow_symlinks } => {
                        if active_sends.len() < limits.max_concurrent_sends {
                            let (handle, cancel_token) = start_send(id.clone(), paths, follow_symlinks, store.clone(), progress_tx.clone());
                            active_sends.insert(id, (handle, cancel_token));
                        } else {
                            let position = send_queue.len() + 1;
                            tracing::info!("Send {} queued at position {}", id, position);
                            send_queue.push(QueuedSend { id: id.clone(), paths, follow_symlinks });
                            let _ = progress_tx
                                .send(TransferProgress::Queued { id: id.clone().into(), position })
                                .await;
                        }
                    }
                    TransferCommand::Receive { id, ticket, output_dir } => {
                        tracing::info!("Receive request for id: {}", id);
                        if active_receives.len() < limits.max_concurrent_receives {
                            let (handle, cancel_token, resolver) = start_receive(
                                id.clone(),
                                ticket,
                                output_dir,
                                store.clone(),
                                progress_tx.clone(),
                            );
                            conflict_resolvers.insert(id.clone(), resolver);
                            active_receives.insert(id, (handle, cancel_token));
                        } else {
                            let position = receive_queue.len() + 1;
                            tracing::info!("Receive {} queued at position {}", id, position);
                            receive_queue.push(QueuedReceive { id: id.clone(), ticket, output_dir });
                            let _ = progress_tx
                                .send(TransferProgress::Queued { id: id.clone().into(), position })
                                .await;
                        }
                    }
                    TransferCommand::ResolveConflict { id, resolution } => {
                        tracing::info!("Resolving conflict for id: {}, resolution: {:?}", id, resolution);
                        if let Some(resolver) = conflict_resolvers.remove(&id) {
                            if let Err(e) = resolver.tx.send(resolution).await {
                                tracing::error!("Failed to send resolution: {}", e);
                            }
                        } else {
                            tracing::warn!("No conflict resolver found for id: {}", id);
                        }
                    }
                    TransferCommand::Cancel { id } => {
                        let was_queued = send_queue.iter().any(|q| q.id == id)
                            || receive_queue.iter().any(|q| q.id == id);
                        send_queue.retain(|q| q.id != id);
                        receive_queue.retain(|q| q.id != id);

                        if was_queued {
                            let _ = progress_tx.send(TransferProgress::Cancelled { id: id.clone().into() }).await;
                        } else {
                            if let Some((_, cancel_token)) = active_sends.remove(&id) {
                                tracing::info!("Cancelling send task: {}", id);
                                cancel_token.cancel();
                            }
                            if let Some((_, cancel_token)) = active_receives.remove(&id) {
                                tracing::info!("Cancelling receive task: {}", id);
                                cancel_token.cancel();
                                conflict_resolvers.remove(&id);
                            }
                        }
                    }
                    TransferCommand::Shutdown => {
                        tracing::info!("Transfer manager shutting down");
                        for (_, (_, cancel_token)) in active_sends.drain() {
                            cancel_token.cancel();
                        }
                        for (_, (_, cancel_token)) in active_receives.drain() {
                            cancel_token.cancel();
                        }
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
            else => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_speed_tracker_empty() {
        let tracker = SpeedTracker::new(Duration::from_secs(5));
        assert_eq!(tracker.speed_bps(), 0);
    }

    #[test]
    fn test_speed_tracker_single_sample() {
        let mut tracker = SpeedTracker::new(Duration::from_secs(5));
        tracker.add_sample(1000);
        assert_eq!(tracker.speed_bps(), 0);
    }

    #[test]
    fn test_speed_tracker_two_samples() {
        let mut tracker = SpeedTracker::new(Duration::from_secs(5));
        tracker.add_sample(0);
        thread::sleep(Duration::from_millis(200));
        tracker.add_sample(2000);

        let speed = tracker.speed_bps();
        // ~10000 bps with some tolerance
        assert!(
            speed > 5000 && speed < 15000,
            "Speed {} not in expected range",
            speed
        );
    }

    #[test]
    fn test_speed_tracker_default_window() {
        let tracker = SpeedTracker::default_window();
        assert_eq!(tracker.window, Duration::from_secs(5));
    }

    #[test]
    fn test_speed_tracker_zero_duration_protection() {
        let mut tracker = SpeedTracker::new(Duration::from_secs(5));
        tracker.add_sample(0);
        tracker.add_sample(10000);
        // Duration < 100ms should return 0
        assert_eq!(tracker.speed_bps(), 0);
    }

    #[test]
    fn test_transfer_limits_default() {
        let limits = TransferLimits::default();
        assert_eq!(limits.max_concurrent_sends, 50);
        assert_eq!(limits.max_concurrent_receives, 50);
    }

    #[test]
    fn test_transfer_progress_variants() {
        let id: TransferId = "test".into();

        let _ = TransferProgress::Preparing {
            id: id.clone(),
            status: "Importing...".into(),
        };
        let _ = TransferProgress::Connecting { id: id.clone() };
        let _ = TransferProgress::Connected {
            id: id.clone(),
            is_relay: false,
        };
        let _ = TransferProgress::Started {
            id: id.clone(),
            name: "test".into(),
            total_bytes: 100,
        };
        let _ = TransferProgress::Progress {
            id: id.clone(),
            transferred_bytes: 50,
            speed_bps: 100,
        };
        let _ = TransferProgress::TicketReady {
            id: id.clone(),
            ticket: "ticket".into(),
        };
        let _ = TransferProgress::Completed {
            id: id.clone(),
            total_bytes: 100,
            duration_secs: 1.0,
        };
        let _ = TransferProgress::Failed {
            id: id.clone(),
            error: "error".into(),
        };
        let _ = TransferProgress::Cancelled { id: id.clone() };
        let _ = TransferProgress::Queued {
            id: id.clone(),
            position: 1,
        };
        let _ = TransferProgress::FileList {
            id: id.clone(),
            files: vec![("file".into(), 100)],
        };
    }

    #[test]
    fn test_conflict_resolution_variants() {
        let _ = ConflictResolution::Rename;
        let _ = ConflictResolution::Overwrite;
        let _ = ConflictResolution::Skip;
        let _ = ConflictResolution::Cancel;
    }

    #[test]
    fn test_transfer_command_variants() {
        let _ = TransferCommand::Send {
            id: "test".into(),
            paths: vec![],
            follow_symlinks: false,
        };
        let _ = TransferCommand::Cancel { id: "test".into() };
        let _ = TransferCommand::Shutdown;
        let _ = TransferCommand::ResolveConflict {
            id: "test".into(),
            resolution: ConflictResolution::Skip,
        };
    }
}
