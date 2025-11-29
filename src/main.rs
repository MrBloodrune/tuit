//! Tuit - Terminal UI for P2P file transfers via iroh

mod app;
mod config;
mod input;
mod theme;
mod transfer;
mod tree_browser;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use config::Config;
use transfer::{TransferCommand, TransferLimits, TransferManager, TransferProgress};

#[derive(Parser, Debug)]
#[command(name = "tuit", about = "P2P file transfers via iroh", version)]
struct Args {
    /// Incognito mode - no config loading, no history, clean exit
    #[arg(long)]
    incognito: bool,

    /// Use alternate config file
    #[arg(long, value_name = "PATH")]
    config: Option<std::path::PathBuf>,

    /// Override receive directory
    #[arg(long, value_name = "PATH")]
    receive_dir: Option<std::path::PathBuf>,
}

fn main() -> Result<()> {
    // Logging to stderr when RUST_LOG is set
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Load configuration based on incognito mode
    let config = if args.incognito {
        tracing::info!("Running in incognito mode - using default config");
        Config::default()
    } else {
        Config::load_from(args.config.clone())
    };

    // Override receive_dir: CLI > config > current directory
    let receive_dir = args
        .receive_dir
        .or(config.preferences.receive_dir.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));

    // Per-instance data directory to avoid conflicts
    let instance_id = std::process::id();
    let base_dir = directories::ProjectDirs::from("com", "tuit", "tuit")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap().join(".tuit"));
    let data_dir = base_dir.join(format!("instance-{}", instance_id));
    tracing::info!("Using data directory: {:?}", data_dir);

    // Determine history path based on incognito mode and config
    let history_path = if args.incognito || !config.persistence.history {
        None
    } else {
        Some(base_dir.join("history.json"))
    };

    // Build transfer limits from config
    let limits = TransferLimits {
        max_concurrent_sends: config.transfer.max_concurrent_sends,
        max_concurrent_receives: config.transfer.max_concurrent_receives,
    };

    let mut transfer_manager = TransferManager::with_limits(data_dir.clone(), limits).await?;

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()
        .with_incognito(args.incognito)
        .with_theme_name(&config.preferences.theme)
        .with_key_preset_name(&config.preferences.key_preset)
        .with_receive_dir(receive_dir)
        .with_history_path_opt(history_path);
    let result = run(&mut terminal, &mut app, &mut transfer_manager).await;

    // Cleanup
    let _ = transfer_manager
        .send_command(TransferCommand::Shutdown)
        .await;

    // Clean up blob store if running in incognito mode
    if args.incognito {
        tracing::info!("Incognito mode: cleaning up data directory");
        if let Err(e) = std::fs::remove_dir_all(&data_dir) {
            tracing::warn!("Failed to remove incognito data directory: {}", e);
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Err(err) = result {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }

    Ok(())
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    transfer_manager: &mut TransferManager,
) -> Result<()> {
    // Start in Ready state - no active transfers
    app.connection = app::ConnectionStatus::Ready;

    loop {
        // Process transfer progress
        while let Some(progress) = transfer_manager.try_recv_progress() {
            handle_transfer_progress(app, progress);
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        // Poll events (100ms timeout for progress updates)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let action = app.handle_key_with_action(key);

                if let Some(action) = action {
                    match action {
                        app::AppAction::StartSend {
                            id,
                            paths,
                            follow_symlinks,
                        } => {
                            transfer_manager
                                .send_command(TransferCommand::Send {
                                    id,
                                    paths,
                                    follow_symlinks,
                                })
                                .await?;
                        }
                        app::AppAction::StartReceive {
                            id,
                            ticket,
                            output_dir,
                        } => {
                            tracing::info!(
                                "Sending Receive command to transfer manager, id: {}",
                                id
                            );
                            transfer_manager
                                .send_command(TransferCommand::Receive {
                                    id: id.clone(),
                                    ticket,
                                    output_dir,
                                })
                                .await?;
                            tracing::info!("Receive command sent for id: {}", id);
                        }
                        app::AppAction::CancelTransfer { id } => {
                            transfer_manager
                                .send_command(TransferCommand::Cancel { id })
                                .await?;
                        }
                        app::AppAction::ResolveConflict { id, resolution } => {
                            tracing::info!(
                                "Resolving conflict for id: {}, resolution: {:?}",
                                id,
                                resolution
                            );
                            // Store the resolution in the transfer for display
                            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id) {
                                transfer.conflict_resolution = Some(resolution.clone());
                            }
                            transfer_manager
                                .send_command(TransferCommand::ResolveConflict { id, resolution })
                                .await?;
                        }
                    }
                }

                if app.should_quit {
                    return Ok(());
                }
            }
        }
    }
}

