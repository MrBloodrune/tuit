//! Progress formatting utilities

/// Format bytes per second as human readable
pub fn format_speed(bps: u64) -> String {
    if bps == 0 {
        return "0 B/s".to_string();
    }
    let size = humansize::format_size(bps, humansize::BINARY);
    format!("{}/s", size)
}

/// Format seconds as MM:SS or HH:MM:SS
pub fn format_eta(seconds: u64) -> String {
    if seconds >= 3600 {
        let h = seconds / 3600;
        let m = (seconds % 3600) / 60;
        let s = seconds % 60;
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        let m = seconds / 60;
        let s = seconds % 60;
        format!("{}:{:02}", m, s)
    }
}
