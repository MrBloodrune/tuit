//! Application state

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use iroh_blobs::ticket::BlobTicket;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::input::KeyPreset;
use crate::theme::ThemeKind;
use crate::transfer::ConflictResolution;
use crate::tree_browser::TreeBrowser;

/// Actions requiring async handling
#[derive(Debug)]
pub enum AppAction {
    StartSend {
        id: String,
        paths: Vec<PathBuf>,
        follow_symlinks: bool,
    },
    StartReceive {
        id: String,
        ticket: BlobTicket,
        output_dir: PathBuf,
    },
    CancelTransfer {
        id: String,
    },
    ResolveConflict {
        id: String,
        resolution: ConflictResolution,
    },
}

/// Active tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Send,
    Receive,
    Active,
    History,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Send, Mode::Receive, Mode::Active, Mode::History];

    pub fn label(&self) -> &'static str {
        match self {
            Mode::Send => "Send",
            Mode::Receive => "Receive",
            Mode::Active => "Active",
            Mode::History => "History",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Mode::Send => 0,
            Mode::Receive => 1,
            Mode::Active => 2,
            Mode::History => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        Mode::ALL[i % 4]
    }

    pub fn next(&self) -> Self {
        Self::from_index(self.index() + 1)
    }

    pub fn prev(&self) -> Self {
        Self::from_index((self.index() + 3) % 4)
    }
}

/// Network connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConnectionStatus {
    #[default]
    Ready,
    Connecting,
    P2P,
    Relay,
}

impl ConnectionStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ConnectionStatus::Ready => "Ready",
            ConnectionStatus::Connecting => "Connecting",
            ConnectionStatus::P2P => "P2P",
            ConnectionStatus::Relay => "Relay",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            ConnectionStatus::Ready => "○",
            ConnectionStatus::Connecting => "◐",
            ConnectionStatus::P2P => "●",
            ConnectionStatus::Relay => "◑",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    Preparing,  // Importing files, creating endpoint
    Connecting, // Waiting for peer connection
    Active,     // Transfer in progress
    Paused,
    Stalled,
    Queued,
    Failed,
    Complete,
}

impl TransferStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            TransferStatus::Preparing => "...",
            TransferStatus::Connecting => "◐",
            TransferStatus::Active => ">",
            TransferStatus::Paused => "||",
            TransferStatus::Stalled => "!",
            TransferStatus::Queued => "~",
            TransferStatus::Failed => "x",
            TransferStatus::Complete => "*",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TransferStatus::Preparing => "PREPARING",
            TransferStatus::Connecting => "WAITING",
            TransferStatus::Active => "",
            TransferStatus::Paused => "PAUSED",
            TransferStatus::Stalled => "STALLED",
            TransferStatus::Queued => "QUEUED",
            TransferStatus::Failed => "FAILED",
            TransferStatus::Complete => "DONE",
        }
    }
}

const MAX_STORED_FILES: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferFile {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: String,
    pub direction: TransferDirection,
    pub name: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub speed_bps: u64,
    pub status: TransferStatus,
    pub ticket: Option<String>,
    pub connection: ConnectionStatus,
    pub error_message: Option<String>,
    pub conflict_resolution: Option<ConflictResolution>,
    pub duration_secs: Option<f64>,
    #[serde(default)]
    pub files: Vec<TransferFile>,
    #[serde(default)]
    pub additional_file_count: usize,
    #[serde(skip)]
    pub source_paths: Option<Vec<PathBuf>>,
}

impl Transfer {
    pub fn progress_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0
        }
    }

    pub fn eta_seconds(&self) -> Option<u64> {
        if self.speed_bps == 0 {
            return None;
        }
        let remaining = self.total_bytes.saturating_sub(self.transferred_bytes);
        Some(remaining / self.speed_bps)
    }

    pub fn remaining_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.transferred_bytes)
    }

    pub fn set_files(&mut self, files: Vec<TransferFile>) {
        let total = files.len();
        if total > MAX_STORED_FILES {
            self.files = files.into_iter().take(MAX_STORED_FILES).collect();
            self.additional_file_count = total - MAX_STORED_FILES;
        } else {
            self.files = files;
            self.additional_file_count = 0;
        }
    }

    pub fn total_file_count(&self) -> usize {
        self.files.len() + self.additional_file_count
    }
}

#[derive(Debug)]
pub struct App {
    pub mode: Mode,
    pub theme: ThemeKind,
    pub key_preset: KeyPreset,
    pub connection: ConnectionStatus,
    pub show_help: bool,
    pub should_quit: bool,

    pub tree_browser: TreeBrowser,
    pub follow_symlinks: bool,
    pub ticket_input: String,
    pub receive_dir: PathBuf,
    pub input_active: bool,
    pub transfers: Vec<Transfer>,
    pub transfer_cursor: usize,
    pub session_history: Vec<Transfer>, // Completed this session
    pub history: Vec<Transfer>,         // Persisted
    pub history_cursor: usize,
    pub show_ticket_popup: Option<String>, // For SSH clipboard issues
    pub conflict_popup: Option<ConflictPopup>,
    pub theme_popup: Option<ThemePopup>,
    pub key_preset_popup: Option<KeyPresetPopup>,
    history_path: Option<PathBuf>,
    pub incognito: bool,
}

#[derive(Debug, Clone)]
pub struct ConflictPopup {
    pub transfer_id: String,
    pub conflicts: Vec<(String, PathBuf)>, // (name, existing_path)
    pub total_bytes: u64,
    pub selected: usize, // 0=Rename, 1=Overwrite, 2=Skip, 3=Cancel
}