fn handle_transfer_progress(app: &mut App, progress: TransferProgress) {
    match progress {
        TransferProgress::Preparing { id, status } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                transfer.status = app::TransferStatus::Preparing;
                transfer.name = status;
            }
        }
        TransferProgress::Connecting { id } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                transfer.status = app::TransferStatus::Connecting;
                transfer.connection = app::ConnectionStatus::Connecting;
            }
            // Update app-level status
            update_app_connection_status(app);
        }
        TransferProgress::Connected { id, is_relay } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                transfer.status = app::TransferStatus::Active;
                transfer.connection = if is_relay {
                    app::ConnectionStatus::Relay
                } else {
                    app::ConnectionStatus::P2P
                };
            }
            // Update app-level status
            update_app_connection_status(app);
        }
        TransferProgress::Started {
            id,
            name,
            total_bytes,
        } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                transfer.name = name;
                transfer.total_bytes = total_bytes;
                // Don't override status here - let Connecting/Connected handle it
            }
        }
        TransferProgress::Progress {
            id,
            transferred_bytes,
            speed_bps,
        } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                transfer.transferred_bytes = transferred_bytes;
                transfer.speed_bps = speed_bps;
                // Ensure status is Active when receiving progress
                if transfer.status == app::TransferStatus::Connecting {
                    transfer.status = app::TransferStatus::Active;
                }
            }
        }
        TransferProgress::TicketReady { id, ticket } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                transfer.ticket = Some(ticket.clone());
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&ticket);
                }
            }
        }
        TransferProgress::Completed {
            id,
            total_bytes,
            duration_secs,
        } => {
            app.show_ticket_popup = None;

            if let Some(idx) = app.transfers.iter().position(|t| t.id == id.as_ref()) {
                let mut transfer = app.transfers.remove(idx);
                transfer.status = app::TransferStatus::Complete;
                transfer.transferred_bytes = total_bytes;
                transfer.duration_secs = Some(duration_secs);
                app.add_to_history(transfer);
            }
            // Update app-level status (may return to Ready)
            update_app_connection_status(app);
        }
        TransferProgress::Failed { id, error } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                transfer.status = app::TransferStatus::Failed;
                transfer.error_message = Some(error);
            }
            // Update app-level status
            update_app_connection_status(app);
        }
        TransferProgress::Cancelled { id } => {
            app.transfers.retain(|t| t.id != id.as_ref());
            // Update app-level status (may return to Ready)
            update_app_connection_status(app);
        }
        TransferProgress::FileConflicts {
            id,
            conflicts,
            total_bytes,
        } => {
            app.conflict_popup = Some(app::ConflictPopup {
                transfer_id: id.to_string(),
                conflicts,
                total_bytes,
                selected: 0,
            });
        }
        TransferProgress::Queued { id, position } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                transfer.status = app::TransferStatus::Queued;
                if !transfer.name.contains("(queued") {
                    transfer.name = format!("{} (queued #{})", transfer.name, position);
                }
            }
        }
        TransferProgress::FileList { id, files } => {
            if let Some(transfer) = app.transfers.iter_mut().find(|t| t.id == id.as_ref()) {
                let file_list: Vec<app::TransferFile> = files
                    .into_iter()
                    .map(|(name, size)| app::TransferFile { name, size })
                    .collect();
                transfer.set_files(file_list);
            }
        }
    }
}

/// Update app-level connection status based on active transfers
fn update_app_connection_status(app: &mut App) {
    // Determine aggregate connection status from all active transfers
    // Priority: P2P > Relay > Connecting > Ready
    let mut has_p2p = false;
    let mut has_relay = false;
    let mut has_connecting = false;

    for transfer in &app.transfers {
        match transfer.status {
            app::TransferStatus::Failed | app::TransferStatus::Complete => continue,
            _ => {}
        }
        match transfer.connection {
            app::ConnectionStatus::P2P => has_p2p = true,
            app::ConnectionStatus::Relay => has_relay = true,
            app::ConnectionStatus::Connecting => has_connecting = true,
            app::ConnectionStatus::Ready => {}
        }
    }

    app.connection = if has_p2p {
        app::ConnectionStatus::P2P
    } else if has_relay {
        app::ConnectionStatus::Relay
    } else if has_connecting {
        app::ConnectionStatus::Connecting
    } else {
        app::ConnectionStatus::Ready
    };
}