#[derive(Debug, Clone)]
pub struct ThemePopup {
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub struct KeyPresetPopup {
    pub selected: usize,
}

impl ThemePopup {
    pub fn new(current: ThemeKind) -> Self {
        let selected = ThemeKind::ALL
            .iter()
            .position(|&t| t == current)
            .unwrap_or(0);
        Self { selected }
    }
}

impl KeyPresetPopup {
    pub fn new(current: KeyPreset) -> Self {
        let selected = KeyPreset::ALL
            .iter()
            .position(|&p| p == current)
            .unwrap_or(0);
        Self { selected }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

const MAX_HISTORY_ENTRIES: usize = 100;

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Send,
            theme: ThemeKind::default(),
            key_preset: KeyPreset::default(),
            connection: ConnectionStatus::Ready,
            show_help: false,
            should_quit: false,

            tree_browser: TreeBrowser::new(),
            follow_symlinks: false,

            ticket_input: String::new(),
            receive_dir: dirs::download_dir().unwrap_or_else(|| PathBuf::from(".")),
            input_active: false,

            transfers: Vec::new(),
            transfer_cursor: 0,

            session_history: Vec::new(),

            history: Vec::new(),
            history_cursor: 0,

            show_ticket_popup: None,
            conflict_popup: None,
            theme_popup: None,
            key_preset_popup: None,

            history_path: None,
            incognito: false,
        }
    }

    /// Builder method: Set incognito mode (no persistence)
    pub fn with_incognito(mut self, incognito: bool) -> Self {
        self.incognito = incognito;
        self
    }

    /// Builder method: Set theme by name
    pub fn with_theme_name(mut self, name: &str) -> Self {
        self.theme = ThemeKind::from_name(name);
        self
    }

    /// Builder method: Set key preset by name
    pub fn with_key_preset_name(mut self, name: &str) -> Self {
        self.key_preset = KeyPreset::from_name(name);
        self
    }

    /// Builder method: Set receive directory
    pub fn with_receive_dir(mut self, dir: PathBuf) -> Self {
        self.receive_dir = dir;
        self
    }

    /// Builder method: Set history path (if Some, loads history)
    pub fn with_history_path_opt(mut self, path: Option<PathBuf>) -> Self {
        self.history_path = path;
        if self.history_path.is_some() {
            self.load_history();
        }
        self
    }

    fn load_history(&mut self) {
        let Some(path) = &self.history_path else {
            return;
        };

        if !path.exists() {
            return;
        }

        match std::fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<Vec<Transfer>>(&contents) {
                Ok(history) => {
                    tracing::info!("Loaded {} history entries from {:?}", history.len(), path);
                    self.history = history;
                }
                Err(e) => {
                    tracing::warn!("Failed to parse history file {:?}: {}", path, e);
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read history file {:?}: {}", path, e);
            }
        }
    }

    pub fn save_history(&self) {
        // Don't save in incognito mode
        if self.incognito {
            return;
        }

        let Some(path) = &self.history_path else {
            return;
        };

        let entries_to_save: Vec<_> = self
            .history
            .iter()
            .rev()
            .take(MAX_HISTORY_ENTRIES)
            .rev()
            .cloned()
            .collect();

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::warn!("Failed to create history directory {:?}: {}", parent, e);
                return;
            }
        }

        match serde_json::to_string_pretty(&entries_to_save) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    tracing::warn!("Failed to write history file {:?}: {}", path, e);
                } else {
                    tracing::debug!(
                        "Saved {} history entries to {:?}",
                        entries_to_save.len(),
                        path
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to serialize history: {}", e);
            }
        }
    }

    pub fn add_to_history(&mut self, transfer: Transfer) {
        self.session_history.push(transfer.clone());
        if self.session_history.len() > 10 {
            self.session_history.remove(0);
        }
        self.history.push(transfer);
        self.save_history();
    }

    /// Save current preferences to config file
    fn save_config(&self) {
        // Don't save in incognito mode
        if self.incognito {
            return;
        }

        let mut config = Config::load();
        config.preferences.theme = self.theme.name().to_string();
        config.preferences.key_preset = self.key_preset.name().to_string();
        config.preferences.receive_dir = Some(self.receive_dir.clone());

        if let Err(e) = config.save() {
            tracing::warn!("Failed to save config: {}", e);
        } else {
            tracing::debug!(
                "Saved config with theme: {}, key_preset: {}",
                self.theme.name(),
                self.key_preset.name()
            );
        }
    }

    /// Returns true when user is actively typing text (ticket input or search)
    fn is_text_input_active(&self) -> bool {
        self.input_active || (self.mode == Mode::Send && self.tree_browser.search_active)
    }

    pub fn handle_key_with_action(&mut self, key: KeyEvent) -> Option<AppAction> {
        // Conflict popup takes priority
        if self.conflict_popup.is_some() {
            return self.handle_conflict_popup_key(key);
        }

        if self.theme_popup.is_some() {
            self.handle_theme_popup_key(key);
            return None;
        }

        if self.key_preset_popup.is_some() {
            self.handle_key_preset_popup_key(key);
            return None;
        }

        if self.show_ticket_popup.is_some() {
            self.show_ticket_popup = None;
            return None;
        }

        if self.show_help {
            self.show_help = false;
            return None;
        }

        // Skip global shortcuts when typing (search or ticket input)
        if !self.is_text_input_active() {
            match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    return None;
                }
                KeyCode::Char('?') => {
                    self.show_help = true;
                    return None;
                }
                KeyCode::Char('t') => {
                    self.theme_popup = Some(ThemePopup::new(self.theme));
                    return None;
                }
                KeyCode::Char('B') => {
                    self.key_preset_popup = Some(KeyPresetPopup::new(self.key_preset));
                    return None;
                }
                KeyCode::Char('1') => {
                    self.mode = Mode::Send;
                    return None;
                }
                KeyCode::Char('2') => {
                    self.mode = Mode::Receive;
                    return None;
                }
                KeyCode::Char('3') => {
                    self.mode = Mode::Active;
                    return None;
                }
                KeyCode::Char('4') => {
                    self.mode = Mode::History;
                    return None;
                }
                KeyCode::Tab => {
                    self.mode = self.mode.next();
                    return None;
                }
                KeyCode::BackTab => {
                    self.mode = self.mode.prev();
                    return None;
                }
                KeyCode::Esc => {
                    if self.mode == Mode::Send {
                        if self.tree_browser.search_active {
                            self.tree_browser.cancel_search();
                        } else if self.tree_browser.has_search_results() {
                            self.tree_browser.clear_search_results();
                        }
                    }
                    return None;
                }
                _ => {}
            }
        }

        match self.mode {
            Mode::Send => self.handle_send_key_with_action(key),
            Mode::Receive => self.handle_receive_key_with_action(key),
            Mode::Active => self.handle_active_key_with_action(key),
            Mode::History => self.handle_history_key_with_action(key),
        }
    }

    fn handle_send_key_with_action(&mut self, key: KeyEvent) -> Option<AppAction> {
        if self.tree_browser.search_active {
            // During active search: arrow keys navigate, all chars go to search query
            match key.code {
                KeyCode::Esc => self.tree_browser.cancel_search(),
                KeyCode::Enter => self.tree_browser.finish_search(),
                KeyCode::Backspace => self.tree_browser.search_pop(),
                KeyCode::Up => self.tree_browser.move_up(),
                KeyCode::Down => self.tree_browser.move_down(),
                KeyCode::Left => self.tree_browser.collapse_selected(),
                KeyCode::Right => self.tree_browser.enter(),
                KeyCode::Char(c) => self.tree_browser.search_push(c),
                _ => {}
            }
            return None;
        }

        if self.key_preset.is_down(&key) {
            self.tree_browser.move_down();
        } else if self.key_preset.is_up(&key) {
            self.tree_browser.move_up();
        } else if self.key_preset.is_right(&key) {
            self.tree_browser.enter();
        } else if self.key_preset.is_left(&key) {
            // Don't navigate to parent directory when viewing search results
            if self.tree_browser.has_search_results() {
                self.tree_browser.collapse_selected();
            } else {
                self.tree_browser.go_up();
            }
        } else {
            match key.code {
                KeyCode::Char(' ') => self.tree_browser.toggle_selection(),
                KeyCode::Char('a') => self.tree_browser.select_all(),
                KeyCode::Char('c') => self.tree_browser.clear_selection(),
                KeyCode::Char('/') => self.tree_browser.start_search(),
                KeyCode::Char('g') => self.tree_browser.move_to_first(),
                KeyCode::Char('G') => self.tree_browser.move_to_last(),
                KeyCode::Char('S') => self.follow_symlinks = !self.follow_symlinks,
                KeyCode::Char('s') | KeyCode::Enter => {
                    if !self.tree_browser.selected.is_empty() {
                        return self.start_send_action();
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn handle_receive_key_with_action(&mut self, key: KeyEvent) -> Option<AppAction> {
        if self.input_active {
            // Ctrl+V paste
            if key.code == KeyCode::Char('v') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        self.ticket_input.push_str(&text);
                    }
                }
                return None;
            }
            // Ctrl+U clear line
            if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.ticket_input.clear();
                return None;
            }

            match key.code {
                KeyCode::Char(c) => self.ticket_input.push(c),
                KeyCode::Backspace => {
                    self.ticket_input.pop();
                }
                KeyCode::Enter => {
                    if !self.ticket_input.is_empty() {
                        return self.start_receive_action();
                    }
                }
                KeyCode::Esc => self.input_active = false,
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Enter | KeyCode::Char('i') => self.input_active = true,
                KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.input_active = true;
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            self.ticket_input.push_str(&text);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn handle_active_key_with_action(&mut self, key: KeyEvent) -> Option<AppAction> {
        if self.key_preset.is_down(&key) {
            if !self.transfers.is_empty() {
                self.transfer_cursor = (self.transfer_cursor + 1).min(self.transfers.len() - 1);
            }
            return None;
        } else if self.key_preset.is_up(&key) {
            self.transfer_cursor = self.transfer_cursor.saturating_sub(1);
            return None;
        }

        match key.code {
            KeyCode::Char('c') => {
                if let Some(transfer) = self.transfers.get(self.transfer_cursor) {
                    if let Some(ref ticket) = transfer.ticket {
                        self.show_ticket_popup = Some(ticket.clone());
                        copy_to_clipboard_osc52(ticket); // Works over SSH
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(ticket.clone());
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                if let Some(transfer) = self.transfers.get_mut(self.transfer_cursor) {
                    match transfer.status {
                        TransferStatus::Active => transfer.status = TransferStatus::Paused,
                        TransferStatus::Paused => transfer.status = TransferStatus::Active,
                        _ => {}
                    }
                }
            }
            KeyCode::Char('x') => {
                if let Some(transfer) = self.transfers.get(self.transfer_cursor) {
                    let id = transfer.id.clone();
                    return Some(AppAction::CancelTransfer { id });
                }
            }
            KeyCode::Char('r') => {
                if let Some(transfer) = self.transfers.get_mut(self.transfer_cursor) {
                    if transfer.status == TransferStatus::Failed {
                        transfer.status = TransferStatus::Active;
                        transfer.error_message = None;
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn handle_history_key_with_action(&mut self, key: KeyEvent) -> Option<AppAction> {
        if self.key_preset.is_down(&key) {
            if !self.history.is_empty() {
                self.history_cursor = (self.history_cursor + 1).min(self.history.len() - 1);
            }
            return None;
        } else if self.key_preset.is_up(&key) {
            self.history_cursor = self.history_cursor.saturating_sub(1);
            return None;
        }

        match key.code {
            KeyCode::Char('c') => {
                if let Some(transfer) = self.history.get(self.history_cursor) {
                    if let Some(ref ticket) = transfer.ticket {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(ticket.clone());
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                // Resend (uploads only)
                if let Some(transfer) = self.history.get(self.history_cursor) {
                    if transfer.direction == TransferDirection::Upload {
                        if let Some(ref paths) = transfer.source_paths {
                            let missing: Vec<_> = paths.iter().filter(|p| !p.exists()).collect();
                            if missing.is_empty() {
                                return self.start_resend(paths.clone());
                            } else {
                                let missing_names: Vec<_> = missing
                                    .iter()
                                    .filter_map(|p| p.file_name())
                                    .map(|n| n.to_string_lossy().to_string())
                                    .collect();
                                tracing::warn!("Cannot resend: missing files: {:?}", missing_names);
                            }
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                if !self.history.is_empty() {
                    self.history.remove(self.history_cursor);
                    if self.history_cursor > 0 && self.history_cursor >= self.history.len() {
                        self.history_cursor = self.history.len().saturating_sub(1);
                    }
                    self.save_history();
                }
            }
            _ => {}
        }
        None
    }

    fn start_resend(&mut self, paths: Vec<PathBuf>) -> Option<AppAction> {
        let name = if paths.len() == 1 {
            paths[0]
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "file".to_string())
        } else {
            format!("{} items", paths.len())
        };

        let id = uuid::Uuid::new_v4().to_string();

        let files: Vec<TransferFile> = paths
            .iter()
            .map(|p| TransferFile {
                name: p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                size: p.metadata().map(|m| m.len()).unwrap_or(0),
            })
            .collect();

        let mut transfer = Transfer {
            id: id.clone(),
            direction: TransferDirection::Upload,
            name,
            total_bytes: 0,
            transferred_bytes: 0,
            speed_bps: 0,
            status: TransferStatus::Active,
            ticket: None,
            connection: ConnectionStatus::Connecting,
            error_message: None,
            conflict_resolution: None,
            duration_secs: None,
            files: Vec::new(),
            additional_file_count: 0,
            source_paths: Some(paths.clone()),
        };
        transfer.set_files(files);

        self.transfers.push(transfer);
        self.mode = Mode::Active;

        Some(AppAction::StartSend {
            id,
            paths,
            follow_symlinks: self.follow_symlinks,
        })
    }

    fn start_send_action(&mut self) -> Option<AppAction> {
        let paths = self.tree_browser.selected.clone();
        if paths.is_empty() {
            return None;
        }

        let name = if paths.len() == 1 {
            paths[0]
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "file".to_string())
        } else {
            format!("{} items", paths.len())
        };

        let id = uuid::Uuid::new_v4().to_string();

        let files: Vec<TransferFile> = paths
            .iter()
            .map(|p| TransferFile {
                name: p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                size: p.metadata().map(|m| m.len()).unwrap_or(0),
            })
            .collect();

        let mut transfer = Transfer {
            id: id.clone(),
            direction: TransferDirection::Upload,
            name,
            total_bytes: 0,
            transferred_bytes: 0,
            speed_bps: 0,
            status: TransferStatus::Active,
            ticket: None,
            connection: ConnectionStatus::Connecting,
            error_message: None,
            conflict_resolution: None,
            duration_secs: None,
            files: Vec::new(),
            additional_file_count: 0,
            source_paths: Some(paths.clone()),
        };
        transfer.set_files(files);

        self.transfers.push(transfer);
        self.tree_browser.clear_selection();
        self.mode = Mode::Active;

        Some(AppAction::StartSend {
            id,
            paths,
            follow_symlinks: self.follow_symlinks,
        })
    }

    fn start_receive_action(&mut self) -> Option<AppAction> {
        let ticket_str = self.ticket_input.trim().to_string();
        tracing::info!(
            "start_receive_action called, ticket len: {}",
            ticket_str.len()
        );
        if ticket_str.is_empty() {
            tracing::warn!("Ticket is empty!");
            return None;
        }

        tracing::info!("Parsing ticket...");
        let ticket = match ticket_str.parse::<BlobTicket>() {
            Ok(t) => {
                tracing::info!("Ticket parsed OK, hash: {}", t.hash());
                t
            }
            Err(e) => {
                tracing::error!("Ticket parse failed: {}", e);
                let id = uuid::Uuid::new_v4().to_string();
                let transfer = Transfer {
                    id,
                    direction: TransferDirection::Download,
                    name: "Invalid ticket".to_string(),
                    total_bytes: 0,
                    transferred_bytes: 0,
                    speed_bps: 0,
                    status: TransferStatus::Failed,
                    ticket: Some(ticket_str),
                    connection: ConnectionStatus::Ready,
                    error_message: Some(format!("Invalid ticket: {}", e)),
                    conflict_resolution: None,
                    duration_secs: None,
                    files: Vec::new(),
                    additional_file_count: 0,
                    source_paths: None,
                };
                self.transfers.push(transfer);
                self.ticket_input.clear();
                self.input_active = false;
                self.mode = Mode::Active;
                return None;
            }
        };

        let id = uuid::Uuid::new_v4().to_string();
        let transfer = Transfer {
            id: id.clone(),
            direction: TransferDirection::Download,
            name: "Connecting...".to_string(),
            total_bytes: 0,
            transferred_bytes: 0,
            speed_bps: 0,
            status: TransferStatus::Active,
            ticket: Some(ticket_str),
            connection: ConnectionStatus::Connecting,
            error_message: None,
            conflict_resolution: None,
            duration_secs: None,
            files: Vec::new(),
            additional_file_count: 0,
            source_paths: None,
        };

        self.transfers.push(transfer);
        self.ticket_input.clear();
        self.input_active = false;
        self.mode = Mode::Active;

        Some(AppAction::StartReceive {
            id,
            ticket,
            output_dir: self.receive_dir.clone(),
        })
    }

    fn handle_conflict_popup_key(&mut self, key: KeyEvent) -> Option<AppAction> {
        let popup = self.conflict_popup.as_mut()?;

        if self.key_preset.is_up(&key) {
            if popup.selected > 0 {
                popup.selected -= 1;
            }
            return None;
        } else if self.key_preset.is_down(&key) {
            if popup.selected < 3 {
                popup.selected += 1;
            }
            return None;
        }

        match key.code {
            KeyCode::Enter => {
                let resolution = match popup.selected {
                    0 => ConflictResolution::Rename,
                    1 => ConflictResolution::Overwrite,
                    2 => ConflictResolution::Skip,
                    _ => ConflictResolution::Cancel,
                };
                let id = popup.transfer_id.clone();
                self.conflict_popup = None;
                return Some(AppAction::ResolveConflict { id, resolution });
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                let id = popup.transfer_id.clone();
                self.conflict_popup = None;
                return Some(AppAction::ResolveConflict {
                    id,
                    resolution: ConflictResolution::Cancel,
                });
            }
            KeyCode::Char('1') => {
                let id = popup.transfer_id.clone();
                self.conflict_popup = None;
                return Some(AppAction::ResolveConflict {
                    id,
                    resolution: ConflictResolution::Rename,
                });
            }
            KeyCode::Char('2') => {
                let id = popup.transfer_id.clone();
                self.conflict_popup = None;
                return Some(AppAction::ResolveConflict {
                    id,
                    resolution: ConflictResolution::Overwrite,
                });
            }
            KeyCode::Char('3') => {
                let id = popup.transfer_id.clone();
                self.conflict_popup = None;
                return Some(AppAction::ResolveConflict {
                    id,
                    resolution: ConflictResolution::Skip,
                });
            }
            KeyCode::Char('4') => {
                let id = popup.transfer_id.clone();
                self.conflict_popup = None;
                return Some(AppAction::ResolveConflict {
                    id,
                    resolution: ConflictResolution::Cancel,
                });
            }
            _ => {}
        }
        None
    }

    fn handle_theme_popup_key(&mut self, key: KeyEvent) {
        let Some(ref mut popup) = self.theme_popup else {
            return;
        };

        if self.key_preset.is_up(&key) {
            if popup.selected > 0 {
                popup.selected -= 1;
            }
            return;
        } else if self.key_preset.is_down(&key) {
            if popup.selected < ThemeKind::ALL.len() - 1 {
                popup.selected += 1;
            }
            return;
        }

        match key.code {
            KeyCode::Enter => {
                self.theme = ThemeKind::ALL[popup.selected];
                self.theme_popup = None;
                self.save_config();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.theme_popup = None;
            }
            KeyCode::Char('1') => {
                self.theme = ThemeKind::ALL[0];
                self.theme_popup = None;
                self.save_config();
            }
            KeyCode::Char('2') => {
                self.theme = ThemeKind::ALL[1];
                self.theme_popup = None;
                self.save_config();
            }
            KeyCode::Char('3') => {
                self.theme = ThemeKind::ALL[2];
                self.theme_popup = None;
                self.save_config();
            }
            KeyCode::Char('4') => {
                self.theme = ThemeKind::ALL[3];
                self.theme_popup = None;
                self.save_config();
            }
            KeyCode::Char('5') => {
                self.theme = ThemeKind::ALL[4];
                self.theme_popup = None;
                self.save_config();
            }
            _ => {}
        }
    }

    fn handle_key_preset_popup_key(&mut self, key: KeyEvent) {
        let Some(ref mut popup) = self.key_preset_popup else {
            return;
        };

        if self.key_preset.is_up(&key) || key.code == KeyCode::Up {
            if popup.selected > 0 {
                popup.selected -= 1;
            }
            return;
        } else if self.key_preset.is_down(&key) || key.code == KeyCode::Down {
            if popup.selected < KeyPreset::ALL.len() - 1 {
                popup.selected += 1;
            }
            return;
        }

        match key.code {
            KeyCode::Enter => {
                self.key_preset = KeyPreset::ALL[popup.selected];
                self.key_preset_popup = None;
                self.save_config();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.key_preset_popup = None;
            }
            KeyCode::Char('1') => {
                self.key_preset = KeyPreset::ALL[0];
                self.key_preset_popup = None;
                self.save_config();
            }
            KeyCode::Char('2') => {
                self.key_preset = KeyPreset::ALL[1];
                self.key_preset_popup = None;
                self.save_config();
            }
            KeyCode::Char('3') => {
                self.key_preset = KeyPreset::ALL[2];
                self.key_preset_popup = None;
                self.save_config();
            }
            _ => {}
        }
    }
}

mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
    }

    pub fn download_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join("Downloads"))
    }
}

/// OSC52 clipboard (works over SSH: VSCode, iTerm2, kitty, etc.)
fn copy_to_clipboard_osc52(text: &str) {
    use std::io::Write;
    let encoded = base64_encode(text.as_bytes());
    let osc52 = format!("\x1b]52;c;{}\x07", encoded);
    if let Ok(mut stdout) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        let _ = stdout.write_all(osc52.as_bytes());
        let _ = stdout.flush();
    }
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };

        result.push(ALPHABET[(b0 >> 2) as usize] as char);
        result.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if i + 1 < data.len() {
            result.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if i + 2 < data.len() {
            result.push(ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test Transfer
    fn test_transfer(total: u64, transferred: u64, speed: u64) -> Transfer {
        Transfer {
            id: "test".to_string(),
            direction: TransferDirection::Upload,
            name: "test.txt".to_string(),
            total_bytes: total,
            transferred_bytes: transferred,
            speed_bps: speed,
            status: TransferStatus::Active,
            ticket: None,
            connection: ConnectionStatus::P2P,
            error_message: None,
            conflict_resolution: None,
            duration_secs: None,
            files: vec![],
            additional_file_count: 0,
            source_paths: None,
        }
    }

    // Mode::ALL tests
    #[test]
    fn test_mode_all_has_four() {
        assert_eq!(Mode::ALL.len(), 4);
    }

    #[test]
    fn test_mode_all_contains_all_variants() {
        assert_eq!(Mode::ALL[0], Mode::Send);
        assert_eq!(Mode::ALL[1], Mode::Receive);
        assert_eq!(Mode::ALL[2], Mode::Active);
        assert_eq!(Mode::ALL[3], Mode::History);
    }

    // Mode::label tests
    #[test]
    fn test_mode_labels_non_empty() {
        for mode in Mode::ALL.iter() {
            assert!(!mode.label().is_empty());
        }
    }

    #[test]
    fn test_mode_labels_correct() {
        assert_eq!(Mode::Send.label(), "Send");
        assert_eq!(Mode::Receive.label(), "Receive");
        assert_eq!(Mode::Active.label(), "Active");
        assert_eq!(Mode::History.label(), "History");
    }

    // Mode::index tests
    #[test]
    fn test_mode_index_unique() {
        let indices: Vec<usize> = Mode::ALL.iter().map(|m| m.index()).collect();
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_mode_index_values() {
        assert_eq!(Mode::Send.index(), 0);
        assert_eq!(Mode::Receive.index(), 1);
        assert_eq!(Mode::Active.index(), 2);
        assert_eq!(Mode::History.index(), 3);
    }

    // Mode::from_index tests
    #[test]
    fn test_mode_from_index_valid() {
        assert_eq!(Mode::from_index(0), Mode::Send);
        assert_eq!(Mode::from_index(1), Mode::Receive);
        assert_eq!(Mode::from_index(2), Mode::Active);
        assert_eq!(Mode::from_index(3), Mode::History);
    }

    #[test]
    fn test_mode_from_index_wraps() {
        assert_eq!(Mode::from_index(4), Mode::Send);
        assert_eq!(Mode::from_index(5), Mode::Receive);
        assert_eq!(Mode::from_index(100), Mode::Send); // 100 % 4 = 0
        assert_eq!(Mode::from_index(101), Mode::Receive); // 101 % 4 = 1
    }

    // Mode::next tests
    #[test]
    fn test_mode_next_cycles() {
        assert_eq!(Mode::Send.next(), Mode::Receive);
        assert_eq!(Mode::Receive.next(), Mode::Active);
        assert_eq!(Mode::Active.next(), Mode::History);
        assert_eq!(Mode::History.next(), Mode::Send); // Wraps around
    }

    // Mode::prev tests
    #[test]
    fn test_mode_prev_cycles() {
        assert_eq!(Mode::Send.prev(), Mode::History); // Wraps to end
        assert_eq!(Mode::History.prev(), Mode::Active);
        assert_eq!(Mode::Active.prev(), Mode::Receive);
        assert_eq!(Mode::Receive.prev(), Mode::Send);
    }

    #[test]
    fn test_mode_next_prev_roundtrip() {
        for mode in Mode::ALL.iter() {
            assert_eq!(mode.next().prev(), *mode);
            assert_eq!(mode.prev().next(), *mode);
        }
    }

    // ConnectionStatus::label tests
    #[test]
    fn test_connection_status_labels() {
        assert!(!ConnectionStatus::Ready.label().is_empty());
        assert!(!ConnectionStatus::Connecting.label().is_empty());
        assert!(!ConnectionStatus::P2P.label().is_empty());
        assert!(!ConnectionStatus::Relay.label().is_empty());
    }

    #[test]
    fn test_connection_status_labels_correct() {
        assert_eq!(ConnectionStatus::Ready.label(), "Ready");
        assert_eq!(ConnectionStatus::Connecting.label(), "Connecting");
        assert_eq!(ConnectionStatus::P2P.label(), "P2P");
        assert_eq!(ConnectionStatus::Relay.label(), "Relay");
    }

    // ConnectionStatus::symbol tests
    #[test]
    fn test_connection_status_symbols() {
        assert!(!ConnectionStatus::Ready.symbol().is_empty());
        assert!(!ConnectionStatus::Connecting.symbol().is_empty());
        assert!(!ConnectionStatus::P2P.symbol().is_empty());
        assert!(!ConnectionStatus::Relay.symbol().is_empty());
    }

    #[test]
    fn test_connection_status_symbols_correct() {
        assert_eq!(ConnectionStatus::Ready.symbol(), "○");
        assert_eq!(ConnectionStatus::Connecting.symbol(), "◐");
        assert_eq!(ConnectionStatus::P2P.symbol(), "●");
        assert_eq!(ConnectionStatus::Relay.symbol(), "◑");
    }

    #[test]
    fn test_connection_status_symbols_unique() {
        let symbols = [
            ConnectionStatus::Ready.symbol(),
            ConnectionStatus::Connecting.symbol(),
            ConnectionStatus::P2P.symbol(),
            ConnectionStatus::Relay.symbol(),
        ];
        let unique_count = symbols
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(unique_count, 4);
    }

    // TransferStatus::symbol tests
    #[test]
    fn test_transfer_status_symbols() {
        assert!(!TransferStatus::Preparing.symbol().is_empty());
        assert!(!TransferStatus::Connecting.symbol().is_empty());
        assert!(!TransferStatus::Active.symbol().is_empty());
        assert!(!TransferStatus::Paused.symbol().is_empty());
        assert!(!TransferStatus::Stalled.symbol().is_empty());
        assert!(!TransferStatus::Queued.symbol().is_empty());
        assert!(!TransferStatus::Failed.symbol().is_empty());
        assert!(!TransferStatus::Complete.symbol().is_empty());
    }

    #[test]
    fn test_transfer_status_symbols_correct() {
        assert_eq!(TransferStatus::Preparing.symbol(), "...");
        assert_eq!(TransferStatus::Connecting.symbol(), "◐");
        assert_eq!(TransferStatus::Active.symbol(), ">");
        assert_eq!(TransferStatus::Paused.symbol(), "||");
        assert_eq!(TransferStatus::Stalled.symbol(), "!");
        assert_eq!(TransferStatus::Queued.symbol(), "~");
        assert_eq!(TransferStatus::Failed.symbol(), "x");
        assert_eq!(TransferStatus::Complete.symbol(), "*");
    }

    #[test]
    fn test_transfer_status_symbols_unique() {
        let symbols = [
            TransferStatus::Preparing.symbol(),
            TransferStatus::Connecting.symbol(),
            TransferStatus::Active.symbol(),
            TransferStatus::Paused.symbol(),
            TransferStatus::Stalled.symbol(),
            TransferStatus::Queued.symbol(),
            TransferStatus::Failed.symbol(),
            TransferStatus::Complete.symbol(),
        ];
        let unique_count = symbols
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(unique_count, 8);
    }

    // TransferStatus::label tests
    #[test]
    fn test_transfer_status_labels() {
        // Note: Active status has empty label, which is valid
        assert!(!TransferStatus::Preparing.label().is_empty());
        assert!(!TransferStatus::Connecting.label().is_empty());
        assert_eq!(TransferStatus::Active.label(), "");
        assert!(!TransferStatus::Paused.label().is_empty());
        assert!(!TransferStatus::Stalled.label().is_empty());
        assert!(!TransferStatus::Queued.label().is_empty());
        assert!(!TransferStatus::Failed.label().is_empty());
        assert!(!TransferStatus::Complete.label().is_empty());
    }

    #[test]
    fn test_transfer_status_labels_correct() {
        assert_eq!(TransferStatus::Preparing.label(), "PREPARING");
        assert_eq!(TransferStatus::Connecting.label(), "WAITING");
        assert_eq!(TransferStatus::Active.label(), "");
        assert_eq!(TransferStatus::Paused.label(), "PAUSED");
        assert_eq!(TransferStatus::Stalled.label(), "STALLED");
        assert_eq!(TransferStatus::Queued.label(), "QUEUED");
        assert_eq!(TransferStatus::Failed.label(), "FAILED");
        assert_eq!(TransferStatus::Complete.label(), "DONE");
    }

    // Transfer::progress_percent tests
    #[test]
    fn test_transfer_progress_zero_total() {
        let transfer = test_transfer(0, 0, 0);
        assert_eq!(transfer.progress_percent(), 0.0);
    }

    #[test]
    fn test_transfer_progress_zero_percent() {
        let transfer = test_transfer(1000, 0, 0);
        assert_eq!(transfer.progress_percent(), 0.0);
    }

    #[test]
    fn test_transfer_progress_50_percent() {
        let transfer = test_transfer(1000, 500, 0);
        assert_eq!(transfer.progress_percent(), 50.0);
    }

    #[test]
    fn test_transfer_progress_100_percent() {
        let transfer = test_transfer(1000, 1000, 0);
        assert_eq!(transfer.progress_percent(), 100.0);
    }

    #[test]
    fn test_transfer_progress_partial() {
        let transfer = test_transfer(1000, 250, 0);
        assert_eq!(transfer.progress_percent(), 25.0);

        let transfer = test_transfer(1000, 750, 0);
        assert_eq!(transfer.progress_percent(), 75.0);
    }

    #[test]
    fn test_transfer_progress_over_100() {
        // Edge case: transferred > total (should not happen in practice)
        let transfer = test_transfer(1000, 1500, 0);
        assert_eq!(transfer.progress_percent(), 150.0);
    }

    // Transfer::eta_seconds tests
    #[test]
    fn test_transfer_eta_zero_speed() {
        let transfer = test_transfer(1000, 0, 0);
        assert_eq!(transfer.eta_seconds(), None);
    }

    #[test]
    fn test_transfer_eta_calculation() {
        // 1000 bytes remaining, 100 bytes per second = 10 seconds
        let transfer = test_transfer(1000, 0, 100);
        assert_eq!(transfer.eta_seconds(), Some(10));
    }

    #[test]
    fn test_transfer_eta_partial_complete() {
        // 500 bytes remaining (1000 total - 500 transferred), 100 bytes per second = 5 seconds
        let transfer = test_transfer(1000, 500, 100);
        assert_eq!(transfer.eta_seconds(), Some(5));
    }

    #[test]
    fn test_transfer_eta_complete() {
        // 0 bytes remaining
        let transfer = test_transfer(1000, 1000, 100);
        assert_eq!(transfer.eta_seconds(), Some(0));
    }

    #[test]
    fn test_transfer_eta_various_speeds() {
        let transfer = test_transfer(10000, 0, 1000);
        assert_eq!(transfer.eta_seconds(), Some(10));

        let transfer = test_transfer(5000, 1000, 500);
        assert_eq!(transfer.eta_seconds(), Some(8)); // 4000 remaining / 500
    }

    // Transfer::remaining_bytes tests
    #[test]
    fn test_transfer_remaining_zero() {
        let transfer = test_transfer(1000, 1000, 0);
        assert_eq!(transfer.remaining_bytes(), 0);
    }

    #[test]
    fn test_transfer_remaining_all() {
        let transfer = test_transfer(1000, 0, 0);
        assert_eq!(transfer.remaining_bytes(), 1000);
    }

    #[test]
    fn test_transfer_remaining_partial() {
        let transfer = test_transfer(1000, 400, 0);
        assert_eq!(transfer.remaining_bytes(), 600);
    }

    #[test]
    fn test_transfer_remaining_saturates() {
        // Edge case: transferred > total should return 0, not underflow
        let transfer = test_transfer(1000, 1500, 0);
        assert_eq!(transfer.remaining_bytes(), 0);
    }

    #[test]
    fn test_transfer_remaining_large_values() {
        let transfer = test_transfer(u64::MAX, 0, 0);
        assert_eq!(transfer.remaining_bytes(), u64::MAX);

        let transfer = test_transfer(u64::MAX, u64::MAX / 2, 0);
        assert_eq!(transfer.remaining_bytes(), u64::MAX / 2 + 1);
    }

    // Transfer::set_files tests
    #[test]
    fn test_transfer_set_files_under_max() {
        let mut transfer = test_transfer(0, 0, 0);
        let files = vec![
            TransferFile {
                name: "file1.txt".to_string(),
                size: 100,
            },
            TransferFile {
                name: "file2.txt".to_string(),
                size: 200,
            },
        ];

        transfer.set_files(files.clone());
        assert_eq!(transfer.files.len(), 2);
        assert_eq!(transfer.additional_file_count, 0);
        assert_eq!(transfer.total_file_count(), 2);
    }

    #[test]
    fn test_transfer_set_files_at_max() {
        let mut transfer = test_transfer(0, 0, 0);
        let files: Vec<TransferFile> = (0..15)
            .map(|i| TransferFile {
                name: format!("file{}.txt", i),
                size: 100,
            })
            .collect();

        transfer.set_files(files);
        assert_eq!(transfer.files.len(), 15);
        assert_eq!(transfer.additional_file_count, 0);
        assert_eq!(transfer.total_file_count(), 15);
    }

    #[test]
    fn test_transfer_set_files_over_max() {
        let mut transfer = test_transfer(0, 0, 0);
        let files: Vec<TransferFile> = (0..20)
            .map(|i| TransferFile {
                name: format!("file{}.txt", i),
                size: 100,
            })
            .collect();

        transfer.set_files(files);
        assert_eq!(transfer.files.len(), 15);
        assert_eq!(transfer.additional_file_count, 5);
        assert_eq!(transfer.total_file_count(), 20);
    }

    #[test]
    fn test_transfer_set_files_way_over_max() {
        let mut transfer = test_transfer(0, 0, 0);
        let files: Vec<TransferFile> = (0..100)
            .map(|i| TransferFile {
                name: format!("file{}.txt", i),
                size: 100,
            })
            .collect();

        transfer.set_files(files);
        assert_eq!(transfer.files.len(), 15);
        assert_eq!(transfer.additional_file_count, 85);
        assert_eq!(transfer.total_file_count(), 100);
    }

    #[test]
    fn test_transfer_set_files_empty() {
        let mut transfer = test_transfer(0, 0, 0);
        transfer.set_files(vec![]);
        assert_eq!(transfer.files.len(), 0);
        assert_eq!(transfer.additional_file_count, 0);
        assert_eq!(transfer.total_file_count(), 0);
    }

    // Integration tests
    #[test]
    fn test_transfer_calculations_integration() {
        let transfer = test_transfer(10_000, 2_500, 1000);

        assert_eq!(transfer.progress_percent(), 25.0);
        assert_eq!(transfer.remaining_bytes(), 7_500);
        assert_eq!(transfer.eta_seconds(), Some(7)); // 7500 / 1000 = 7.5, truncated to 7
    }

    #[test]
    fn test_mode_enum_default() {
        assert_eq!(Mode::default(), Mode::Send);
    }

    #[test]
    fn test_connection_status_default() {
        assert_eq!(ConnectionStatus::default(), ConnectionStatus::Ready);
    }
}
