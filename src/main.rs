use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Margin, RichText, Stroke, Vec2,
    ViewportCommand,
};
use serde::{Deserialize, Serialize};
use base64::Engine;
#[cfg(target_os = "macos")]
use std::ffi::CString;
mod agents;
use agents::AgentSnapshot;
mod airpods;
use airpods::AirPodsStatus;
mod artwork;
mod codex_bridge;
use codex_bridge::CodexEvent;
mod codex_desktop;
mod clipboard_images;
mod downloads;
use downloads::DownloadEntry;
mod island_gesture;
use island_gesture::{Action as IslandGestureAction, IslandGesture};
mod lyrics;
mod media_remote_adapter;
mod music_airplay;
use music_airplay::AirPlayDevice;
mod music_queue;
mod pocket;
mod pipelines;
mod shake;
mod prompter;
use lyrics::Line as LyricLine;
mod video;
use video::{VideoEvent, VideoProgress};
mod weather;
use weather::Weather;
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    collections::VecDeque,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const BLACK: Color32 = Color32::from_rgb(3, 4, 6);
const CARD: Color32 = Color32::from_rgba_premultiplied(18, 20, 26, 174);
const CARD_2: Color32 = Color32::from_rgb(51, 53, 60);
const TEXT: Color32 = Color32::from_rgb(246, 246, 248);
const MUTED: Color32 = Color32::from_rgb(158, 160, 167);
const ACCENT: Color32 = Color32::from_rgb(198, 217, 255);

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn nooklet_install_glass(view: *mut std::ffi::c_void) -> i32;
    fn nooklet_anchor_host_to_screen_top(enabled: i32, display_id: u32);
    fn nooklet_displays_copy(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn nooklet_reduce_motion() -> i32;
    fn nooklet_physical_notch(width: *mut f64, height: *mut f64) -> i32;
    fn nooklet_progress_ring_set(enabled: i32, x: f64, y: f64, usage_style: i32);
    fn bookland_activity_bubble_show(message: *const std::ffi::c_char, closed_width: f64);
    fn bookland_activity_bubble_hide();
    fn bookland_result_card_show(path: *const std::ffi::c_char,
        subtitle: *const std::ffi::c_char, seconds: f64, panel_bottom: f64);
    fn bookland_result_card_position(panel_bottom: f64);
    fn bookland_conversion_card_hide();
    fn bookland_volume_hud_set(enabled: i32, inline_mode: i32, closed_width: f64);
    fn bookland_system_volume_get() -> f32;
    fn bookland_system_volume_set(value: f32) -> i32;
    fn bookland_audio_spectrum_set(enabled: i32);
    fn bookland_audio_spectrum_copy(levels: *mut f32, count: usize) -> i32;
    #[cfg(test)]
    fn bookland_audio_spectrum_test_tone(frequency: f32, channel_mode: i32,
        levels: *mut f32, count: usize);
    fn bookland_airpods_watch_set(enabled: i32) -> i32;
    fn bookland_airpods_events_take() -> i32;
    fn bookland_airpods_watch_status() -> i32;
    fn bookland_airpods_authorization() -> i32;
    fn bookland_airpods_request_access() -> i32;
    fn bookland_scroll_take(delta_x: *mut f64, delta_y: *mut f64) -> i32;
    fn bookland_haptic_feedback();
    fn nooklet_hover_monitor_set(enabled: i32, context: *mut std::ffi::c_void,
        wake: Option<extern "C" fn(*mut std::ffi::c_void)>);
    fn nooklet_animation_clock_set(enabled: i32, context: *mut std::ffi::c_void,
        wake: Option<extern "C" fn(*mut std::ffi::c_void)>) -> i32;
    fn nooklet_animation_clock_rate(fps: i32);
    fn nooklet_set_glass(
        slot: i32, x: f64, y: f64, width: f64, height: f64,
        radius: f64, level: f64, opacity: f64, enabled: i32,
    );
    fn nooklet_morph_shell_set(enabled: i32, x: f64, y: f64, width: f64,
        height: f64, top_radius: f64, bottom_radius: f64, opacity: f64);
    fn nooklet_morph_shell_animate(frames: *const NativeMorphFrame, count: usize,
        duration: f64, minimum_width: f64, minimum_height: f64) -> i32;
    fn nooklet_calendar_status() -> i32;
    fn nooklet_calendar_request_pending() -> i32;
    fn nooklet_calendar_month_index() -> i32;
    fn nooklet_calendar_month(offset: i32, buffer: *mut std::ffi::c_char,
        capacity: usize) -> usize;
    fn nooklet_calendar_connect();
    fn nooklet_calendar_refresh(calendar_id: *const std::ffi::c_char, month_offset: i32);
    fn nooklet_calendar_snapshot_version() -> u64;
    fn nooklet_calendar_copy(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn nooklet_calendar_calendars_copy(buffer: *mut std::ffi::c_char,
        capacity: usize) -> usize;
    fn nooklet_calendar_add(title: *const std::ffi::c_char, start: f64,
        duration: f64, calendar_id: *const std::ffi::c_char) -> i32;
    fn nooklet_shortcut_set(enabled: i32) -> i32;
    fn nooklet_shortcut_take() -> i32;
    fn bookland_clipboard_shortcut_set(enabled: i32) -> i32;
    fn bookland_clipboard_shortcut_take() -> i32;
    fn bookland_choose_folder(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn bookland_choose_prompter_script(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn bookland_choose_pocket_file(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn bookland_airdrop_file(path: *const std::ffi::c_char) -> bool;
    fn bookland_pocket_drag_monitor_set(enabled: bool, context: *mut std::ffi::c_void,
        wake: Option<extern "C" fn(*mut std::ffi::c_void)>,
        sample: Option<extern "C" fn(f64, bool) -> bool>);
    fn bookland_pocket_drag_monitor_take(x: *mut f64, y: *mut f64) -> bool;
    fn bookland_pointer_over_island_rect(x: f64, y: f64, width: f64,
        height: f64, margin: f64, require_released: bool) -> bool;
    fn bookland_mirror_permission() -> i32;
    fn bookland_mirror_camera_available() -> bool;
    fn bookland_mirror_state() -> i32;
    fn bookland_mirror_start() -> i32;
    fn bookland_mirror_stop();
    fn bookland_mirror_copy(buffer: *mut u8, capacity: usize,
        width: *mut i32, height: *mut i32, sequence: *mut u64) -> usize;
    fn bookland_pocket_anchor_at_cursor(x: *mut f64, y: *mut f64) -> bool;
    fn bookland_clipboard_is_sensitive() -> i32;
    fn bookland_frontmost_bundle_id(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn bookland_clipboard_change_count() -> i64;
    fn bookland_clipboard_read_file_urls(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn bookland_clipboard_write_file_urls(buffer: *const std::ffi::c_char, length: usize) -> i32;
    fn bookland_speech_start(locale: *const std::ffi::c_char, allow_network: i32) -> i32;
    fn bookland_speech_stop();
    fn bookland_speech_state() -> i32;
    fn bookland_speech_transcript(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn bookland_speech_message(buffer: *mut std::ffi::c_char, capacity: usize) -> usize;
    fn bookland_speech_capabilities(locale: *const std::ffi::c_char) -> i32;
}

#[cfg(target_os = "macos")]
extern "C" fn bookland_pocket_wake(context: *mut std::ffi::c_void) {
    if !context.is_null() {
        unsafe { (&*(context as *const egui::Context)).request_repaint(); }
    }
}

#[cfg(target_os = "macos")]
extern "C" fn bookland_pocket_shake_sample(x: f64, dragging: bool) -> bool {
    static DETECTOR: std::sync::OnceLock<std::sync::Mutex<shake::DragShake>> =
        std::sync::OnceLock::new();
    DETECTOR.get_or_init(|| std::sync::Mutex::new(shake::DragShake::default()))
        .lock().map(|mut detector| detector.update(dragging.then_some(x as f32),
            dragging, Instant::now())).unwrap_or(false)
}

#[derive(Clone, Deserialize)]
struct CalendarEvent {
    title: String,
    start: f64,
    #[allow(dead_code)]
    end: f64,
    calendar: String,
    #[serde(default)]
    day_key: String,
    #[serde(default)]
    day_keys: Vec<String>,
    #[allow(dead_code)]
    all_day: bool,
    #[serde(default)]
    display_time: String,
}

impl CalendarEvent {
    fn occurs_on_day(&self, key: &str) -> bool {
        self.day_key == key || self.day_keys.iter().any(|day| day == key)
    }
}

fn calendar_day_counts(events: &[CalendarEvent], year: i32, month: u32) -> [u8; 32] {
    let prefix = format!("{year:04}-{month:02}-");
    let mut counts = [0_u8; 32];
    for event in events {
        let mut counted = [false; 32];
        for key in std::iter::once(&event.day_key).chain(event.day_keys.iter()) {
            if let Some(day) = key.strip_prefix(&prefix)
                .and_then(|value| value.parse::<usize>().ok()) {
                if day <= 31 && !counted[day] {
                    counts[day] = counts[day].saturating_add(1);
                    counted[day] = true;
                }
            }
        }
    }
    counts
}

#[cfg(test)]
mod calendar_event_tests {
    use super::{calendar_day_counts, CalendarEvent};

    #[test]
    fn event_spanning_dates_appears_on_each_day_without_including_exclusive_end() {
        let event: CalendarEvent = serde_json::from_str(r#"{
            "title":"Conference", "start":0, "end":1,
            "calendar":"Office", "day_key":"2026-11-24",
            "day_keys":["2026-11-24","2026-11-25","2026-11-26"],
            "all_day":true
        }"#).unwrap();
        assert!(event.occurs_on_day("2026-11-24"));
        assert!(event.occurs_on_day("2026-11-25"));
        assert!(event.occurs_on_day("2026-11-26"));
        assert!(!event.occurs_on_day("2026-11-27"));
        let counts = calendar_day_counts(&[event], 2026, 11);
        assert_eq!((counts[24], counts[25], counts[26], counts[27]), (1, 1, 1, 0));
    }

    #[test]
    fn older_event_snapshots_still_show_the_start_day() {
        let event: CalendarEvent = serde_json::from_str(r#"{
            "title":"Meeting", "start":0, "end":1,
            "calendar":"Office", "day_key":"2026-11-24",
            "all_day":false
        }"#).unwrap();
        assert!(event.occurs_on_day("2026-11-24"));
        assert!(!event.occurs_on_day("2026-11-25"));
    }
}

#[derive(Clone, Deserialize)]
struct CalendarChoice {
    id: String,
    title: String,
    source: String,
    writable: bool,
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct CalendarMonth {
    title: String,
    #[allow(dead_code)]
    grid: String,
    year: i32,
    month: u32,
    first_weekday: u32,
    days_in_month: u32,
    today: u32,
}

fn load_calendar_month(offset: i32) -> Option<CalendarMonth> {
    #[cfg(target_os = "macos")]
    {
        let mut buffer = [0_u8; 2048];
        let size = unsafe { nooklet_calendar_month(offset, buffer.as_mut_ptr().cast(), buffer.len()) };
        (size > 0).then(|| serde_json::from_slice(&buffer[..size]).ok()).flatten()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = offset;
        None
    }
}

#[cfg(all(test, target_os = "macos"))]
mod calendar_month_tests {
    use super::{load_calendar_month, run_output};
    use std::process::Command;

    #[test]
    fn native_month_matches_existing_calendar_text() {
        for offset in [-1, 0, 1] {
            let shift = format!("-v{:+}m", offset);
            let numerical = run_output("date", &[&shift, "+%m %Y"]).unwrap();
            let parts: Vec<_> = numerical.split_whitespace().collect();
            let expected_title = run_output("date", &[&shift, "+%B %Y"]).unwrap();
            let expected_grid = String::from_utf8(Command::new("cal")
                .args([parts[0], parts[1]]).output().unwrap().stdout).unwrap();
            let native = load_calendar_month(offset).unwrap();
            assert_eq!(native.title, expected_title);
            assert_eq!(native.grid, expected_grid.trim());
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Workspace {
    Nook,
    Music,
    Calendar,
    Tray,
    Clipboard,
    Agents,
    Prompts,
    Prompter,
}

#[derive(Clone, Copy)]
struct TourPage {
    eyebrow: &'static str,
    title: &'static str,
    summary: &'static str,
    detail: &'static str,
    workspace: Option<Workspace>,
    settings: &'static str,
}

const TOUR_PAGES: [TourPage; 7] = [
    TourPage { eyebrow: "WELCOME", title: "A little more room at the top.",
        summary: "Bookland lives around your Mac's camera cutout. Hover over the island to open it; move away to close it.",
        detail: "Click the island when you want it to stay open. Press Escape to close it again.",
        workspace: Some(Workspace::Nook), settings: "General" },
    TourPage { eyebrow: "NOOK", title: "The things you reach for most.",
        summary: "See the current song, live Mac readings, a Focus timer, and your to-dos in one glance.",
        detail: "Choose and reorder Nook widgets in Settings. Your workspace stays at the top edge.",
        workspace: Some(Workspace::Nook), settings: "Nook" },
    TourPage { eyebrow: "MUSIC", title: "Keep the music close.",
        summary: "Play, pause, skip, and scrub the current Mac media session without switching windows.",
        detail: "Lyrics appear when a synced match is available. AirPlay routes are available for Apple Music.",
        workspace: Some(Workspace::Music), settings: "Music" },
    TourPage { eyebrow: "POCKET + CLIPBOARD", title: "Keep what you need nearby.",
        summary: "Search recent copied items, then drop files into Pocket for a quick copy or conversion.",
        detail: "Clipboard capture and its privacy controls are configurable in Settings.",
        workspace: Some(Workspace::Clipboard), settings: "Clipboard" },
    TourPage { eyebrow: "PLAN + PRESENT", title: "Make room for the next task.",
        summary: "Check upcoming calendar events, keep a script near the camera, or run a Focus timer.",
        detail: "Calendar and voice following ask for macOS permission only when you use them.",
        workspace: Some(Workspace::Prompter), settings: "Prompter" },
    TourPage { eyebrow: "AGENTS", title: "See Codex at work.",
        summary: "Follow local Codex activity and open a connected chat from the island.",
        detail: "For turns started through Bookland, approval requests and questions can be answered here.",
        workspace: Some(Workspace::Agents), settings: "Agents" },
    TourPage { eyebrow: "MAKE IT YOURS", title: "Set the island your way.",
        summary: "Choose Glass or Classic, tune the motion, and arrange the controls that belong in your Nook.",
        detail: "Start with the Appearance settings, then come back any time to replay this tour.",
        workspace: None, settings: "Appearance" },
];
impl Workspace {
    const ISLAND: [Self; 5] = [
        Self::Nook, Self::Music, Self::Calendar, Self::Tray, Self::Agents,
    ];
    const DEFAULT: [Self; 7] = [
        Self::Nook,
        Self::Music,
        Self::Calendar,
        Self::Tray,
        Self::Clipboard,
        Self::Agents,
        Self::Prompts,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Nook => "Nook",
            Self::Music => "Music",
            Self::Calendar => "Calendar",
            Self::Tray => "Tray",
            Self::Clipboard => "Clipboard",
            Self::Agents => "Agents",
            Self::Prompts => "Prompts",
            Self::Prompter => "Prompter",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
enum Widget {
    MediaPlayer,
    Pulse,
    Pomodoro,
    Todos,
    Notes,
    Clipboard,
    Downloads,
    Terminal,
    Weather,
    Mirror,
}
impl Widget {
    const DEFAULT: [Self; 4] = [Self::MediaPlayer, Self::Pulse, Self::Pomodoro, Self::Todos];
    const ALL: [Self; 10] = [Self::MediaPlayer, Self::Pulse, Self::Pomodoro, Self::Todos,
        Self::Notes, Self::Clipboard, Self::Downloads, Self::Terminal, Self::Weather, Self::Mirror];
    fn name(self) -> &'static str {
        match self {
            Self::MediaPlayer => "Media Player",
            Self::Pulse => "Pulse",
            Self::Pomodoro => "Pomodoro",
            Self::Todos => "To-dos",
            Self::Notes => "Notes",
            Self::Clipboard => "Clipboard",
            Self::Downloads => "Downloads",
            Self::Terminal => "Terminal",
            Self::Weather => "Weather",
            Self::Mirror => "Mirror",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::MediaPlayer => "♫",
            Self::Pulse => "◉",
            Self::Pomodoro => "◷",
            Self::Todos => "✓",
            Self::Notes => "✎",
            Self::Clipboard => "▤",
            Self::Downloads => "↓",
            Self::Terminal => ">_",
            Self::Weather => "☀",
            Self::Mirror => "◩",
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct TodoItem {
    text: String,
    done: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct CodexChatLine {
    role: String,
    text: String,
}

struct AgentCompletion {
    source: String,
    path: PathBuf,
    created_at: Instant,
}

fn selected_agent_index(selected: Option<(String, PathBuf)>, fallback: usize,
                        agents: &[agents::AgentSnapshot]) -> usize {
    selected.and_then(|(source, path)| agents.iter().position(|agent|
        agent.source == source && agent.path == path))
        .unwrap_or_else(|| fallback.min(agents.len().saturating_sub(1)))
}

#[cfg(test)]
mod agent_selection_tests {
    use super::{agents::AgentSnapshot, selected_agent_index};
    use std::path::PathBuf;

    #[test]
    fn selection_follows_task_across_refresh_order() {
        let agents = [
            AgentSnapshot { source: "Codex".into(), path: PathBuf::from("new"), ..Default::default() },
            AgentSnapshot { source: "Codex".into(), path: PathBuf::from("chosen"), ..Default::default() },
        ];
        assert_eq!(selected_agent_index(Some(("Codex".into(), PathBuf::from("chosen"))),
            0, &agents), 1);
        assert_eq!(selected_agent_index(Some(("Codex".into(), PathBuf::from("gone"))),
            9, &agents), 1);
    }
}

struct ClipboardImage {
    fingerprint: u64,
    width: usize,
    height: usize,
    rgba: Vec<u8>,
    texture: egui::TextureHandle,
    favorite: bool,
}

impl ClipboardImage {
    fn saved_record(&self) -> clipboard_images::SavedImage {
        clipboard_images::SavedImage {
            fingerprint: self.fingerprint, width: self.width,
            height: self.height, favorite: self.favorite,
        }
    }
}

fn restore_clipboard_images(ctx: &egui::Context,
                            records: &[clipboard_images::SavedImage]) -> Vec<ClipboardImage> {
    let Some(directory) = clipboard_images_dir() else { return Vec::new(); };
    records.iter().take(clipboard_images::MAX_IMAGES).filter_map(|record| {
        let rgba = clipboard_images::read_image(&directory, record)?;
        let full = image::RgbaImage::from_raw(record.width as u32,
            record.height as u32, rgba)?;
        let thumb = image::imageops::thumbnail(&full, 256, 160);
        let rgba = full.into_raw();
        let preview = egui::ColorImage::from_rgba_unmultiplied(
            [thumb.width() as usize, thumb.height() as usize], thumb.as_raw());
        let texture = ctx.load_texture(format!("clipboard-{}", record.fingerprint),
            preview, egui::TextureOptions::LINEAR);
        Some(ClipboardImage { fingerprint: record.fingerprint,
            width: record.width, height: record.height, rgba, texture,
            favorite: record.favorite })
    }).collect()
}

#[derive(Clone, Serialize, Deserialize)]
struct ClipboardFiles {
    paths: Vec<PathBuf>,
    favorite: bool,
}

fn trim_clipboard_files(items: &mut Vec<ClipboardFiles>, limit: usize) {
    let mut ordinary = 0;
    items.retain(|item| {
        if item.favorite { return true; }
        ordinary += 1;
        ordinary <= limit
    });
}

#[cfg(target_os = "macos")]
fn clipboard_file_urls() -> Vec<PathBuf> {
    let mut buffer = vec![0u8; 65_536];
    let length = unsafe { bookland_clipboard_read_file_urls(buffer.as_mut_ptr().cast(), buffer.len()) };
    if length == 0 || length > buffer.len() { return Vec::new(); }
    buffer[..length].split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .take(32)
        .filter_map(|part| std::str::from_utf8(part).ok().map(PathBuf::from))
        .collect()
}

#[cfg(target_os = "macos")]
fn copy_file_urls(paths: &[PathBuf]) -> bool {
    let mut bytes = Vec::new();
    for path in paths.iter().take(32) {
        let Some(text) = path.to_str() else { return false; };
        if text.as_bytes().contains(&0) || text.len() > 4096 { return false; }
        bytes.extend_from_slice(text.as_bytes());
        bytes.push(0);
    }
    !bytes.is_empty() && bytes.len() <= 65_536 &&
        unsafe { bookland_clipboard_write_file_urls(bytes.as_ptr().cast(), bytes.len()) != 0 }
}

fn trim_clipboard_history(items: &mut Vec<String>, favorites: &[String], limit: usize) {
    let mut ordinary = 0;
    items.retain(|item| {
        if favorites.contains(item) { return true; }
        ordinary += 1;
        ordinary <= limit
    });
}

#[cfg(test)]
mod clipboard_history_tests {
    use super::trim_clipboard_history;

    #[test]
    fn keeps_favorites_and_newest_ordinary_items() {
        let mut items = vec!["new".into(), "favorite".into(), "old".into()];
        trim_clipboard_history(&mut items, &["favorite".into()], 1);
        assert_eq!(items, vec!["new", "favorite"]);
    }
}

#[cfg(target_os = "macos")]
fn frontmost_bundle_id() -> Option<String> {
    let mut buffer = [0_u8; 512];
    let length = unsafe { bookland_frontmost_bundle_id(buffer.as_mut_ptr().cast(), buffer.len()) };
    (length > 0).then(|| String::from_utf8_lossy(&buffer[..length]).into_owned())
}

fn compose_codex_turn(prompt: &str, messages: &[CodexChatLine]) -> String {
    let Some(last_reply) = messages.iter().rposition(|line| line.role == "Codex") else {
        return prompt.to_owned();
    };
    let mut recent = Vec::new();
    let mut length = 0;
    for line in messages[..=last_reply].iter().rev().take(12) {
        let text: String = line.text.chars().take(3_000).collect();
        length += text.chars().count();
        if length > 12_000 { break; }
        recent.push(format!("{}: {}", line.role, text));
    }
    recent.reverse();
    format!("Continue this Bookland conversation. Use the recent messages as context, then answer the new user message.\n\nRecent messages:\n{}\n\nNew user message:\n{}",
        recent.join("\n\n"), prompt)
}

#[cfg(test)]
mod codex_chat_tests {
    use super::{compose_codex_turn, CodexChatLine};

    #[test]
    fn follow_up_includes_last_answer_without_unsent_turns() {
        let history = vec![
            CodexChatLine { role: "You".into(), text: "Say alpha".into() },
            CodexChatLine { role: "Codex".into(), text: "alpha".into() },
            CodexChatLine { role: "You".into(), text: "Failed request".into() },
        ];
        let next = compose_codex_turn("What did you say?", &history);
        assert!(next.contains("Codex: alpha"));
        assert!(next.contains("New user message:\nWhat did you say?"));
        assert!(!next.contains("Failed request"));
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct AppData {
    files: Vec<PathBuf>,
    pipelines: Vec<pipelines::Pipeline>,
    clipboard: Vec<String>,
    clipboard_files: Vec<ClipboardFiles>,
    clipboard_images: Vec<clipboard_images::SavedImage>,
    clipboard_favorites: Vec<String>,
    notes: String,
    prompter: String,
    codex_session_id: Option<String>,
    codex_messages: Vec<CodexChatLine>,
    sheet_goal: String,
    sheet_source: String,
    sheet_result: String,
    sheet_constraints: String,
    sheet_prompt: String,
}

struct PocketJob {
    source: PathBuf,
    operation: PocketOperation,
    receiver: std::sync::mpsc::Receiver<Result<PathBuf, String>>,
}

enum PocketOperation {
    Action(pocket::Action),
    Pipeline(String),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    open_on_hover: bool,
    capture_clipboard: bool,
    persist_clipboard: bool,
    clipboard_open_bottom: bool,
    clipboard_hide_after_copy: bool,
    clipboard_focus_search: bool,
    clipboard_card_size: String,
    clipboard_history_size: usize,
    clipboard_clear_on_quit: bool,
    clipboard_ignore_passwords: bool,
    clipboard_ignore_apps: Vec<String>,
    start_expanded: bool,
    launch_at_login: bool,
    demo_mode: bool,
    media_source: String,
    calendar_id: String,
    island_display_id: u32,
    style: String,
    glass_level: f32,
    animation_speed: f32,
    menu_style: String,
    open_width: f32,
    open_height: f32,
    closed_width: f32,
    closed_height: f32,
    match_resolution: bool,
    rounded_buttons: bool,
    menu_labels: bool,
    show_extra_workspaces: bool,
    content_padding: f32,
    nook_enabled: bool,
    dividers: bool,
    widgets: Vec<Widget>,
    media_layout: String,
    extra_action: String,
    show_synced_lyrics: bool,
    audio_spectrum: bool,
    pulse_style: String,
    pulse_left: String,
    pulse_right: String,
    pulse_explain: bool,
    pulse_interval: u64,
    agent_monitor: bool,
    codex_allow_edits: bool,
    monitor_craft: bool,
    prompter_enabled: bool,
    prompter_speed: f32,
    prompter_mode: String,
    prompter_language: String,
    prompter_allow_network: bool,
    prompter_text_size: f32,
    weather_city: String,
    downloads_folder: String,
    watch_downloads: bool,
    global_shortcut: bool,
    clipboard_shortcut: bool,
    auto_close: bool,
    close_delay_ms: u64,
    prevent_close_while_typing: bool,
    lock_while_typing: bool,
    haptic_feedback: bool,
    gestures_on_hover: bool,
    gesture_vertical: bool,
    gesture_horizontal_media: bool,
    gesture_invert_media: bool,
    pocket_mode: String,
    pocket_battery: bool,
    pocket_shake: bool,
    notify_timer: bool,
    volume_hud: bool,
    volume_hud_style: String,
    todos: Vec<TodoItem>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            open_on_hover: true,
            capture_clipboard: true,
            persist_clipboard: false,
            clipboard_open_bottom: true,
            clipboard_hide_after_copy: true,
            clipboard_focus_search: false,
            clipboard_card_size: "Large".into(),
            clipboard_history_size: 200,
            clipboard_clear_on_quit: false,
            clipboard_ignore_passwords: true,
            clipboard_ignore_apps: vec!["com.apple.keychainaccess".into(), "com.apple.Passwords".into()],
            start_expanded: false,
            launch_at_login: false,
            demo_mode: false,
            media_source: "System".into(),
            calendar_id: String::new(),
            island_display_id: 0,
            style: "Glass".into(),
            glass_level: 0.75,
            animation_speed: 1.0,
            menu_style: "Compact".into(),
            open_width: 1025.0,
            open_height: 170.0,
            closed_width: 255.0,
            closed_height: 33.0,
            match_resolution: true,
            rounded_buttons: true,
            menu_labels: true,
            show_extra_workspaces: false,
            content_padding: 18.0,
            nook_enabled: true,
            dividers: false,
            widgets: Widget::DEFAULT.to_vec(),
            media_layout: "Default".into(),
            extra_action: "Shuffle".into(),
            show_synced_lyrics: true,
            audio_spectrum: false,
            pulse_style: "Cards".into(),
            pulse_left: "CPU".into(),
            pulse_right: "RAM".into(),
            pulse_explain: true,
            pulse_interval: 2,
            agent_monitor: true,
            codex_allow_edits: false,
            monitor_craft: false,
            prompter_enabled: false,
            prompter_speed: 34.0,
            prompter_mode: "Scroll".into(),
            prompter_language: "en-US".into(),
            prompter_allow_network: false,
            prompter_text_size: 25.0,
            weather_city: String::new(),
            downloads_folder: String::new(),
            watch_downloads: false,
            global_shortcut: false,
            clipboard_shortcut: false,
            auto_close: true,
            close_delay_ms: 550,
            prevent_close_while_typing: true,
            lock_while_typing: false,
            haptic_feedback: true,
            gestures_on_hover: true,
            gesture_vertical: true,
            gesture_horizontal_media: true,
            gesture_invert_media: false,
            pocket_mode: "Auto".into(),
            pocket_battery: false,
            pocket_shake: true,
            notify_timer: true,
            volume_hud: true,
            volume_hud_style: "Below the notch".into(),
            todos: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
struct IslandDisplay {
    id: u32,
    name: String,
    main: bool,
    notch: bool,
}

fn available_island_displays() -> Vec<IslandDisplay> {
    #[cfg(target_os = "macos")]
    unsafe {
        let mut buffer = [0u8; 8192];
        let length = nooklet_displays_copy(buffer.as_mut_ptr().cast(), buffer.len());
        if length > 0 && length < buffer.len() {
            return serde_json::from_slice(&buffer[..length]).unwrap_or_default();
        }
    }
    Vec::new()
}

#[cfg(all(test, target_os = "macos"))]
#[test]
fn detects_the_current_mac_display_for_island_targeting() {
    let displays = available_island_displays();
    assert!(!displays.is_empty(), "macOS did not return any display targets");
    assert!(displays.iter().any(|display| display.main));
}

struct FileResultCard {
    path: PathBuf,
    subtitle: String,
    lifetime: Duration,
    ready_at: Instant,
}

impl FileResultCard {
    fn new(path: PathBuf, subtitle: String, lifetime: Duration) -> Self {
        Self { path, subtitle, lifetime, ready_at: Instant::now() }
    }
}

struct DeskApp {
    expanded: bool,
    workspace: Workspace,
    workspace_chosen_by_user: bool,
    settings_open: bool,
    settings_section: String,
    tour_step: Option<usize>,
    selected_widget: Widget,
    dragged_widget: Option<Widget>,
    prefs: Preferences,
    screenshot: Option<PathBuf>,
    frames: usize,
    shot_requested: bool,
    positioned: bool,
    last_bottom_position: bool,
    clipboard_bottom_sheet: bool,
    clipboard_shelf_width: f32,
    applied_size: Vec2,
    transition: f32,
    transition_velocity: f32,
    animation_target_expanded: bool,
    native_morph_pending: bool,
    native_morph_animating: bool,
    last_frame: Instant,
    last_hovered: Instant,
    hover_opened_at: Instant,
    opened_by_hover: bool,
    hover_blocked_until_exit: bool,
    native_ring_center: Option<egui::Pos2>,
    native_usage_ring_center: Option<egui::Pos2>,
    pending_activity_bubble: Option<String>,
    activity_bubble_visible: bool,
    pending_result_card: Option<FileResultCard>,
    result_card_visible: bool,
    native_morph_shell: Option<(egui::Rect, f32, f32, f32)>,
    island_gesture: IslandGesture,
    last_haptic_expanded: bool,
    battery: String,
    volume: f32,
    cpu: String,
    ram: String,
    gpu: String,
    disk: String,
    network: String,
    last_network_bytes: Option<(u64, u64)>,
    last_network_sample_at: Option<Instant>,
    last_system_poll: Instant,
    system_rx: Option<std::sync::mpsc::Receiver<SystemSnapshot>>,
    memory_total: u64,
    cpu_history: VecDeque<f32>,
    ram_history: VecDeque<f32>,
    gpu_history: VecDeque<f32>,
    battery_history: VecDeque<f32>,
    disk_history: VecDeque<f32>,
    clipboard: Vec<String>,
    clipboard_images: Vec<ClipboardImage>,
    clipboard_files: Vec<ClipboardFiles>,
    clipboard_favorites: Vec<String>,
    clipboard_search: String,
    clipboard_focus_pending: bool,
    clipboard_filter: String,
    clipboard_app_draft: String,
    last_clip: String,
    last_clip_change: i64,
    last_image_fingerprint: Option<u64>,
    last_clip_poll: Instant,
    files: Vec<PathBuf>,
    pipelines: Vec<pipelines::Pipeline>,
    pipeline_draft_name: String,
    pipeline_draft_steps: Vec<pocket::Action>,
    pipeline_draft_action: pocket::Action,
    pipeline_editor_open: bool,
    pocket_job: Option<PocketJob>,
    pocket_status: String,
    pocket_activity: Option<(String, bool, Instant)>,
    drag_shake: shake::DragShake,
    pocket_floating: bool,
    pocket_anchor: egui::Pos2,
    pocket_opened_from_drag: bool,
    pocket_monitor_enabled: bool,
    hover_monitor_enabled: bool,
    animation_clock_enabled: bool,
    volume_hud_monitor_enabled: bool,
    volume_hud_inline_enabled: bool,
    volume_hud_closed_width: f32,
    spectrum_capture_active: bool,
    spectrum_state: i32,
    spectrum_levels: [f32; 16],
    airpods_events_attempted: bool,
    airpods_events_enabled: bool,
    pocket_wake_context: Box<egui::Context>,
    notes: String,
    note_export_rx: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    prompter: String,
    codex_session_id: Option<String>,
    codex_messages: Vec<CodexChatLine>,
    codex_draft: String,
    codex_rx: Option<std::sync::mpsc::Receiver<CodexEvent>>,
    codex_pid: Option<u32>,
    codex_status: String,
    codex_stopping: bool,
    desktop_codex_thread_id: Option<String>,
    desktop_codex_title: String,
    desktop_codex_messages: Vec<CodexChatLine>,
    desktop_codex_draft: String,
    desktop_codex_rx: Option<std::sync::mpsc::Receiver<codex_desktop::Event>>,
    desktop_codex_decision_tx: Option<std::sync::mpsc::Sender<codex_desktop::Decision>>,
    desktop_codex_approval: Option<codex_desktop::Approval>,
    desktop_codex_question: Option<codex_desktop::UserInput>,
    desktop_codex_question_answers: Vec<String>,
    desktop_codex_allow_edits: bool,
    desktop_codex_pid: Option<u32>,
    desktop_codex_status: String,
    desktop_codex_stopping: bool,
    sheet_goal: String,
    sheet_source: String,
    sheet_result: String,
    sheet_constraints: String,
    sheet_prompt: String,
    prompter_scroll: f32,
    prompter_playing: bool,
    prompter_voice_started: bool,
    prompter_voice_word: usize,
    prompter_last_transcript: String,
    last_prompter_frame: Instant,
    downloads: Vec<DownloadEntry>,
    downloads_error: Option<String>,
    last_download_poll: Instant,
    last_download_sample: Instant,
    downloads_rx: Option<std::sync::mpsc::Receiver<Result<Vec<DownloadEntry>, String>>>,
    video_offer: Option<String>,
    video_url_draft: String,
    video_progress: VideoProgress,
    video_rx: Option<std::sync::mpsc::Receiver<VideoEvent>>,
    video_pid: Option<u32>,
    video_saved_path: Option<PathBuf>,
    video_status: String,
    video_stopping: bool,
    terminal_draft: String,
    terminal_output: String,
    terminal_rx: Option<std::sync::mpsc::Receiver<String>>,
    weather: Weather,
    weather_rx: Option<std::sync::mpsc::Receiver<Weather>>,
    last_weather_poll: Instant,
    airpods: AirPodsStatus,
    airpods_rx: Option<std::sync::mpsc::Receiver<AirPodsStatus>>,
    last_airpods_poll: Instant,
    airpods_refresh_pending: bool,
    airpods_baseline_seen: bool,
    airpods_activity: Option<(String, Instant)>,
    calendar_offset: i32,
    timer_secs: u64,
    timer_end: Option<Instant>,
    timer_done: bool,
    stopwatch_started: Option<Instant>,
    stopwatch_saved: Duration,
    muted_volume: Option<f32>,
    todo_draft: String,
    todo_editing: bool,
    todo_focus_pending: bool,
    media_title: String,
    media_artist: String,
    media_app: Option<String>,
    media_position: f32,
    media_duration: f32,
    media_playing: bool,
    media_rx: Option<std::sync::mpsc::Receiver<MediaSnapshot>>,
    media_action_tx: std::sync::mpsc::Sender<(bool, Option<bool>)>,
    media_action_rx: std::sync::mpsc::Receiver<(bool, Option<bool>)>,
    media_actions_pending: usize,
    last_media_request: Instant,
    airplay_open: bool,
    airplay_devices: Vec<AirPlayDevice>,
    airplay_rx: Option<std::sync::mpsc::Receiver<Result<Vec<AirPlayDevice>, String>>>,
    airplay_status: String,
    queue_open: bool,
    queue_for: Option<(String, String)>,
    queue_rx: Option<std::sync::mpsc::Receiver<music_queue::Preview>>,
    queue_preview: music_queue::Preview,
    mirror_open: bool,
    mirror_error: String,
    mirror_texture: Option<egui::TextureHandle>,
    mirror_sequence: u64,
    mirror_buffer: Vec<u8>,
    artwork_for: Option<(String, String, String)>,
    artwork_rx: Option<std::sync::mpsc::Receiver<Result<Option<egui::ColorImage>, String>>>,
    artwork_texture: Option<egui::TextureHandle>,
    codex_logo: Option<egui::TextureHandle>,
    bookland_mark: Option<egui::TextureHandle>,
    notch_shoulders: Option<[egui::TextureHandle; 2]>,
    lyrics_for: Option<(String, String)>,
    lyrics_rx: Option<std::sync::mpsc::Receiver<Result<Vec<LyricLine>, String>>>,
    lyrics_lines: Vec<LyricLine>,
    lyrics_status: String,
    lyrics_open: bool,
    last_media_poll: Instant,
    toast: Option<(String, Instant)>,
    native_view: usize,
    preview_mode: bool,
    shortcut_registered: bool,
    clipboard_shortcut_registered: bool,
    native_installed: bool,
    glass_rects: [Option<egui::Rect>; 8],
    open_notch_rect: Option<egui::Rect>,
    agents: Vec<AgentSnapshot>,
    agents_initialized: bool,
    agent_completion: Option<AgentCompletion>,
    selected_agent: usize,
    agent_usage_view: bool,
    agent_chat_open: bool,
    agent_chat_desktop: bool,
    last_agent_poll: Instant,
    agent_rx: Option<std::sync::mpsc::Receiver<Vec<AgentSnapshot>>>,
    craft_agents: Vec<AgentSnapshot>,
    craft_rx: Option<std::sync::mpsc::Receiver<Vec<AgentSnapshot>>>,
    last_craft_poll: Instant,
    calendar_status: i32,
    calendar_choices: Vec<CalendarChoice>,
    calendar_events: Vec<CalendarEvent>,
    calendar_snapshot_version: u64,
    calendar_query_pending: bool,
    calendar_month_key: Option<(i32, i32)>,
    calendar_month: CalendarMonth,
    calendar_selected_day: Option<u32>,
    last_calendar_poll: Instant,
    new_event_title: String,
    new_event_date: String,
    new_event_time: String,
    new_event_minutes: u32,
}

fn run_output(program: &str, args: &[&str]) -> Option<String> {
    let result = Command::new(program).args(args).output().ok()?;
    result
        .status
        .success()
        .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
}
fn note_html(note: &str) -> String {
    let mut html = String::from("<div>");
    for ch in note.chars() {
        match ch {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            '"' => html.push_str("&quot;"),
            '\n' => html.push_str("<br>"),
            '\r' => {},
            _ => html.push(ch),
        }
    }
    html.push_str("</div>");
    html
}
fn compose_sheet_prompt(goal: &str, source: &str, result: &str, constraints: &str) -> String {
    let mut prompt = format!("Help me work on the spreadsheet attached to or open in this chat.\n\nGoal: {}",
        goal.trim());
    if !source.trim().is_empty() {
        prompt.push_str(&format!("\nRelevant data, ranges, or columns: {}", source.trim()));
    }
    if !result.trim().is_empty() {
        prompt.push_str(&format!("\nDesired result: {}", result.trim()));
    }
    if !constraints.trim().is_empty() {
        prompt.push_str(&format!("\nConstraints and preferences: {}", constraints.trim()));
    }
    prompt.push_str("\n\nFirst inspect the sheet structure and column headers. Ask a short question only if essential information is missing. Then do the work, check formulas or calculations against sample rows, and summarize which sheets and ranges changed and what you verified.");
    prompt
}
fn export_to_apple_notes(note: &str) -> Result<(), String> {
    const SCRIPT: &str = r#"on run argv
set noteBody to item 1 of argv
tell application "Notes"
    set destinationFolder to default folder of default account
    make new note at destinationFolder with properties {body:noteBody}
end tell
end run"#;
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", SCRIPT, &note_html(note)])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() { Ok(()) } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}
fn parse_battery_status(output: &str) -> Option<u8> {
    output
        .lines()
        .filter(|line| line.contains("Battery"))
        .flat_map(str::split_whitespace)
        .find_map(|part| {
            part.trim_end_matches([';', ','])
                .strip_suffix('%')
                .and_then(|value| value.parse::<u8>().ok())
                .filter(|value| *value <= 100)
        })
}
fn battery_status() -> String {
    run_output("pmset", &["-g", "batt"])
        .and_then(|output| parse_battery_status(&output))
        .map(|value| format!("{value}%"))
        .unwrap_or_else(|| "—".into())
}
fn disk_status() -> String {
    run_output("df", &["-k", "/"])
        .and_then(|output| {
            output
                .lines()
                .last()
                .and_then(|line| line.split_whitespace().nth(4))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "—".into())
}
fn parse_gpu_utilization(output: &str) -> Option<u8> {
    let value = output.split_once("\"Device Utilization %\"=")?.1;
    let digits: String = value.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits.parse::<u8>().ok().filter(|value| *value <= 100)
}
fn gpu_status() -> String {
    run_output("ioreg", &["-r", "-c", "IOAccelerator", "-d", "1"])
        .and_then(|output| parse_gpu_utilization(&output))
        .map(|value| format!("{value}%"))
        .unwrap_or_else(|| "—".into())
}
fn network_bytes() -> Option<(u64, u64)> {
    let route = run_output("route", &["-n", "get", "default"])?;
    let interface = route
        .lines()
        .find_map(|line| line.trim().strip_prefix("interface:").map(str::trim))?;
    let output = run_output("netstat", &["-ibn", "-I", interface])?;
    let mut lines = output.lines();
    let header: Vec<_> = lines.next()?.split_whitespace().collect();
    let inbound = header.iter().position(|name| *name == "Ibytes")?;
    let outbound = header.iter().position(|name| *name == "Obytes")?;
    lines
        .filter_map(|line| {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.first().copied() != Some(interface) {
                return None;
            }
            Some((
                parts.get(inbound)?.parse::<u64>().ok()?,
                parts.get(outbound)?.parse::<u64>().ok()?,
            ))
        })
        .max_by_key(|(received, sent)| received + sent)
}
struct SystemSnapshot {
    sampled_at: Instant,
    battery: String,
    volume: f32,
    disk: String,
    network_bytes: Option<(u64, u64)>,
    cpu: Option<String>,
    ram: Option<String>,
    gpu: Option<String>,
}
fn parse_top_metrics(output: &str, memory_total: u64) -> (Option<String>, Option<String>) {
    let cpu = output.lines().find(|line| line.starts_with("CPU usage:"))
        .and_then(|line| {
            let values: Vec<f32> = line.split_whitespace()
                .filter_map(|part| part.trim_end_matches('%').parse().ok())
                .collect();
            (values.len() >= 2).then(|| format!("{:.0}%", values[0] + values[1]))
        });
    let ram = output.lines().find(|line| line.starts_with("PhysMem:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .map(|used| {
            parse_memory_bytes(used)
                .filter(|_| memory_total > 0)
                .map(|bytes| format!("{:.0}%", bytes / memory_total as f64 * 100.0))
                .unwrap_or_else(|| used.to_owned())
        });
    (cpu, ram)
}
fn collect_system_snapshot(include_gpu: bool, memory_total: u64) -> SystemSnapshot {
    let battery = battery_status();
    let volume = system_volume();
    let disk = disk_status();
    let network_bytes = network_bytes();
    let (cpu, ram) = run_output("top", &["-l", "1", "-n", "0"])
        .map(|output| parse_top_metrics(&output, memory_total))
        .unwrap_or_default();
    let gpu = include_gpu.then(gpu_status);
    SystemSnapshot {
        sampled_at: Instant::now(), battery, volume, disk,
        network_bytes, cpu, ram, gpu,
    }
}
fn network_rate(bytes: u64, elapsed: f32) -> String {
    let per_second = bytes as f32 / elapsed.max(0.1);
    if per_second >= 1_000_000.0 {
        format!("{:.1} MB/s", per_second / 1_000_000.0)
    } else {
        format!("{:.0} KB/s", per_second / 1_000.0)
    }
}
fn parse_memory_bytes(text: &str) -> Option<f64> {
    let text = text.trim().trim_end_matches(',');
    let unit = text.chars().last()?;
    let multiplier = match unit {
        'K' => 1024_f64,
        'M' => 1024_f64.powi(2),
        'G' => 1024_f64.powi(3),
        _ => return text.parse().ok(),
    };
    text[..text.len() - 1].parse::<f64>().ok().map(|value| value * multiplier)
}
fn percent_value(text: &str) -> Option<f32> {
    text.trim_end_matches('%').parse::<f32>().ok()
}
fn record_sample(history: &mut VecDeque<f32>, value: Option<f32>) {
    if let Some(value) = value.filter(|value| value.is_finite()) {
        history.push_back(value.clamp(0.0, 100.0));
        if history.len() > 24 { history.pop_front(); }
    }
}

#[cfg(test)]
mod pulse_tests {
    use super::*;

    #[test]
    fn memory_units_and_history_are_bounded() {
        assert_eq!(parse_memory_bytes("1.5G"), Some(1.5 * 1024_f64.powi(3)));
        assert_eq!(parse_memory_bytes("512M"), Some(512.0 * 1024_f64.powi(2)));
        let mut history = VecDeque::new();
        for value in 0..30 { record_sample(&mut history, Some(value as f32)); }
        assert_eq!(history.len(), 24);
        assert_eq!(history.front(), Some(&6.0));
    }
    #[test]
    fn gpu_reading_uses_device_utilization_only() {
        let statistics = "\"PerformanceStatistics\" = {\"Renderer Utilization %\"=67,\"Device Utilization %\"=43}";
        assert_eq!(parse_gpu_utilization(statistics), Some(43));
        assert_eq!(parse_gpu_utilization("\"Device Utilization %\"=101"), None);
        assert_eq!(parse_gpu_utilization("\"Renderer Utilization %\"=67"), None);
    }
    #[test]
    fn battery_reading_accepts_pmset_semicolon() {
        let output = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=5439587)\t100%; charged; 0:00 remaining present: true";
        assert_eq!(parse_battery_status(output), Some(100));
        assert_eq!(parse_battery_status("Now drawing from 'AC Power'"), None);
        assert_eq!(parse_battery_status("Battery 101%; charged"), None);
    }
    #[test]
    fn background_top_snapshot_keeps_cpu_and_ram_values() {
        let output = "CPU usage: 12.0% user, 8.0% sys, 80.0% idle\nPhysMem: 16G used (2G wired), 16G unused.";
        assert_eq!(parse_top_metrics(output, 32 * 1024_u64.pow(3)),
            (Some("20%".into()), Some("50%".into())));
        assert_eq!(parse_top_metrics("", 32 * 1024_u64.pow(3)), (None, None));
    }
}
fn system_volume() -> f32 {
    #[cfg(target_os = "macos")]
    {
        let scalar = unsafe { bookland_system_volume_get() };
        if scalar >= 0.0 {
            return scalar * 100.0;
        }
    }
    run_output(
        "osascript",
        &["-e", "output volume of (get volume settings)"],
    )
    .and_then(|v| v.parse::<f32>().ok())
    .unwrap_or(50.0)
}
fn set_system_volume(value: f32) {
    #[cfg(target_os = "macos")]
    if unsafe { bookland_system_volume_set(value / 100.0) } != 0 {
        return;
    }
    let script = format!("set volume output volume {}", value.round() as i32);
    let _ = run_output("osascript", &["-e", &script]);
}
fn running_media_app(source: &str) -> Option<String> {
    for app in ["Music", "Spotify"] {
        if source != "Auto" && source != "System" && source != app {
            continue;
        }
        if Command::new("pgrep")
            .args(["-x", app])
            .output()
            .ok()?
            .status
            .success()
        {
            return Some(app.into());
        }
    }
    None
}
fn media_script(app: &str, action: &str) -> Option<String> {
    osascript_with_timeout(&format!("tell application \"{}\" to {}", app, action),
        Duration::from_secs(3))
}
fn osascript_with_timeout(script: &str, timeout: Duration) -> Option<String> {
    osascript_args_with_timeout(&["-e", script], timeout)
}
fn osascript_args_with_timeout(args: &[&str], timeout: Duration) -> Option<String> {
    let mut child = Command::new("/usr/bin/osascript")
        .args(args)
        .stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().ok()?;
                return status.success().then(||
                    String::from_utf8_lossy(&output.stdout).trim().to_owned());
            }
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}
fn system_media_script(mode: &[&str]) -> Option<String> {
    let mut args = vec!["-l", "JavaScript", "-e", include_str!("now_playing.jxa")];
    args.extend_from_slice(mode);
    osascript_args_with_timeout(&args, Duration::from_secs(3))
}
fn system_media_control(action: &str) -> bool {
    if let Some(seconds) = action.strip_prefix("set player position to ")
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && (0.0..=86400.0).contains(value)) {
        return media_remote_adapter::seek(seconds)
            || system_media_script(&["seek", &seconds.to_string()]).as_deref() == Some("sent");
    }
    let (command, name) = match action {
        "play" => (0, "play"),
        "pause" => (1, "pause"),
        "next track" => (4, "next"),
        "previous track" => (5, "previous"),
        "skip back 15 seconds" => (12, ""),
        "skip forward 15 seconds" => (13, ""),
        "shuffle" => (6, "shuffle"),
        _ => return false,
    };
    media_remote_adapter::send(command)
        || (!name.is_empty()
            && system_media_script(&["control", name]).as_deref() == Some("sent"))
}
fn is_browser_media_app(app: Option<&str>) -> bool {
    matches!(app, Some("Google Chrome" | "Safari" | "Arc" | "Microsoft Edge"))
}
fn media_skip_command(backward: bool) -> &'static str {
    if backward { "previous track" } else { "next track" }
}
fn browser_skip_target(position: f32, duration: f32, backward: bool) -> Option<f32> {
    (position.is_finite() && duration.is_finite() && duration > 0.0).then(|| {
        (position + if backward { -15.0 } else { 15.0 }).clamp(0.0, duration)
    })
}
fn system_artwork() -> Result<Option<Vec<u8>>, String> {
    let encoded = system_media_script(&["artwork"])
        .ok_or_else(|| "System artwork query timed out".to_owned())?;
    if encoded.is_empty() { return Ok(None); }
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded.trim())
        .map_err(|error| error.to_string())?;
    if bytes.is_empty() || bytes.len() > 1_000_000 { return Ok(None); }
    Ok(Some(bytes))
}
#[cfg(test)]
mod media_script_tests {
    use super::*;

    #[test]
    fn apple_script_returns_output_and_times_out() {
        assert_eq!(osascript_with_timeout("return 42", Duration::from_secs(2)), Some("42".into()));
        let started = Instant::now();
        assert_eq!(osascript_with_timeout("delay 2", Duration::from_millis(100)), None);
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
struct MediaSnapshot {
    app: Option<String>,
    title: String,
    artist: String,
    position: f32,
    duration: f32,
    playing: bool,
}
fn system_media() -> Option<MediaSnapshot> {
    let value = media_remote_adapter::now_playing().or_else(|| {
        serde_json::from_str(&system_media_script(&[])?).ok()
    })?;
    system_media_from_value(&value)
}
fn system_media_from_value(value: &serde_json::Value) -> Option<MediaSnapshot> {
    let title = value["title"].as_str().or_else(|| value["Title"].as_str())?.trim();
    if title.is_empty() { return None; }
    let bundle = value["parentApplicationBundleIdentifier"].as_str()
        .or_else(|| value["bundleIdentifier"].as_str())
        .or_else(|| value["bundle"].as_str()).unwrap_or("");
    let app = match bundle {
        "com.apple.Music" => "Music",
        "com.spotify.client" => "Spotify",
        "com.google.Chrome" => "Google Chrome",
        "com.apple.Safari" => "Safari",
        "company.thebrowser.Browser" => "Arc",
        "com.microsoft.edgemac" => "Microsoft Edge",
        _ => bundle.rsplit('.').next().filter(|name| !name.is_empty())
            .unwrap_or("System media"),
    };
    Some(MediaSnapshot {
        app: Some(app.into()),
        title: title.into(),
        artist: value["artist"].as_str().or_else(|| value["Artist"].as_str())
            .unwrap_or("").into(),
        position: value["elapsedTimeNow"].as_f64()
            .or_else(|| value["elapsedTime"].as_f64())
            .or_else(|| value["ElapsedTime"].as_f64())
            .unwrap_or(0.0).max(0.0) as f32,
        duration: value["duration"].as_f64().or_else(|| value["Duration"].as_f64())
            .unwrap_or(0.0).max(0.0) as f32,
        playing: value["playing"].as_bool().unwrap_or(false),
    })
}
#[cfg(test)]
mod system_media_tests {
    use super::{browser_skip_target, media_skip_command, system_media_from_value};

    #[test]
    fn media_skip_changes_tracks() {
        assert_eq!(media_skip_command(true), "previous track");
        assert_eq!(media_skip_command(false), "next track");
    }

    #[test]
    fn browser_skip_clamps_to_the_video_bounds() {
        assert_eq!(browser_skip_target(10.0, 120.0, true), Some(0.0));
        assert_eq!(browser_skip_target(10.0, 120.0, false), Some(25.0));
        assert_eq!(browser_skip_target(115.0, 120.0, false), Some(120.0));
        assert_eq!(browser_skip_target(f32::NAN, 120.0, false), None);
    }

    #[test]
    fn adapter_snapshot_uses_parent_player_and_live_elapsed_time() {
        let input = serde_json::json!({
            "title": "A track", "artist": "An artist", "bundleIdentifier": "com.apple.WebKit.GPU",
            "parentApplicationBundleIdentifier": "com.apple.Safari", "elapsedTime": 10.0,
            "elapsedTimeNow": 12.5, "duration": 90.0, "playing": true
        });
        let snapshot = system_media_from_value(&input).unwrap();
        assert_eq!(snapshot.app.as_deref(), Some("Safari"));
        assert_eq!(snapshot.position, 12.5);
        assert_eq!(snapshot.duration, 90.0);
        assert!(snapshot.playing);
    }

    #[test]
    fn legacy_jxa_snapshot_remains_usable() {
        let input = serde_json::json!({
            "Title": "Another track", "Artist": "Another artist", "bundle": "com.google.Chrome",
            "ElapsedTime": 30.0, "Duration": 120.0, "playing": false
        });
        let snapshot = system_media_from_value(&input).unwrap();
        assert_eq!(snapshot.app.as_deref(), Some("Google Chrome"));
        assert_eq!(snapshot.title, "Another track");
        assert_eq!(snapshot.position, 30.0);
    }
}
#[cfg(all(test, target_os = "macos"))]
mod audio_spectrum_tests {
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    #[test]
    fn real_tone_moves_energy_to_higher_frequency_bands() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut low = [0.0_f32; 16];
        let mut high = [0.0_f32; 16];
        unsafe {
            super::bookland_audio_spectrum_test_tone(220.0, 0,
                low.as_mut_ptr(), low.len());
            super::bookland_audio_spectrum_test_tone(2200.0, 0,
                high.as_mut_ptr(), high.len());
        }
        let peak = |levels: &[f32; 16]| levels.iter().enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index).unwrap();
        assert!(low.iter().all(|value| value.is_finite() && (0.0..=1.0).contains(value)));
        assert!(high.iter().all(|value| value.is_finite() && (0.0..=1.0).contains(value)));
        assert!(low[peak(&low)] > 0.2);
        assert!(high[peak(&high)] > 0.2);
        assert!(peak(&high) > peak(&low) + 3,
            "frequency bands should distinguish a 220 Hz tone from 2.2 kHz");
    }
    #[test]
    fn stereo_phase_and_planar_layout_keep_real_audio_visible() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut interleaved = [0.0_f32; 16];
        let mut antiphase = [0.0_f32; 16];
        let mut planar = [0.0_f32; 16];
        unsafe {
            super::bookland_audio_spectrum_test_tone(440.0, 0,
                interleaved.as_mut_ptr(), interleaved.len());
            super::bookland_audio_spectrum_test_tone(440.0, 1,
                antiphase.as_mut_ptr(), antiphase.len());
            super::bookland_audio_spectrum_test_tone(440.0, 2,
                planar.as_mut_ptr(), planar.len());
        }
        let peak = |levels: &[f32; 16]| levels.iter().copied()
            .fold(0.0_f32, f32::max);
        assert!(peak(&antiphase) > 0.2,
            "opposite-phase stereo channels must not cancel the spectrum");
        assert!(peak(&planar) > 0.2,
            "Core Audio's non-interleaved stereo layout must remain visible");
        assert!((peak(&interleaved) - peak(&antiphase)).abs() < 0.04);
        assert!((peak(&interleaved) - peak(&planar)).abs() < 0.04);
    }
}
fn read_media(source: &str) -> MediaSnapshot {
    if (source == "System" || source == "Auto") && cfg!(target_os = "macos") {
        if let Some(snapshot) = system_media() { return snapshot; }
    }
    let app = running_media_app(source);
    let mut snapshot = MediaSnapshot {
        app: app.clone(), title: "Nothing playing".into(),
        artist: "Press Play for music".into(),
        position: 0.0, duration: 0.0, playing: false,
    };
    if let Some(app) = app {
        if let Some(title) = media_script(&app, "name of current track").filter(|value| !value.is_empty()) {
            snapshot.title = title;
            snapshot.artist = media_script(&app, "artist of current track").unwrap_or_default();
            snapshot.position = media_script(&app, "player position")
                .and_then(|value| value.parse().ok()).unwrap_or(0.0);
            let duration = media_script(&app, "duration of current track")
                .and_then(|value| value.parse::<f32>().ok()).unwrap_or(0.0);
            snapshot.duration = if app == "Spotify" { duration / 1000.0 } else { duration };
            snapshot.playing = media_script(&app, "player state")
                .is_some_and(|state| state == "playing");
        }
    }
    snapshot
}
fn format_time(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}
fn compact_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 { format!("{:.1}M", tokens as f64 / 1_000_000.0) }
    else if tokens >= 1_000 { format!("{:.1}K", tokens as f64 / 1_000.0) }
    else { tokens.to_string() }
}
fn codex_workspace() -> PathBuf {
    if let Some(path) = std::env::var_os("BOOKLAND_CODEX_WORKSPACE") {
        return PathBuf::from(path);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let project = PathBuf::from(home).join("Code/Bookland/app");
        if project.is_dir() { return project; }
    }
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if project.is_dir() { return project; }
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}
fn desktop_codex_thread_id(path: &Path) -> Option<String> {
    let home = std::env::var_os("CODEX_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))?;
    if !path.starts_with(home.join("sessions")) { return None; }
    let stem = path.file_stem()?.to_str()?;
    let id = stem.get(stem.len().checked_sub(36)?..)?;
    let bytes = id.as_bytes();
    if [8, 13, 18, 23].iter().all(|&index| bytes[index] == b'-')
        && bytes.iter().enumerate().all(|(index, byte)|
            [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit()) {
        Some(id.to_owned())
    } else { None }
}

#[cfg(test)]
mod desktop_codex_thread_tests {
    use super::desktop_codex_thread_id;
    use std::path::PathBuf;

    #[test]
    fn only_codex_session_rollouts_can_address_desktop_threads() {
        let home = std::env::var_os("CODEX_HOME").map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".codex"))).unwrap();
        let id = "01a2034c-68fe-7d00-805a-96e35c5cf102";
        let session = home.join("sessions/2026/11/23")
            .join(format!("rollout-2026-11-23T00-00-00-{id}.jsonl"));
        assert_eq!(desktop_codex_thread_id(&session).as_deref(), Some(id));
        assert!(desktop_codex_thread_id(&PathBuf::from("/tmp/rollout-2026-11-23T00-00-00-")
            .join(id)).is_none());
        assert!(desktop_codex_thread_id(&home.join("sessions/bad-id.jsonl")).is_none());
    }
}
fn downloads_root(folder: &str) -> Option<PathBuf> {
    if !folder.is_empty() { return Some(PathBuf::from(folder)); }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads"))
}
fn preferences_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library/Application Support/Bookland")
            .join("preferences.json")
    })
}
fn previous_preferences_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library/Application Support/Nooklet/preferences.json")
    })
}
fn data_path() -> Option<PathBuf> {
    preferences_path().and_then(|path| path.parent().map(|parent| parent.join("data.json")))
}
fn clipboard_images_dir() -> Option<PathBuf> {
    data_path().and_then(|path| path.parent().map(|parent| parent.join("ClipboardImages")))
}
fn previous_data_path() -> Option<PathBuf> {
    previous_preferences_path().and_then(|path| path.parent().map(|parent| parent.join("data.json")))
}
fn login_agent_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home)
        .join("Library/LaunchAgents/local.bookland.app.plist"))
}
fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let path = login_agent_path().ok_or("Home folder is unavailable")?;
    if !enabled {
        if path.exists() { std::fs::remove_file(&path).map_err(|err| err.to_string())?; }
        return Ok(());
    }
    let executable = std::env::current_exe().map_err(|err| err.to_string())?;
    let executable = executable.to_string_lossy()
        .replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&apos;");
    let plist = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\"><dict>\n\
<key>Label</key><string>local.bookland.app</string>\n\
<key>ProgramArguments</key><array><string>{executable}</string></array>\n\
<key>RunAtLoad</key><true/>\n\
</dict></plist>\n");
    std::fs::create_dir_all(path.parent().ok_or("Invalid login item path")?)
        .map_err(|err| err.to_string())?;
    std::fs::write(&path, plist).map_err(|err| err.to_string())
}
fn label(ui: &mut egui::Ui, text: &str, size: f32, color: Color32) {
    ui.label(RichText::new(text).size(size).color(color));
}
// egui 0.32 lays glyphs left to right, even when the installed font contains
// Hebrew. Reorder dynamic music metadata into visual order before painting it.
fn mirrored_bidi_punctuation(ch: char) -> char {
    match ch {
        '(' => ')', ')' => '(', '[' => ']', ']' => '[',
        '{' => '}', '}' => '{', '<' => '>', '>' => '<',
        '«' => '»', '»' => '«', '‹' => '›', '›' => '‹',
        _ => ch,
    }
}
fn music_visual_text(text: &str) -> (String, bool) {
    let bidi = unicode_bidi::BidiInfo::new(text, None);
    let Some(paragraph) = bidi.paragraphs.first() else {
        return (text.to_owned(), false);
    };
    let (levels, runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
    let mut visual = String::with_capacity(text.len());
    for run in runs {
        if levels[run.start].is_rtl() {
            visual.extend(text[run].chars().rev().map(mirrored_bidi_punctuation));
        } else {
            visual.push_str(&text[run]);
        }
    }
    (visual, paragraph.level.is_rtl())
}
fn complete_music_prefix(prefix: &str) -> &str {
    let last_open = prefix.rfind('(');
    let last_close = prefix.rfind(')');
    let complete = match (last_open, last_close) {
        (Some(open), Some(close)) if open > close => &prefix[..open],
        (Some(open), None) => &prefix[..open],
        _ => prefix,
    };
    complete.trim_end()
}
fn truncate_rtl_visual(text: &str, max_width: f32, mut width: impl FnMut(&str) -> f32) -> String {
    let full = music_visual_text(text).0;
    if width(&full) <= max_width { return full; }
    let chars: Vec<char> = text.chars().collect();
    let mut low = 0;
    let mut high = chars.len();
    while low < high {
        let middle = (low + high + 1) / 2;
        let prefix: String = chars[..middle].iter().collect();
        let candidate = music_visual_text(&format!("{}…", complete_music_prefix(&prefix))).0;
        if width(&candidate) <= max_width { low = middle; }
        else { high = middle - 1; }
    }
    if low == 0 { return "…".into(); }
    let prefix: String = chars[..low].iter().collect();
    music_visual_text(&format!("{}…", complete_music_prefix(&prefix))).0
}
fn music_text(ui: &mut egui::Ui, text: &str, size: f32, color: Color32,
              strong: bool, truncate: bool) {
    let (mut visual, rtl) = music_visual_text(text);
    if rtl && truncate {
        let available = (ui.available_width() - 8.0).max(0.0);
        let font = FontId::proportional(size);
        visual = truncate_rtl_visual(text, available, |candidate| {
            ui.ctx().fonts(|fonts| fonts.layout_no_wrap(
                candidate.to_owned(), font.clone(), color).size().x)
        });
    }
    let mut rich = RichText::new(visual).size(size).color(color);
    if strong { rich = rich.strong(); }
    let mut widget = egui::Label::new(rich)
        .halign(if rtl { Align::RIGHT } else { Align::LEFT });
    if truncate { widget = widget.truncate(); }
    let response = if truncate {
        ui.add_sized([ui.available_width(), size + 5.0], widget)
    } else {
        ui.add(widget)
    };
    response.widget_info(|| egui::WidgetInfo::labeled(
        egui::WidgetType::Label, true, text));
}
#[cfg(test)]
mod music_bidi_tests {
    use super::{music_visual_text, truncate_rtl_visual};

    #[test]
    fn hebrew_music_text_is_in_visual_order() {
        assert_eq!(music_visual_text("שלום עולם"), ("םלוע םולש".into(), true));
        assert_eq!(music_visual_text("Hello World"), ("Hello World".into(), false));
    }

    #[test]
    fn mixed_music_text_keeps_latin_word_order() {
        let (visual, rtl) = music_visual_text("שלום ABC 123");
        assert!(rtl);
        assert!(visual.contains("ABC 123"));
    }

    #[test]
    fn hebrew_title_mirrors_english_credit_parentheses() {
        let (visual, rtl) = music_visual_text(
            "שירז אברהם - מה עשיתי לך ילד (Prod. By Moshe Saboni & Nadav)");
        assert!(rtl);
        assert!(visual.starts_with("(Prod. By Moshe Saboni & Nadav)"), "{visual}");
        assert!(!visual.contains(")Prod."));
    }

    #[test]
    fn narrow_rtl_title_keeps_logical_beginning() {
        let text = "שירז אברהם - מה עשיתי לך ילד (Prod. By Moshe Saboni & Nadav)";
        let visual = truncate_rtl_visual(text, 24.0, |candidate| candidate.chars().count() as f32);
        assert!(visual.starts_with('…'), "{visual}");
        assert!(visual.ends_with("םהרבא זריש"), "{visual}");
        assert!(!visual.contains("Prod."));
    }

    #[test]
    fn truncated_credit_does_not_leave_open_parenthesis() {
        let text = "שירז אברהם - מה עשיתי לך ילד (Prod. By Moshe Saboni & Nadav)";
        let visual = truncate_rtl_visual(text, 45.0, |candidate| candidate.chars().count() as f32);
        assert!(visual.starts_with('…'), "{visual}");
        assert!(!visual.contains("Prod."), "{visual}");
        assert!(!visual.contains('(') && !visual.contains(')'), "{visual}");
    }
}
#[cfg(target_os = "macos")]
fn speech_string(reader: unsafe extern "C" fn(*mut std::ffi::c_char, usize) -> usize) -> String {
    let mut buffer = vec![0u8; 8192];
    let length = unsafe { reader(buffer.as_mut_ptr().cast(), buffer.len()) };
    if length == 0 || length >= buffer.len() { return String::new(); }
    String::from_utf8_lossy(&buffer[..length]).into_owned()
}
fn small_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(TEXT))
            .fill(CARD_2)
            .min_size(Vec2::new(34.0, 29.0)),
    )
}
fn clipboard_kind(value: &str) -> &'static str {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") { return "Links"; }
    let hex = value.strip_prefix('#').unwrap_or("");
    if [3, 4, 6, 8].contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return "Colors";
    }
    if (value.starts_with('/') || lower.starts_with("file://")) && !value.contains('\n') {
        return "Files";
    }
    "Text"
}

#[cfg(test)]
mod clipboard_tests {
    use super::clipboard_kind;
    #[test]
    fn classifies_visible_clipboard_types() {
        assert_eq!(clipboard_kind("https://example.com/a"), "Links");
        assert_eq!(clipboard_kind("#a0b1c2"), "Colors");
        assert_eq!(clipboard_kind("/Users/test/report.pdf"), "Files");
        assert_eq!(clipboard_kind("Notes about #abc"), "Text");
    }
}
fn glass_sheen(rect: egui::Rect, opacity: u8) -> egui::Shape {
    let mut mesh = egui::Mesh::default();
    let xs = [rect.left(), rect.left() + rect.width() * 0.17,
        rect.right() - rect.width() * 0.17, rect.right()];
    let ys = [rect.top(), rect.top() + rect.height() * 0.17,
        rect.bottom() - rect.height() * 0.17, rect.bottom()];
    for (y, py) in ys.into_iter().enumerate() {
        for (x, px) in xs.into_iter().enumerate() {
            let color = if x == 0 || x == 3 || y == 0 || y == 3 {
                Color32::TRANSPARENT
            } else {
                match (x, y) {
                    (1, 1) => Color32::from_rgba_unmultiplied(250, 252, 255, opacity),
                    (2, 1) => Color32::from_rgba_unmultiplied(218, 226, 238, opacity / 2),
                    (1, 2) => Color32::from_rgba_unmultiplied(172, 184, 202, opacity / 3),
                    _ => Color32::from_rgba_unmultiplied(110, 126, 148, opacity / 4),
                }
            };
            mesh.colored_vertex(egui::pos2(px, py), color);
        }
    }
    for y in 0..3 {
        for x in 0..3 {
            let a = (y * 4 + x) as u32;
            mesh.add_triangle(a, a + 1, a + 4);
            mesh.add_triangle(a + 1, a + 5, a + 4);
        }
    }
    egui::Shape::mesh(mesh)
}
fn glass_surface_fill() -> Color32 {
    // Let NSGlassEffectView sample the desktop behind the island. A painted
    // gray surface made the panel look white on every wallpaper.
    Color32::from_rgba_unmultiplied(12, 16, 23, 24)
}
fn island_upper_shade(rect: egui::Rect) -> egui::Shape {
    let mut mesh = egui::Mesh::default();
    // Keep only the camera band opaque. The rest remains actual system glass,
    // so its color and specular treatment follow the background.
    let band = (33.0 / rect.height().max(1.0)).min(1.0);
    let shoulder = (47.0 / rect.height().max(1.0)).min(1.0);
    let clear = (69.0 / rect.height().max(1.0)).min(1.0);
    let rows = [(0.0, 255), (band, 255), (shoulder, 120), (clear, 0), (1.0, 0)];
    for (fraction, alpha) in rows {
        let color = Color32::from_rgba_unmultiplied(0, 0, 0, alpha);
        let y = rect.top() + rect.height() * fraction;
        mesh.colored_vertex(egui::pos2(rect.left(), y), color);
        mesh.colored_vertex(egui::pos2(rect.right(), y), color);
    }
    for row in 0..rows.len() - 1 {
        let left = (row * 2) as u32;
        mesh.add_triangle(left, left + 1, left + 2);
        mesh.add_triangle(left + 1, left + 3, left + 2);
    }
    egui::Shape::mesh(mesh)
}
fn paint_codex_logo(painter: &egui::Painter, center: egui::Pos2,
                    diameter: f32, color: Color32, logo: Option<&egui::TextureHandle>) {
    if let Some(logo) = logo {
        painter.image(logo.id(), egui::Rect::from_center_size(center, Vec2::splat(diameter)),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), color);
    } else {
        painter.circle_stroke(center, diameter * 0.38,
            Stroke::new((diameter * 0.08).max(0.8), color));
    }
}
fn paint_agent_progress_icon(painter: &egui::Painter, center: egui::Pos2,
                             color: Color32) {
    let stroke = Stroke::new(1.25, color);
    for y in [-3.4, 3.4] {
        painter.circle_stroke(center + Vec2::new(-4.7, y), 1.45, stroke);
        painter.line_segment([center + Vec2::new(-0.8, y),
            center + Vec2::new(5.4, y)], stroke);
    }
}
fn paint_agent_usage_icon(painter: &egui::Painter, center: egui::Pos2,
                          color: Color32) {
    let stroke = Stroke::new(1.2, color);
    painter.circle_stroke(center, 5.7, stroke);
    painter.line_segment([center, center + Vec2::new(2.9, -2.5)], stroke);
    painter.circle_filled(center, 0.85, color);
    for offset in [Vec2::new(-2.8, -2.6), Vec2::new(-4.0, 0.4),
                   Vec2::new(-2.0, 2.8)] {
        painter.circle_filled(center + offset, 0.55, color);
    }
}
fn agents_upper_shade(rect: egui::Rect) -> egui::Shape {
    island_upper_shade(rect)
}
fn hover_allowed_after_close(blocked_until_exit: &mut bool, physically_hovered: bool) -> bool {
    if !physically_hovered {
        *blocked_until_exit = false;
    }
    physically_hovered && !*blocked_until_exit
}
fn hover_close_guard(speed: f32, reduce_motion: bool) -> Duration {
    if reduce_motion {
        Duration::from_millis(250)
    } else {
        Duration::from_millis((600.0 / f64::from(speed.clamp(0.65, 1.0))).round() as u64)
    }
}
fn spring_step(position: f32, velocity: f32, target: f32,
               omega: f32, damping: f32, dt: f32) -> (f32, f32) {
    let displacement = position - target;
    let decay = (-damping * omega * dt).exp();
    if damping < 1.0 {
        let frequency = omega * (1.0 - damping * damping).sqrt();
        let sine = (frequency * dt).sin();
        let cosine = (frequency * dt).cos();
        let weighted = (velocity + damping * omega * displacement) / frequency;
        let next = decay * (displacement * cosine + weighted * sine);
        let next_velocity = decay * (velocity * cosine
            - (omega * omega * displacement + damping * omega * velocity)
                / frequency * sine);
        (target + next, next_velocity)
    } else {
        let weighted = velocity + omega * displacement;
        (target + decay * (displacement + weighted * dt),
            decay * (velocity - omega * weighted * dt))
    }
}
fn spring_frame_dt(elapsed: Duration, target_changed: bool) -> f32 {
    let seconds = elapsed.as_secs_f32().min(1.0);
    if target_changed {
        // An input can arrive after the closed island's low-frequency idle
        // redraw. That idle time is not elapsed animation time.
        seconds.clamp(1.0 / 120.0, 1.0 / 60.0)
    } else {
        seconds
    }
}
#[repr(C)]
struct NativeMorphFrame {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    top_radius: f64,
    bottom_radius: f64,
    opacity: f64,
}

fn native_morph_frames(center_x: f32, top_y: f32,
    closed: (f32, f32), open: (f32, f32),
    position: f32, velocity: f32, target: f32, omega: f32, damping: f32)
    -> Vec<NativeMorphFrame> {
    let mut frames = Vec::with_capacity(61);
    let (mut position, mut velocity) = (position, velocity);
    for index in 0..=60 {
        let progress = position.clamp(0.0, 1.02);
        let width = egui::lerp(closed.0..=open.0, progress);
        let height = egui::lerp(closed.1..=open.1, progress);
        let reveal = ((progress - 0.82) / 0.14).clamp(0.0, 1.0);
        frames.push(NativeMorphFrame {
            x: f64::from(center_x - width * 0.5),
            y: f64::from(top_y),
            width: f64::from(width),
            height: f64::from(height),
            top_radius: f64::from(((1.0 - progress) * 10.0).max(0.0)),
            bottom_radius: f64::from(17.0 + progress * 12.0),
            opacity: f64::from(1.0 - reveal),
        });
        if index > 1 && (position - target).abs() < 0.005
            && velocity.abs() < 0.02 { break; }
        (position, velocity) = spring_step(position, velocity, target,
            omega, damping, 1.0 / 60.0);
    }
    if let Some(last) = frames.last_mut() {
        let width = egui::lerp(closed.0..=open.0, target);
        let height = egui::lerp(closed.1..=open.1, target);
        last.x = f64::from(center_x - width * 0.5);
        last.width = f64::from(width);
        last.height = f64::from(height);
        last.top_radius = f64::from((1.0 - target) * 10.0);
        last.bottom_radius = f64::from(17.0 + target * 12.0);
        last.opacity = f64::from(1.0 - ((target - 0.82) / 0.14).clamp(0.0, 1.0));
    }
    frames
}
#[cfg(test)]
mod hover_gate_tests {
    use std::time::Duration;
    use super::{hover_allowed_after_close, hover_close_guard, native_morph_frames,
        spring_frame_dt, spring_step};

    #[test]
    fn explicit_close_requires_pointer_exit_before_hover_reopens() {
        let mut blocked = true;
        assert!(!hover_allowed_after_close(&mut blocked, true));
        assert!(blocked);
        assert!(!hover_allowed_after_close(&mut blocked, false));
        assert!(!blocked);
        assert!(hover_allowed_after_close(&mut blocked, true));
    }

    #[test]
    fn close_guard_follows_measured_speeds_and_reduce_motion() {
        assert_eq!(hover_close_guard(1.0, false).as_millis(), 600);
        assert_eq!(hover_close_guard(0.8, false).as_millis(), 750);
        assert_eq!(hover_close_guard(0.65, false).as_millis(), 923);
        assert_eq!(hover_close_guard(1.0, true).as_millis(), 250);
    }

    #[test]
    fn spring_position_is_stable_across_frame_intervals() {
        let omega = std::f32::consts::TAU / 0.5;
        let once = spring_step(0.0, 0.0, 1.0, omega, 0.82, 0.08);
        let first = spring_step(0.0, 0.0, 1.0, omega, 0.82, 0.04);
        let twice = spring_step(first.0, first.1, 1.0, omega, 0.82, 0.04);
        assert!((once.0 - twice.0).abs() < 0.0001);
        assert!((once.1 - twice.1).abs() < 0.0001);
        let (over, _) = spring_step(0.0, 0.0, 1.0, omega, 0.82, 0.42);
        assert!(over > 1.0 && over < 1.02);
    }

    #[test]
    fn first_spring_frame_does_not_include_closed_idle_time() {
        let dt = spring_frame_dt(Duration::from_millis(350), true);
        assert!((dt - 1.0 / 60.0).abs() < f32::EPSILON);
        assert!((spring_frame_dt(Duration::ZERO, true) - 1.0 / 120.0).abs()
            < f32::EPSILON);
        assert!((spring_frame_dt(Duration::from_millis(30), false) - 0.03).abs()
            < f32::EPSILON);
    }

    #[test]
    fn native_morph_samples_preserve_the_top_edge_and_finish_at_the_target() {
        let frames = native_morph_frames(300.0, 0.0, (255.0, 33.0),
            (585.0, 230.0), 0.0, 0.0, 1.0,
            std::f32::consts::TAU / 0.5, 0.82);
        assert!(frames.len() > 10 && frames.len() <= 61);
        assert_eq!((frames[0].width, frames[0].height), (255.0, 33.0));
        assert_eq!((frames.last().unwrap().width, frames.last().unwrap().height),
            (585.0, 230.0));
        assert!(frames.iter().all(|frame| frame.y == 0.0));
        assert_eq!(frames.first().unwrap().opacity, 1.0);
        assert_eq!(frames.last().unwrap().opacity, 0.0);
    }
}
#[cfg(test)]
mod desktop_codex_request_tests {
    use super::{codex_desktop, DeskApp, Workspace};
    use std::sync::mpsc;

    #[test]
    fn pending_decision_opens_the_island_and_clears_after_answer() {
        let mut app = DeskApp::new(None, 0, true, eframe::egui::Context::default());
        app.workspace = Workspace::Music;
        app.expanded = false;
        app.opened_by_hover = true;
        let (sender, receiver) = mpsc::channel();
        app.desktop_codex_rx = Some(receiver);
        sender.send(codex_desktop::Event::Approval(codex_desktop::Approval {
            title: "Codex requests a command".into(),
            detail: "Run a check".into(),
        })).unwrap();
        app.poll_desktop_codex();
        assert!(app.expanded);
        assert!(matches!(app.workspace, Workspace::Agents));
        assert!(app.agent_chat_open && app.agent_chat_desktop);
        assert!(!app.opened_by_hover);
        assert!(app.desktop_codex_approval.is_some());
        sender.send(codex_desktop::Event::Progress("Waiting for Codex…".into())).unwrap();
        app.poll_desktop_codex();
        assert!(app.desktop_codex_approval.is_none());
    }
}
fn glass_card_icon(ui: &egui::Ui, rect: egui::Rect, kind: usize) {
    let p = ui.painter();
    let c = rect.center();
    let line = Stroke::new(1.8, TEXT);
    match kind {
        0 => {
            p.circle_stroke(c, 8.0, line);
            p.add(egui::Shape::convex_polygon(vec![
                c + Vec2::new(-2.0, -4.2), c + Vec2::new(4.0, 0.0),
                c + Vec2::new(-2.0, 4.2)], TEXT, Stroke::NONE));
        }
        1 => {
            p.add(egui::Shape::line(vec![
                c + Vec2::new(-9.0, 0.0), c + Vec2::new(-5.0, 0.0),
                c + Vec2::new(-3.0, -5.0), c + Vec2::new(0.0, 6.0),
                c + Vec2::new(3.0, -3.0), c + Vec2::new(5.0, 0.0),
                c + Vec2::new(9.0, 0.0)], line));
        }
        2 => {
            p.circle_stroke(c + Vec2::new(0.0, 1.0), 8.0, line);
            p.line_segment([c + Vec2::new(0.0, -11.0), c + Vec2::new(0.0, -7.0)], line);
            p.line_segment([c + Vec2::new(0.0, 1.0), c + Vec2::new(2.5, -4.0)], line);
        }
        _ => {
            p.rect_stroke(rect.shrink(2.0), 3.0, line, egui::StrokeKind::Inside);
            p.add(egui::Shape::line(vec![c + Vec2::new(-5.0, 0.0),
                c + Vec2::new(-1.0, 4.0), c + Vec2::new(5.0, -4.0)], line));
        }
    }
}
fn glass_card_title(ui: &mut egui::Ui, text: &str, kind: usize) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(21.0), egui::Sense::hover());
        glass_card_icon(ui, rect, kind);
        label(ui, text, 17.0, TEXT);
    });
}
fn album_placeholder(ui: &egui::Ui, art: egui::Rect) {
    let p = ui.painter();
    p.rect_filled(art, 15.0, Color32::from_rgb(216, 190, 202));
    p.circle_filled(art.center() + Vec2::new(18.0, -15.0), 16.0,
        Color32::from_rgba_unmultiplied(255, 228, 211, 120));
    p.add(egui::Shape::convex_polygon(vec![
        egui::pos2(art.left() + 2.0, art.bottom() - 22.0),
        egui::pos2(art.right() - 2.0, art.top() + 33.0),
        egui::pos2(art.right() - 2.0, art.bottom() - 2.0),
        egui::pos2(art.left() + 2.0, art.bottom() - 2.0),
    ], Color32::from_rgb(137, 160, 203), Stroke::NONE));
    p.add(egui::Shape::convex_polygon(vec![
        egui::pos2(art.left() + 2.0, art.bottom() - 13.0),
        egui::pos2(art.right() - 2.0, art.top() + 48.0),
        egui::pos2(art.right() - 2.0, art.bottom() - 2.0),
        egui::pos2(art.left() + 2.0, art.bottom() - 2.0),
    ], Color32::from_rgb(91, 129, 188), Stroke::NONE));
    p.add(glass_sheen(art, 55));
    p.rect_stroke(art, 15.0, Stroke::new(0.7,
        Color32::from_rgba_unmultiplied(255, 255, 255, 75)), egui::StrokeKind::Inside);
}
fn album_art(ui: &mut egui::Ui, art: egui::Rect, texture: Option<&egui::TextureHandle>) {
    if let Some(texture) = texture {
        let size = texture.size_vec2();
        let uv = if size.x > size.y {
            let inset = (1.0 - size.y / size.x) * 0.5;
            egui::Rect::from_min_max(egui::pos2(inset, 0.0), egui::pos2(1.0 - inset, 1.0))
        } else {
            let inset = (1.0 - size.x / size.y) * 0.5;
            egui::Rect::from_min_max(egui::pos2(0.0, inset), egui::pos2(1.0, 1.0 - inset))
        };
        ui.put(art, egui::Image::from_texture(texture)
            .uv(uv).maintain_aspect_ratio(false)
            .fit_to_exact_size(art.size()).corner_radius(15));
    } else { album_placeholder(ui, art); }
}
fn card() -> Frame {
    Frame::new()
        .fill(CARD)
        .corner_radius(16)
        .stroke(Stroke::new(0.7, Color32::from_rgba_unmultiplied(233, 238, 255, 43)))
        .inner_margin(Margin::same(14))
}
fn settings_card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R)
    -> egui::InnerResponse<R> {
    card().show(ui, |ui| {
        // Settings cards should read as full-width grouped controls rather
        // than a stack of tiny content-sized boxes.
        ui.set_min_width(475.0);
        add_contents(ui)
    })
}
fn apply_visuals(ctx: &egui::Context, rounded: bool) {
    let mut visuals = egui::Visuals::dark();
    let radius = CornerRadius::same(if rounded { 11 } else { 3 });
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = radius;
    }
    ctx.set_visuals(visuals);
}
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, path, family) in [
        ("bookland_sf", "/System/Library/Fonts/SFNS.ttf", egui::FontFamily::Proportional),
        ("bookland_sf_mono", "/System/Library/Fonts/SFNSMono.ttf", egui::FontFamily::Monospace),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.into(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            fonts.families.entry(family).or_default().insert(0, name.into());
        }
    }
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFHebrewRounded.ttf") {
        fonts.font_data.insert("bookland_hebrew".into(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push("bookland_hebrew".into());
        }
    }
    ctx.set_fonts(fonts);
}
fn paint_workspace_icon(
    ui: &egui::Ui,
    rect: egui::Rect,
    workspace: Workspace,
    bookland_mark: Option<&egui::TextureHandle>,
    color: Color32,
    dots: bool,
) {
    let p = ui.painter();
    let c = egui::pos2(
        rect.left()
            + if rect.width() > 80.0 {
                21.0
            } else {
                rect.width() / 2.0
            },
        rect.center().y,
    );
    if dots {
        p.circle_filled(c, 3.2, color);
        return;
    }
    let s = Stroke::new(1.6, color);
    match workspace {
        Workspace::Nook => {
            if let Some(mark) = bookland_mark {
                p.image(mark.id(), egui::Rect::from_center_size(c, Vec2::splat(27.0)),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Color32::WHITE);
            } else {
                p.add(egui::Shape::line(vec![
                    c + Vec2::new(-8.0, 7.0), c + Vec2::new(-8.0, -6.0),
                    c, c + Vec2::new(8.0, -6.0), c + Vec2::new(8.0, 7.0),
                ], s));
                p.line_segment([c, c + Vec2::new(0.0, 7.0)], s);
            }
        }
        Workspace::Music => {
            p.add(egui::Shape::convex_polygon(vec![
                c + Vec2::new(-5.0, -8.0), c + Vec2::new(8.0, 0.0),
                c + Vec2::new(-5.0, 8.0),
            ], color, Stroke::NONE));
        }
        Workspace::Calendar => {
            let r = egui::Rect::from_center_size(c, Vec2::new(16.0, 15.0));
            p.rect_stroke(r, 2.0, s, egui::StrokeKind::Inside);
            p.line_segment(
                [
                    egui::pos2(r.left(), r.top() + 5.0),
                    egui::pos2(r.right(), r.top() + 5.0),
                ],
                s,
            );
            p.circle_filled(egui::pos2(c.x - 3.0, c.y + 3.0), 1.1, color);
            p.circle_filled(egui::pos2(c.x + 3.0, c.y + 3.0), 1.1, color);
        }
        Workspace::Tray => {
            p.add(egui::Shape::line(vec![
                c + Vec2::new(-8.0, -2.0), c + Vec2::new(-5.0, -7.0),
                c + Vec2::new(5.0, -7.0), c + Vec2::new(8.0, -2.0),
                c + Vec2::new(8.0, 6.0), c + Vec2::new(-8.0, 6.0),
                c + Vec2::new(-8.0, -2.0),
            ], s));
            p.line_segment([c + Vec2::new(-8.0, -1.0), c + Vec2::new(-4.0, -1.0)], s);
            p.line_segment([c + Vec2::new(4.0, -1.0), c + Vec2::new(8.0, -1.0)], s);
        }
        Workspace::Clipboard => {
            let page = egui::Rect::from_center_size(c, Vec2::new(15.0, 18.0));
            p.rect_stroke(page, 2.0, s, egui::StrokeKind::Inside);
            for y in [-4.0, 0.0, 4.0] {
                p.line_segment([egui::pos2(c.x - 4.0, c.y + y),
                    egui::pos2(c.x + 4.0, c.y + y)], s);
            }
        }
        Workspace::Agents => {
            for (offset, radius) in [(Vec2::ZERO, 8.0), (Vec2::new(7.0, -7.0), 2.6)] {
                let center = c + offset;
                p.add(egui::Shape::convex_polygon(vec![
                    center + Vec2::new(0.0, -radius),
                    center + Vec2::new(radius * 0.24, -radius * 0.24),
                    center + Vec2::new(radius, 0.0),
                    center + Vec2::new(radius * 0.24, radius * 0.24),
                    center + Vec2::new(0.0, radius),
                    center + Vec2::new(-radius * 0.24, radius * 0.24),
                    center + Vec2::new(-radius, 0.0),
                    center + Vec2::new(-radius * 0.24, -radius * 0.24),
                ], color, Stroke::NONE));
            }
        }
        Workspace::Prompts => {
            let page = egui::Rect::from_center_size(c, Vec2::new(15.0, 18.0));
            p.rect_stroke(page, 2.0, s, egui::StrokeKind::Inside);
            for y in [-4.0, 0.0, 4.0] {
                p.line_segment([egui::pos2(c.x - 4.0, c.y + y),
                    egui::pos2(c.x + 4.0, c.y + y)], s);
            }
        }
        Workspace::Prompter => {
            p.text(c, Align2::CENTER_CENTER, "▤", FontId::proportional(18.0), color);
        }
    }
}

fn paint_settings_icon(p: &egui::Painter, c: egui::Pos2, name: &str, color: Color32) {
    let s = Stroke::new(1.35, color);
    let segment = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([c + Vec2::new(a.0, a.1), c + Vec2::new(b.0, b.1)], s);
    };
    let box_at = |w: f32, h: f32, radius: f32| {
        p.rect_stroke(egui::Rect::from_center_size(c, Vec2::new(w, h)),
            radius, s, egui::StrokeKind::Inside);
    };
    match name {
        "General" => {
            p.circle_stroke(c, 3.0, s);
            for n in 0..8 {
                let angle = n as f32 * std::f32::consts::FRAC_PI_4;
                let v = Vec2::angled(angle);
                p.line_segment([c + v * 4.4, c + v * 6.1], s);
            }
        }
        "Appearance" => {
            p.circle_stroke(c, 5.6, s);
            let mut half = vec![c];
            for n in 0..=8 {
                let angle = std::f32::consts::FRAC_PI_2
                    + n as f32 * std::f32::consts::PI / 8.0;
                half.push(c + Vec2::angled(angle) * 5.0);
            }
            p.add(egui::Shape::convex_polygon(half, color, Stroke::NONE));
        }
        "Displays" => {
            box_at(12.0, 8.5, 1.3);
            segment((0.0, 4.5), (0.0, 6.0));
            segment((-3.0, 6.0), (3.0, 6.0));
        }
        "Behavior" => {
            for (y, x) in [(-4.0, -2.5), (0.0, 3.0), (4.0, -1.0)] {
                segment((-6.0, y), (6.0, y));
                p.circle_filled(c + Vec2::new(x, y), 1.8, color);
            }
        }
        "Pocket" | "Drop Area" => {
            p.add(egui::Shape::line(vec![c + Vec2::new(-6.0, -2.5),
                c + Vec2::new(-2.0, -2.5), c + Vec2::new(-0.5, -4.5),
                c + Vec2::new(6.0, -4.5), c + Vec2::new(6.0, 5.0),
                c + Vec2::new(-6.0, 5.0), c + Vec2::new(-6.0, -2.5)], s));
            if name == "Drop Area" {
                segment((0.0, -1.5), (0.0, 2.7));
                segment((-2.0, 0.8), (0.0, 2.7));
                segment((2.0, 0.8), (0.0, 2.7));
            }
        }
        "Permissions" => {
            p.add(egui::Shape::line(vec![
                c + Vec2::new(0.0, -6.0), c + Vec2::new(5.0, -3.8),
                c + Vec2::new(4.3, 2.0), c + Vec2::new(0.0, 6.0),
                c + Vec2::new(-4.3, 2.0), c + Vec2::new(-5.0, -3.8),
                c + Vec2::new(0.0, -6.0)], s));
        }
        "Nook" => {
            p.add(egui::Shape::line(vec![c + Vec2::new(-6.0, -0.5),
                c + Vec2::new(0.0, -5.5), c + Vec2::new(6.0, -0.5)], s));
            segment((-4.5, -1.0), (-4.5, 5.5));
            segment((-4.5, 5.5), (4.5, 5.5));
            segment((4.5, 5.5), (4.5, -1.0));
            segment((-1.5, 5.0), (-1.5, 1.0));
            segment((-1.5, 1.0), (1.5, 1.0));
        }
        "Music" => {
            segment((1.0, -5.5), (5.0, -6.0));
            segment((1.0, -5.5), (1.0, 3.0));
            segment((5.0, -6.0), (5.0, 1.5));
            p.circle_filled(c + Vec2::new(-1.0, 3.5), 2.2, color);
            p.circle_filled(c + Vec2::new(3.0, 2.0), 2.2, color);
        }
        "Tray" => {
            segment((-6.0, 1.5), (-4.0, 5.0));
            segment((-4.0, 5.0), (4.0, 5.0));
            segment((4.0, 5.0), (6.0, 1.5));
            segment((0.0, -6.0), (0.0, 1.5));
            segment((-2.2, -0.5), (0.0, 1.7));
            segment((2.2, -0.5), (0.0, 1.7));
        }
        "Clipboard" => {
            box_at(10.0, 11.0, 1.5);
            p.rect_filled(egui::Rect::from_center_size(c + Vec2::new(0.0, -5.2),
                Vec2::new(4.0, 2.5)), 0.8, color);
        }
        "Prompter" => {
            box_at(11.5, 10.5, 1.5);
            for y in [-2.7, 0.0, 2.7] { segment((-3.4, y), (3.4, y)); }
        }
        "Calendar" => {
            box_at(11.5, 10.5, 1.4);
            segment((-5.2, -2.0), (5.2, -2.0));
            for x in [-2.5, 1.5] {
                for y in [0.5, 3.0] { p.circle_filled(c + Vec2::new(x, y), 0.7, color); }
            }
        }
        "Agents" => {
            p.add(egui::Shape::convex_polygon(vec![c + Vec2::new(0.0, -6.5),
                c + Vec2::new(1.5, -1.5), c + Vec2::new(6.5, 0.0),
                c + Vec2::new(1.5, 1.5), c + Vec2::new(0.0, 6.5),
                c + Vec2::new(-1.5, 1.5), c + Vec2::new(-6.5, 0.0),
                c + Vec2::new(-1.5, -1.5)], color, Stroke::NONE));
        }
        "Live Activities" => {
            p.circle_stroke(c, 5.7, s);
            p.circle_filled(c, 2.1, color);
        }
        "Lock Screen" => {
            p.add(egui::Shape::line(vec![c + Vec2::new(-3.5, -1.0),
                c + Vec2::new(-3.5, -3.2), c + Vec2::new(-2.0, -5.2),
                c + Vec2::new(2.0, -5.2), c + Vec2::new(3.5, -3.2),
                c + Vec2::new(3.5, -1.0)], s));
            p.rect_stroke(egui::Rect::from_center_size(c + Vec2::new(0.0, 2.5),
                Vec2::new(10.0, 7.0)), 1.2, s, egui::StrokeKind::Inside);
        }
        "Downloads" => {
            segment((0.0, -6.0), (0.0, 2.5));
            segment((-3.0, -0.5), (0.0, 2.5));
            segment((3.0, -0.5), (0.0, 2.5));
            segment((-5.5, 5.0), (5.5, 5.0));
        }
        "Messages (beta)" => {
            box_at(11.5, 8.5, 2.0);
            segment((-2.5, 4.0), (-4.5, 6.0));
            segment((-4.5, 6.0), (-4.5, 3.0));
        }
        "Terminal" => {
            segment((-5.0, -4.0), (-1.0, 0.0));
            segment((-1.0, 0.0), (-5.0, 4.0));
            segment((1.0, 4.0), (5.5, 4.0));
        }
        "Keyboard Shortcuts" => {
            for (x, y) in [(-3.5, -3.5), (3.5, -3.5), (-3.5, 3.5), (3.5, 3.5)] {
                p.circle_stroke(c + Vec2::new(x, y), 2.3, s);
            }
        }
        "About" => {
            p.circle_stroke(c, 5.7, s);
            p.circle_filled(c + Vec2::new(0.0, -2.9), 0.9, color);
            segment((0.0, -0.5), (0.0, 3.5));
        }
        _ => { p.circle_filled(c, 2.0, color); }
    }
}
fn section_title(ui: &mut egui::Ui, text: &str) {
    label(ui, text, 11.0, MUTED);
    ui.add_space(7.0);
}

impl DeskApp {
    fn open_from_hover(&mut self) {
        if self.pocket_job.is_some() || self.pocket_activity.is_some() {
            self.choose_hover_workspace();
        } else if let Some(done) = self.agent_completion.take() {
            self.workspace = Workspace::Agents;
            self.agent_chat_open = false;
            self.agent_usage_view = false;
            self.selected_agent = self.agents.iter()
                .position(|agent| agent.path == done.path && agent.source == done.source)
                .unwrap_or(0);
        } else {
            self.choose_hover_workspace();
        }
        self.expanded = true;
        self.opened_by_hover = true;
        self.last_hovered = Instant::now();
        self.hover_opened_at = self.last_hovered;
    }

    fn choose_hover_workspace(&mut self) {
        if self.pocket_job.is_some() || self.pocket_activity.take().is_some() {
            self.workspace = Workspace::Tray;
            return;
        }
        if self.workspace_chosen_by_user { return; }
        if let Some(index) = self.prefs.agent_monitor.then(|| self.agents.iter()
            .position(|agent| agent.source == "Codex" && agent.status == "Working"))
            .flatten() {
            self.workspace = Workspace::Agents;
            self.selected_agent = index;
            self.agent_usage_view = false;
            self.agent_chat_open = false;
        } else if self.video_rx.is_some() || self.downloads.iter().any(|item| item.active) {
            self.workspace = Workspace::Tray;
        } else if let Some(index) = self.prefs.agent_monitor.then(|| self.agents.iter()
            .position(|agent| agent.source == "Codex")).flatten() {
            self.workspace = Workspace::Agents;
            self.selected_agent = index;
            self.agent_usage_view = false;
            self.agent_chat_open = false;
        } else {
            self.workspace = Workspace::Nook;
        }
    }
    fn closed_dimensions(&self) -> (f32, f32) {
        let mut width = self.prefs.closed_width.clamp(180.0, 400.0);
        let mut height = self.prefs.closed_height.clamp(30.0, 70.0);
        #[cfg(target_os = "macos")]
        if !self.preview_mode && self.screenshot.is_none() {
            let mut notch_width = 0.0;
            let mut notch_height = 0.0;
            if unsafe { nooklet_physical_notch(&mut notch_width, &mut notch_height) } != 0 {
                // Leave room for the Codex mark and activity ring beside the camera cutout.
                width = width.max((notch_width as f32 + 76.0).clamp(180.0, 500.0));
                height = height.max(notch_height as f32);
            }
        }
        (width, height)
    }
    fn open_dimensions(&self) -> (f32, f32) {
        if self.prefs.style == "Glass" {
            if self.workspace == Workspace::Agents {
                return if self.agent_chat_open {
                    (585.0, if self.agent_chat_desktop
                        && self.desktop_codex_question.is_some() { 370.0 }
                        else if self.agent_chat_desktop { 285.0 } else { 300.0 })
                } else { (585.0, 230.0) };
            }
            if self.workspace == Workspace::Music {
                let apple_music = self.media_app.as_deref() == Some("Music");
                let outputs = self.airplay_devices.iter()
                    .filter(|device| device.supports_audio).count().min(4);
                return (585.0, if apple_music && self.airplay_open { 330.0 + outputs as f32 * 28.0 }
                    else if apple_music && self.queue_open { 430.0 }
                    else if self.lyrics_open { 385.0 } else { 315.0 });
            }
            if self.workspace == Workspace::Tray {
                // Give recent files room above the workspace switcher while
                // capping the panel; additional files scroll within it.
                return (585.0, 450.0 + 80.0 * self.files.len().min(3) as f32);
            }
            if self.workspace != Workspace::Nook {
                return (585.0, if self.workspace == Workspace::Calendar { 400.0 }
                    else { self.prefs.open_height.clamp(380.0, 500.0) });
            }
            let height = if self.workspace == Workspace::Nook && self.prefs.widgets == Widget::DEFAULT {
                self.prefs.open_height.clamp(170.0, 550.0)
            } else {
                self.prefs.open_height.clamp(300.0, 550.0)
            };
            (self.prefs.open_width.clamp(900.0, 1200.0), height)
        } else {
            (self.prefs.open_width.clamp(420.0, 700.0), self.prefs.open_height.clamp(530.0, 780.0))
        }
    }
    fn new(screenshot: Option<PathBuf>, native_view: usize, preview_mode: bool,
        repaint_context: egui::Context) -> Self {
        let data = data_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .or_else(|| previous_data_path().and_then(|path| std::fs::read_to_string(path).ok()))
            .and_then(|text| serde_json::from_str::<AppData>(&text).ok())
            .unwrap_or_default();
        let mut prefs = preferences_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .or_else(|| previous_preferences_path().and_then(|path| std::fs::read_to_string(path).ok()))
            .or_else(|| std::fs::read_to_string("data/preferences.json").ok())
            .and_then(|s| serde_json::from_str::<Preferences>(&s).ok())
            .unwrap_or_default();
        if prefs.menu_style == "Compact labeled" {
            prefs.menu_style = "Compact".into();
        }
        if prefs.menu_style == "In-notch" {
            prefs.menu_style = "In the notch".into();
        }
        if prefs.media_source == "Auto" {
            prefs.media_source = "System".into();
        }
        if prefs.volume_hud_style != "Inline by the notch"
            && prefs.volume_hud_style != "Below the notch" {
            prefs.volume_hud_style = "Below the notch".into();
        }
        // Migrate the first prototype's oversized default notch and sample to-dos.
        if ((prefs.open_height - 300.0).abs() < f32::EPSILON
            || (prefs.open_height - 190.0).abs() < f32::EPSILON)
            && prefs.style == "Glass" && prefs.widgets == Widget::DEFAULT {
            prefs.open_height = 170.0;
        }
        if ((prefs.open_width - 1040.0).abs() < f32::EPSILON
            || (prefs.open_width - 1200.0).abs() < f32::EPSILON)
            && prefs.style == "Glass" && prefs.widgets == Widget::DEFAULT {
            prefs.open_width = 1025.0;
        }
        if (prefs.closed_width - 240.0).abs() < f32::EPSILON
            || (prefs.closed_width - 270.0).abs() < f32::EPSILON {
            prefs.closed_width = 255.0;
        }
        if (prefs.closed_height - 38.0).abs() < f32::EPSILON {
            prefs.closed_height = 33.0;
        }
        if prefs.todos.len() == 4
            && prefs.todos.iter().map(|item| item.text.as_str()).collect::<Vec<_>>()
                == ["Focus work", "Draft update", "Plan next sprint", "Review notes"] {
            prefs.todos.clear();
        }
        if !preview_mode {
            prefs.launch_at_login = login_agent_path().is_some_and(|path| path.exists());
        }
        if !preview_mode && screenshot.is_none() {
            codex_desktop::recover_pending_async();
        }
        let clipboard_images = if prefs.persist_clipboard && !preview_mode
            && screenshot.is_none() {
            restore_clipboard_images(&repaint_context, &data.clipboard_images)
        } else { Vec::new() };
        if !prefs.persist_clipboard && !preview_mode && screenshot.is_none() {
            if let Some(directory) = clipboard_images_dir() {
                clipboard_images::prune(&directory, &[]);
            }
        }
        let expanded = screenshot.is_some() || preview_mode || prefs.start_expanded;
        let now = Instant::now();
        let (media_action_tx, media_action_rx) = std::sync::mpsc::channel();
        Self {
            expanded,
            workspace: Workspace::Nook,
            workspace_chosen_by_user: false,
            settings_open: false,
            settings_section: "General".into(),
            tour_step: None,
            selected_widget: Widget::MediaPlayer,
            dragged_widget: None,
            prefs,
            screenshot,
            frames: 0,
            shot_requested: false,
            positioned: false,
            last_bottom_position: false,
            clipboard_bottom_sheet: false,
            clipboard_shelf_width: 1040.0,
            applied_size: Vec2::ZERO,
            transition: if expanded { 1.0 } else { 0.0 },
            transition_velocity: 0.0,
            animation_target_expanded: expanded,
            native_morph_pending: false,
            native_morph_animating: false,
            last_frame: now,
            last_hovered: now,
            hover_opened_at: now,
            opened_by_hover: false,
            hover_blocked_until_exit: false,
            native_ring_center: None,
            native_usage_ring_center: None,
            pending_activity_bubble: None,
            activity_bubble_visible: false,
            pending_result_card: None,
            result_card_visible: false,
            native_morph_shell: None,
            island_gesture: IslandGesture::default(),
            last_haptic_expanded: expanded,
            battery: battery_status(),
            volume: system_volume(),
            cpu: "—".into(),
            ram: "—".into(),
            gpu: "—".into(),
            disk: disk_status(),
            network: "—".into(),
            last_network_bytes: None,
            last_network_sample_at: None,
            last_system_poll: now - Duration::from_secs(25),
            system_rx: None,
            memory_total: run_output("sysctl", &["-n", "hw.memsize"])
                .and_then(|value| value.parse().ok()).unwrap_or(0),
            cpu_history: VecDeque::new(),
            ram_history: VecDeque::new(),
            gpu_history: VecDeque::new(),
            battery_history: VecDeque::new(),
            disk_history: VecDeque::new(),
            clipboard: data.clipboard,
            clipboard_images,
            clipboard_files: data.clipboard_files,
            clipboard_favorites: data.clipboard_favorites,
            clipboard_search: String::new(),
            clipboard_focus_pending: false,
            clipboard_filter: "All Items".into(),
            clipboard_app_draft: String::new(),
            last_clip: String::new(),
            last_clip_change: -1,
            last_image_fingerprint: None,
            last_clip_poll: now,
            files: data.files,
            pipelines: data.pipelines.into_iter()
                .filter(|pipeline| pipeline.validate().is_ok()).take(20).collect(),
            pipeline_draft_name: String::new(),
            pipeline_draft_steps: Vec::new(),
            pipeline_draft_action: pocket::Action::Webp,
            pipeline_editor_open: false,
            pocket_job: None,
            pocket_status: String::new(),
            pocket_activity: None,
            drag_shake: shake::DragShake::default(),
            pocket_floating: false,
            pocket_anchor: egui::pos2(320.0, 80.0),
            pocket_opened_from_drag: false,
            pocket_monitor_enabled: false,
            hover_monitor_enabled: false,
            animation_clock_enabled: false,
            volume_hud_monitor_enabled: false,
            volume_hud_inline_enabled: false,
            volume_hud_closed_width: 0.0,
            spectrum_capture_active: false,
            spectrum_state: 0,
            spectrum_levels: [0.0; 16],
            airpods_events_attempted: false,
            airpods_events_enabled: false,
            pocket_wake_context: Box::new(repaint_context),
            notes: data.notes,
            note_export_rx: None,
            prompter: data.prompter,
            codex_session_id: data.codex_session_id,
            codex_messages: data.codex_messages,
            codex_draft: String::new(),
            codex_rx: None,
            codex_pid: None,
            codex_status: String::new(),
            codex_stopping: false,
            desktop_codex_thread_id: None,
            desktop_codex_title: String::new(),
            desktop_codex_messages: Vec::new(),
            desktop_codex_draft: String::new(),
            desktop_codex_rx: None,
            desktop_codex_decision_tx: None,
            desktop_codex_approval: None,
            desktop_codex_question: None,
            desktop_codex_question_answers: Vec::new(),
            desktop_codex_allow_edits: false,
            desktop_codex_pid: None,
            desktop_codex_status: String::new(),
            desktop_codex_stopping: false,
            sheet_goal: data.sheet_goal,
            sheet_source: data.sheet_source,
            sheet_result: data.sheet_result,
            sheet_constraints: data.sheet_constraints,
            sheet_prompt: data.sheet_prompt,
            prompter_scroll: 0.0,
            prompter_playing: false,
            prompter_voice_started: false,
            prompter_voice_word: 0,
            prompter_last_transcript: String::new(),
            last_prompter_frame: now,
            downloads: Vec::new(),
            downloads_error: None,
            last_download_poll: now - Duration::from_secs(20),
            last_download_sample: now,
            downloads_rx: None,
            video_offer: None,
            video_url_draft: String::new(),
            video_progress: VideoProgress::default(),
            video_rx: None,
            video_pid: None,
            video_saved_path: None,
            video_status: String::new(),
            video_stopping: false,
            terminal_draft: String::new(),
            terminal_output: String::new(),
            terminal_rx: None,
            weather: Weather::default(),
            weather_rx: None,
            last_weather_poll: now - Duration::from_secs(1000),
            airpods: AirPodsStatus::default(),
            airpods_rx: None,
            last_airpods_poll: now - Duration::from_secs(600),
            airpods_refresh_pending: false,
            airpods_baseline_seen: false,
            airpods_activity: None,
            calendar_offset: 0,
            timer_secs: 10 * 60,
            timer_end: None,
            timer_done: false,
            stopwatch_started: None,
            stopwatch_saved: Duration::ZERO,
            muted_volume: None,
            todo_draft: String::new(),
            todo_editing: false,
            todo_focus_pending: false,
            media_title: "Nothing playing".into(),
            media_artist: "Press Play for music".into(),
            media_app: None,
            media_position: 0.0,
            media_duration: 0.0,
            media_playing: false,
            media_rx: None,
            media_action_tx,
            media_action_rx,
            media_actions_pending: 0,
            last_media_request: now - Duration::from_secs(20),
            airplay_open: false,
            airplay_devices: Vec::new(),
            airplay_rx: None,
            airplay_status: String::new(),
            queue_open: false,
            queue_for: None,
            queue_rx: None,
            queue_preview: music_queue::Preview { tracks: Vec::new(), status: String::new() },
            mirror_open: false,
            mirror_error: String::new(),
            mirror_texture: None,
            mirror_sequence: 0,
            mirror_buffer: Vec::new(),
            artwork_for: None,
            artwork_rx: None,
            artwork_texture: None,
            codex_logo: None,
            bookland_mark: None,
            notch_shoulders: None,
            lyrics_for: None,
            lyrics_rx: None,
            lyrics_lines: Vec::new(),
            lyrics_status: String::new(),
            lyrics_open: false,
            last_media_poll: now - Duration::from_secs(20),
            toast: None,
            native_view,
            preview_mode,
            shortcut_registered: false,
            clipboard_shortcut_registered: false,
            native_installed: false,
            glass_rects: [None; 8],
            open_notch_rect: None,
            agents: Vec::new(),
            agents_initialized: false,
            agent_completion: None,
            selected_agent: 0,
            agent_usage_view: false,
            agent_chat_open: false,
            agent_chat_desktop: false,
            last_agent_poll: now - Duration::from_secs(20),
            agent_rx: None,
            craft_agents: Vec::new(),
            craft_rx: None,
            last_craft_poll: now - Duration::from_secs(30),
            // Read the persisted macOS grant before the first Calendar frame so
            // a connected calendar never flashes the Connect button on launch.
            calendar_status: {
                #[cfg(target_os = "macos")]
                { unsafe { nooklet_calendar_status() } }
                #[cfg(not(target_os = "macos"))]
                { 0 }
            },
            calendar_choices: Vec::new(),
            calendar_events: Vec::new(),
            calendar_snapshot_version: 0,
            calendar_query_pending: false,
            calendar_month_key: None,
            calendar_month: CalendarMonth::default(),
            calendar_selected_day: None,
            last_calendar_poll: now - Duration::from_secs(35),
            new_event_title: String::new(),
            new_event_date: run_output("date", &["+%Y-%m-%d"]).unwrap_or_default(),
            new_event_time: run_output("date", &["+%H:00"]).unwrap_or_default(),
            new_event_minutes: 60,
        }
    }
    fn apply_native_glass(&mut self) {
        #[cfg(target_os = "macos")]
        unsafe {
            if !self.native_installed && self.native_view != 0 {
                self.native_installed = nooklet_install_glass(self.native_view as *mut _) != 0;
            }
            if self.native_installed {
                if (self.expanded || self.prefs.style != "Glass"
                    || self.screenshot.is_some() || self.preview_mode)
                    && self.activity_bubble_visible {
                    bookland_activity_bubble_hide();
                    self.activity_bubble_visible = false;
                }
                if (self.prefs.style != "Glass"
                    || self.screenshot.is_some() || self.preview_mode)
                    && self.result_card_visible {
                    bookland_conversion_card_hide();
                    self.result_card_visible = false;
                }
                let result_panel_bottom = if self.expanded {
                    self.glass_rects[0].map(|rect| f64::from(rect.max.y)).unwrap_or(0.0)
                } else { 0.0 };
                if self.result_card_visible {
                    bookland_result_card_position(result_panel_bottom);
                }
                if let Some(message) = self.pending_activity_bubble.take() {
                    if !self.expanded && self.prefs.style == "Glass"
                        && self.screenshot.is_none() && !self.preview_mode {
                        if let Ok(message) = CString::new(message.chars().take(80)
                            .collect::<String>()) {
                            bookland_activity_bubble_show(message.as_ptr(),
                                f64::from(self.closed_dimensions().0));
                            self.activity_bubble_visible = true;
                        }
                    }
                }
                if self.pending_result_card.as_ref().is_some_and(|card|
                    card.ready_at.elapsed() >= card.lifetime) {
                    self.pending_result_card = None;
                }
                if self.prefs.style == "Glass"
                    && self.screenshot.is_none() && !self.preview_mode {
                    if let Some(card) = self.pending_result_card.take() {
                        if let (Ok(path), Ok(subtitle)) = (
                            CString::new(card.path.to_string_lossy().as_bytes()),
                            CString::new(card.subtitle)) {
                            bookland_result_card_show(path.as_ptr(), subtitle.as_ptr(),
                                card.lifetime.as_secs_f64(), result_panel_bottom);
                            self.result_card_visible = true;
                        }
                    }
                }
                if !self.airpods_events_attempted && !self.preview_mode
                    && self.screenshot.is_none() {
                    self.airpods_events_enabled = bookland_airpods_watch_set(1) != 0;
                    self.airpods_events_attempted = true;
                }
                let target = if self.expanded { 1.0 } else { 0.0 };
                let clock_wanted = self.transition != target && !self.preview_mode
                    && self.screenshot.is_none();
                if clock_wanted != self.animation_clock_enabled {
                    let context = self.pocket_wake_context.as_ref() as *const egui::Context
                        as *mut std::ffi::c_void;
                    self.animation_clock_enabled = nooklet_animation_clock_set(
                        clock_wanted as i32, context,
                        clock_wanted.then_some(bookland_pocket_wake)) != 0 && clock_wanted;
                }
                if !self.preview_mode && self.screenshot.is_none() {
                    if let Some(center) = self.native_ring_center.filter(|_|
                        self.transition == 0.0) {
                        nooklet_progress_ring_set(1, center.x as f64, center.y as f64, 0);
                    } else if let Some(center) = self.native_usage_ring_center.filter(|_|
                        self.transition == 1.0) {
                        nooklet_progress_ring_set(1, center.x as f64, center.y as f64, 1);
                    } else {
                        nooklet_progress_ring_set(0, 0.0, 0.0, 0);
                    }
                } else {
                    nooklet_progress_ring_set(0, 0.0, 0.0, 0);
                }
                nooklet_anchor_host_to_screen_top((!self.preview_mode && self.screenshot.is_none()
                    && !(self.expanded && self.clipboard_bottom_sheet
                        && self.workspace == Workspace::Clipboard)) as i32,
                    self.prefs.island_display_id);
                let hover_wanted = self.screenshot.is_none() && !self.preview_mode
                    && (self.prefs.open_on_hover
                        || (self.expanded && self.opened_by_hover && self.prefs.auto_close));
                if hover_wanted != self.hover_monitor_enabled {
                    let context = self.pocket_wake_context.as_ref() as *const egui::Context
                        as *mut std::ffi::c_void;
                    nooklet_hover_monitor_set(hover_wanted as i32, context,
                        hover_wanted.then_some(bookland_pocket_wake));
                    self.hover_monitor_enabled = hover_wanted;
                }
                let volume_hud_wanted = self.prefs.volume_hud
                    && self.screenshot.is_none() && !self.preview_mode;
                let volume_hud_inline = self.prefs.volume_hud_style == "Inline by the notch";
                let closed_width = self.closed_dimensions().0;
                if volume_hud_wanted != self.volume_hud_monitor_enabled
                    || volume_hud_inline != self.volume_hud_inline_enabled
                    || (closed_width - self.volume_hud_closed_width).abs() > 0.5 {
                    bookland_volume_hud_set(volume_hud_wanted as i32,
                        volume_hud_inline as i32, closed_width as f64);
                    self.volume_hud_monitor_enabled = volume_hud_wanted;
                    self.volume_hud_inline_enabled = volume_hud_inline;
                    self.volume_hud_closed_width = closed_width;
                }
                for (slot, area) in self.glass_rects.iter().enumerate() {
                    let morphing = slot == 0 && self.transition >= 0.82;
                    let enabled = slot != 1 && self.screenshot.is_none()
                        && self.prefs.style == "Glass"
                        && (self.transition >= 0.96 || morphing);
                    if let Some(rect) = area.filter(|_| enabled) {
                        nooklet_set_glass(
                            slot as i32, rect.min.x as f64, rect.min.y as f64,
                            rect.width() as f64, rect.height() as f64,
                            if slot == 0 { f64::from(26.0_f32.min(rect.height() * 0.5)) }
                                else { 23.0 },
                            self.prefs.glass_level as f64,
                            if morphing { f64::from(((self.transition - 0.82) / 0.14)
                                .clamp(0.0, 1.0)) } else { 1.0 }, 1,
                        );
                    } else {
                        nooklet_set_glass(slot as i32, 0.0, 0.0, 0.0, 0.0,
                            0.0, 0.0, 0.0, 0);
                    }
                }
                if let Some((rect, top, bottom, opacity)) = self.native_morph_shell {
                    if self.native_morph_pending && self.prefs.style == "Glass"
                        && self.screenshot.is_none() && !self.preview_mode
                        && !self.clipboard_bottom_sheet {
                        let (open_width, open_height) = self.open_dimensions();
                        let frames = native_morph_frames(rect.center().x, rect.min.y,
                            self.closed_dimensions(), (open_width, open_height),
                            self.transition, self.transition_velocity, target,
                            std::f32::consts::TAU
                                * self.prefs.animation_speed.clamp(0.5, 2.0)
                                / if nooklet_reduce_motion() != 0 { 0.25 } else { 0.5 },
                            if nooklet_reduce_motion() != 0 { 1.0 } else { 0.82 });
                        let duration = (frames.len() - 1) as f64 / 60.0;
                        if nooklet_morph_shell_animate(frames.as_ptr(), frames.len(), duration,
                            f64::from(open_width + 30.0),
                            f64::from(open_height + 60.0)) != 0 {
                            self.native_morph_pending = false;
                            self.native_morph_animating = true;
                        }
                    }
                    nooklet_morph_shell_set(1, rect.min.x as f64, rect.min.y as f64,
                        rect.width() as f64, rect.height() as f64,
                        top as f64, bottom as f64, opacity as f64);
                } else {
                    self.native_morph_pending = false;
                    self.native_morph_animating = false;
                    nooklet_morph_shell_set(0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
                }
                if self.animation_clock_enabled {
                    // Core Animation interpolates the shell independently, but the
                    // glass and controls are still painted by egui. Running those
                    // at 30 Hz makes them visibly trail the 60 Hz shell, and
                    // switching rates near the reveal adds a pacing discontinuity.
                    // The display link is paused again as soon as the spring rests.
                    nooklet_animation_clock_rate(60);
                }
            }
        }
    }
    fn save_prefs(&self) {
        if self.preview_mode || self.screenshot.is_some() { return; }
        if let Ok(value) = serde_json::to_string_pretty(&self.prefs) {
            let saved = preferences_path()
                .and_then(|path| {
                    std::fs::create_dir_all(path.parent()?).ok()?;
                    std::fs::write(path, &value).ok()
                })
                .is_some();
            if !saved {
                let _ = std::fs::create_dir_all("data");
                let _ = std::fs::write("data/preferences.json", value);
            }
        }
    }
    fn save_data(&self) {
        if self.preview_mode || self.screenshot.is_some() { return; }
        let saved_images = if self.prefs.persist_clipboard {
            if let Some(directory) = clipboard_images_dir() {
                self.clipboard_images.iter().take(clipboard_images::MAX_IMAGES)
                    .filter_map(|image| {
                        let record = image.saved_record();
                        clipboard_images::write_image(&directory, &record, &image.rgba)
                            .ok().map(|_| record)
                    }).collect::<Vec<_>>()
            } else { Vec::new() }
        } else { Vec::new() };
        let data = AppData {
            files: self.files.clone(),
            pipelines: self.pipelines.clone(),
            clipboard: if self.prefs.persist_clipboard { self.clipboard.clone() } else { Vec::new() },
            clipboard_files: if self.prefs.persist_clipboard { self.clipboard_files.clone() } else { Vec::new() },
            clipboard_images: saved_images.clone(),
            clipboard_favorites: if self.prefs.persist_clipboard { self.clipboard_favorites.clone() } else { Vec::new() },
            notes: self.notes.clone(),
            prompter: self.prompter.clone(),
            codex_session_id: self.codex_session_id.clone(),
            codex_messages: self.codex_messages.clone(),
            sheet_goal: self.sheet_goal.clone(),
            sheet_source: self.sheet_source.clone(),
            sheet_result: self.sheet_result.clone(),
            sheet_constraints: self.sheet_constraints.clone(),
            sheet_prompt: self.sheet_prompt.clone(),
        };
        if let (Some(path), Ok(json)) = (data_path(), serde_json::to_vec_pretty(&data)) {
            if let Some(parent) = path.parent() {
                if std::fs::create_dir_all(parent).is_ok() {
                    let temp = path.with_extension("json.tmp");
                    if std::fs::write(&temp, json).is_ok()
                        && std::fs::rename(temp, path).is_ok() {
                        if let Some(directory) = clipboard_images_dir() {
                            clipboard_images::prune(&directory, &saved_images);
                        }
                    }
                }
            }
        }
    }
    fn notify(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }
    fn install_pipeline(&mut self, path: &Path) {
        match pipelines::import(path) {
            Ok(pipeline) => {
                if let Some(existing) = self.pipelines.iter_mut()
                    .find(|item| item.name.eq_ignore_ascii_case(&pipeline.name)) {
                    *existing = pipeline.clone();
                } else if self.pipelines.len() < 20 {
                    self.pipelines.push(pipeline.clone());
                } else {
                    self.notify("Remove a pipeline before importing another");
                    return;
                }
                self.save_data();
                self.pocket_status = format!("Installed pipeline: {}", pipeline.name);
                self.notify(self.pocket_status.clone());
            }
            Err(error) => self.notify(format!("Could not import pipeline: {error}")),
        }
    }
    fn remember_pocket_file(&mut self, path: PathBuf, open_tray: bool) {
        if !path.exists() {
            self.notify("That file is no longer available");
            return;
        }
        if path.extension().and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("booklandpipeline")) {
            self.install_pipeline(&path);
            if open_tray { self.expanded = true; self.workspace = Workspace::Tray; }
            return;
        }
        self.files.retain(|item| item != &path);
        self.files.insert(0, path);
        self.files.truncate(8);
        self.pocket_status.clear();
        self.save_data();
        if open_tray {
            self.expanded = true;
            self.workspace = Workspace::Tray;
        }
    }
    fn add_pocket_file(&mut self, path: PathBuf) {
        self.remember_pocket_file(path, true);
    }
    fn open_pocket_near_pointer(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        {
            let mut x = 0.0;
            let mut y = 0.0;
            if unsafe { bookland_pocket_anchor_at_cursor(&mut x, &mut y) } {
                self.pocket_anchor = egui::pos2(x as f32, y as f32);
                self.pocket_floating = true;
                self.pocket_opened_from_drag = false;
                return;
            }
        }
        let anchor = ctx.input(|input| {
            let outer = input.viewport().outer_rect.map(|rect| rect.min)
                .unwrap_or(egui::Pos2::ZERO);
            let pointer = input.pointer.hover_pos().unwrap_or(egui::pos2(80.0, 50.0));
            let screen = outer + pointer.to_vec2();
            let monitor = input.viewport().monitor_size.unwrap_or(Vec2::new(1440.0, 900.0));
            let x = if screen.x + 430.0 > monitor.x {
                (screen.x - 430.0).max(8.0)
            } else { (screen.x + 22.0).max(8.0) };
            let y = if screen.y + 280.0 > monitor.y {
                (screen.y - 280.0).max(8.0)
            } else { (screen.y + 18.0).max(8.0) };
            egui::pos2(x, y)
        });
        self.pocket_anchor = anchor;
        self.pocket_floating = true;
        self.pocket_opened_from_drag = false;
    }
    fn start_pocket_action(&mut self, source: PathBuf, action: pocket::Action) {
        if self.pocket_job.is_some() { return; }
        let Some(root) = preferences_path().and_then(|path| path.parent()
            .map(|parent| parent.join("Pocket"))) else {
            self.notify("Pocket storage is unavailable");
            return;
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let copy = source.clone();
        std::thread::spawn(move || {
            let _ = sender.send(pocket::execute(action, &copy, &root));
        });
        self.pocket_job = Some(PocketJob { source,
            operation: PocketOperation::Action(action), receiver });
        self.pocket_activity = None;
        self.pocket_status = match action {
            pocket::Action::Keep => "Keeping a copy…",
            pocket::Action::Png | pocket::Action::Jpeg |
            pocket::Action::Heic | pocket::Action::Tiff |
            pocket::Action::Webp => "Converting image…",
            pocket::Action::Aiff | pocket::Action::Alac |
            pocket::Action::Flac => "Converting audio…",
            pocket::Action::Docx | pocket::Action::Html => "Converting document…",
            pocket::Action::Zip => "Creating ZIP…",
        }.into();
    }
    fn start_pocket_pipeline(&mut self, source: PathBuf, pipeline: pipelines::Pipeline) {
        if self.pocket_job.is_some() || !pipeline.can_run(&source) { return; }
        let Some(root) = preferences_path().and_then(|path| path.parent()
            .map(|parent| parent.join("Pocket"))) else {
            self.notify("Pocket storage is unavailable");
            return;
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let copy = source.clone();
        let name = pipeline.name.clone();
        std::thread::spawn(move || {
            let _ = sender.send(pipelines::run(&pipeline, &copy, &root));
        });
        self.pocket_job = Some(PocketJob { source,
            operation: PocketOperation::Pipeline(name.clone()), receiver });
        self.pocket_activity = None;
        self.pocket_status = format!("Running pipeline: {name}…");
    }
    fn poll_pocket(&mut self) {
        use std::sync::mpsc::TryRecvError;
        let result = self.pocket_job.as_ref().and_then(|job| match job.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Disconnected) => Some(Err("Pocket operation stopped unexpectedly".into())),
            Err(TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            let job = self.pocket_job.take().unwrap();
            match result {
                Ok(path) => {
                    let is_conversion = match &job.operation {
                        PocketOperation::Pipeline(_) => true,
                        PocketOperation::Action(action) =>
                            !matches!(action, pocket::Action::Keep | pocket::Action::Zip),
                    };
                    let short = match &job.operation {
                        PocketOperation::Action(pocket::Action::Keep) => "Copy".into(),
                        PocketOperation::Pipeline(_) => "Pipeline".into(),
                        _ => path.extension().and_then(|value| value.to_str())
                            .unwrap_or("File").to_ascii_uppercase(),
                    };
                    if matches!(job.operation, PocketOperation::Action(pocket::Action::Keep)) {
                        self.files.retain(|item| item != &job.source);
                    }
                    self.files.retain(|item| item != &path);
                    self.files.insert(0, path.clone());
                    self.files.truncate(8);
                    self.save_data();
                    self.pocket_status = match job.operation {
                        PocketOperation::Pipeline(name) => format!("Pipeline complete: {name}"),
                        PocketOperation::Action(action) => match action {
                            pocket::Action::Keep => "Saved a copy in Pocket",
                            pocket::Action::Png => "Converted to PNG in Pocket",
                            pocket::Action::Jpeg => "Converted to JPEG in Pocket",
                            pocket::Action::Heic => "Converted to HEIC in Pocket",
                            pocket::Action::Tiff => "Converted to TIFF in Pocket",
                            pocket::Action::Webp => "Converted to WebP in Pocket",
                            pocket::Action::Aiff => "Converted to AIFF in Pocket",
                            pocket::Action::Alac => "Converted to ALAC in Pocket",
                            pocket::Action::Flac => "Converted to FLAC in Pocket",
                            pocket::Action::Docx => "Converted to DOCX in Pocket",
                            pocket::Action::Html => "Converted to HTML in Pocket",
                            pocket::Action::Zip => "Created ZIP in Pocket",
                        }.into(),
                    };
                    let cue = format!("{short} ready");
                    self.pocket_activity = Some((cue.clone(), true, Instant::now()));
                    if is_conversion {
                        self.pending_result_card = Some(FileResultCard::new(path,
                            "Converted in Pocket".into(), Duration::from_secs(6)));
                    } else {
                        self.pending_activity_bubble = Some(cue);
                    }
                    self.notify(self.pocket_status.clone());
                }
                Err(error) => {
                    self.pocket_status = format!("Pocket: {error}");
                    let cue = "File action failed".to_owned();
                    self.pocket_activity = Some((cue.clone(), false, Instant::now()));
                    self.pending_activity_bubble = Some(cue);
                    self.notify(self.pocket_status.clone());
                }
            }
        }
        if self.pocket_activity.as_ref().is_some_and(|(_, _, at)|
            at.elapsed() >= Duration::from_secs(6)) {
            self.pocket_activity = None;
        }
    }
    fn start_video_download(&mut self, input: &str) {
        if self.video_rx.is_some() { return; }
        let Some(url) = video::youtube_url(input) else {
            self.notify("Paste a YouTube video link");
            return;
        };
        let Some(tool) = video::tool_path() else {
            self.notify("Video downloader is missing from Bookland");
            return;
        };
        let folder = data_path().and_then(|path| path.parent().map(|parent| parent.join("Tray")))
            .unwrap_or_else(|| std::env::temp_dir().join("Bookland/Tray"));
        let (tx, rx) = std::sync::mpsc::channel();
        self.video_rx = Some(rx);
        self.video_offer = None;
        self.video_url_draft = url.clone();
        self.video_progress = VideoProgress::default();
        self.video_status = "Starting download…".into();
        self.video_stopping = false;
        self.video_saved_path = None;
        std::thread::spawn(move || video::download(&tool, &url, &folder, tx));
    }
    fn send_codex(&mut self) {
        let prompt = self.codex_draft.trim().to_owned();
        if prompt.is_empty() || self.codex_rx.is_some() { return; }
        let session = self.codex_session_id.clone();
        let codex_prompt = if session.is_some() { prompt.clone() }
            else { compose_codex_turn(&prompt, &self.codex_messages) };
        self.codex_messages.push(CodexChatLine { role: "You".into(), text: prompt.clone() });
        self.codex_draft.clear();
        self.codex_status = "Connecting to Codex…".into();
        self.codex_stopping = false;
        self.codex_rx = Some(codex_bridge::run(codex_prompt,
            codex_workspace(), session, self.prefs.codex_allow_edits));
        self.save_data();
    }
    fn poll_codex(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(receiver) = &self.codex_rx {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        let mut save = false;
        for event in events {
            match event {
                CodexEvent::Process(pid) => self.codex_pid = Some(pid),
                CodexEvent::Thread(id) => { self.codex_session_id = Some(id); save = true; }
                CodexEvent::ResetThread => { self.codex_session_id = None; save = true; }
                CodexEvent::Progress(text) => self.codex_status = text,
                CodexEvent::Message(text) => {
                    self.codex_messages.push(CodexChatLine { role: "Codex".into(), text });
                    if self.codex_messages.len() > 40 {
                        self.codex_messages.drain(..self.codex_messages.len() - 40);
                    }
                    save = true;
                }
                CodexEvent::Finished(result) => {
                    self.codex_status = if self.codex_stopping { "Stopped".into() }
                        else { match result {
                            Ok(()) => "Ready".into(),
                            Err(error) => format!("Codex: {error}"),
                        }};
                    self.codex_rx = None;
                    self.codex_pid = None;
                    self.codex_stopping = false;
                    save = true;
                }
            }
        }
        if disconnected && self.codex_rx.is_some() {
            self.codex_rx = None;
            self.codex_pid = None;
            self.codex_status = "Codex connection ended unexpectedly".into();
        }
        if save { self.save_data(); }
    }
    fn desktop_codex_busy(&self) -> bool {
        self.desktop_codex_thread_id.as_ref().is_some_and(|id|
            self.agents.iter().any(|agent| agent.status == "Working"
                && desktop_codex_thread_id(&agent.path).as_ref() == Some(id)))
    }
    fn send_desktop_codex(&mut self) {
        let Some(id) = self.desktop_codex_thread_id.clone() else { return };
        let prompt = self.desktop_codex_draft.trim().to_owned();
        if prompt.is_empty() || self.desktop_codex_rx.is_some() || self.desktop_codex_busy() { return; }
        self.desktop_codex_messages.push(CodexChatLine { role: "You".into(), text: prompt.clone() });
        self.desktop_codex_draft.clear();
        self.desktop_codex_status = "Connecting to the selected Codex chat…".into();
        self.desktop_codex_stopping = false;
        self.desktop_codex_approval = None;
        self.desktop_codex_question = None;
        self.desktop_codex_question_answers.clear();
        let (events, decisions) = codex_desktop::run(id, prompt,
            self.desktop_codex_allow_edits, self.pocket_wake_context.as_ref().clone());
        self.desktop_codex_rx = Some(events);
        self.desktop_codex_decision_tx = Some(decisions);
    }
    fn show_desktop_codex_request(&mut self) {
        if let Some(id) = &self.desktop_codex_thread_id {
            if let Some(index) = self.agents.iter().position(|agent|
                agent.source == "Codex" && desktop_codex_thread_id(&agent.path).as_ref() == Some(id)) {
                self.selected_agent = index;
            }
        }
        self.workspace = Workspace::Agents;
        self.agent_usage_view = false;
        self.agent_chat_desktop = true;
        self.agent_chat_open = true;
        self.expanded = true;
        // Keep a decision open until the user answers or closes it.
        self.opened_by_hover = false;
        self.hover_blocked_until_exit = false;
    }
    fn poll_desktop_codex(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(receiver) = &self.desktop_codex_rx {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => { disconnected = true; break; }
                }
            }
        }
        for event in events {
            match event {
                codex_desktop::Event::Process(pid) => self.desktop_codex_pid = Some(pid),
                codex_desktop::Event::Progress(text) => {
                    if text == "Waiting for Codex…" {
                        self.desktop_codex_approval = None;
                        self.desktop_codex_question = None;
                        self.desktop_codex_question_answers.clear();
                    }
                    self.desktop_codex_status = text;
                }
                codex_desktop::Event::Approval(request) => {
                    self.desktop_codex_approval = Some(request);
                    self.desktop_codex_status = "Codex is waiting for your decision".into();
                    self.show_desktop_codex_request();
                }
                codex_desktop::Event::Question(request) => {
                    self.desktop_codex_question_answers =
                        vec![String::new(); request.questions.len()];
                    self.desktop_codex_question = Some(request);
                    self.desktop_codex_status = "Codex has a question for you".into();
                    self.show_desktop_codex_request();
                }
                codex_desktop::Event::Message(text) => {
                    self.desktop_codex_messages.push(CodexChatLine { role: "Codex".into(), text });
                    if self.desktop_codex_messages.len() > 30 {
                        self.desktop_codex_messages.drain(..self.desktop_codex_messages.len() - 30);
                    }
                }
                codex_desktop::Event::Finished(result) => {
                    self.desktop_codex_status = if self.desktop_codex_stopping { "Stopped".into() }
                        else { match result { Ok(()) => "Ready".into(),
                            Err(error) => format!("Codex: {error}") } };
                    self.desktop_codex_rx = None;
                    self.desktop_codex_decision_tx = None;
                    self.desktop_codex_approval = None;
                    self.desktop_codex_question = None;
                    self.desktop_codex_question_answers.clear();
                    self.desktop_codex_pid = None;
                    self.desktop_codex_stopping = false;
                }
            }
        }
        if disconnected && self.desktop_codex_rx.is_some() {
            self.desktop_codex_rx = None;
            self.desktop_codex_decision_tx = None;
            self.desktop_codex_approval = None;
            self.desktop_codex_question = None;
            self.desktop_codex_question_answers.clear();
            self.desktop_codex_pid = None;
            self.desktop_codex_status = "Codex desktop connection ended unexpectedly".into();
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        self.poll_codex();
        self.poll_desktop_codex();
        self.poll_pocket();
        self.poll_music_airplay();
        self.poll_mirror(ctx);
        self.poll_audio_spectrum(ctx);
        let mut video_events = Vec::new();
        if let Some(receiver) = &self.video_rx {
            while let Ok(event) = receiver.try_recv() { video_events.push(event); }
        }
        for event in video_events {
            match event {
                VideoEvent::Started(pid) => {
                    self.video_pid = Some(pid);
                    self.video_status = "Downloading…".into();
                }
                VideoEvent::Progress(progress) => {
                    self.video_progress = progress;
                    self.video_status = "Downloading…".into();
                }
                VideoEvent::Saved(path) => {
                    self.files.retain(|item| item != &path);
                    self.files.insert(0, path.clone());
                    self.files.truncate(8);
                    self.video_saved_path = Some(path);
                    self.save_data();
                }
                VideoEvent::Finished(result) => {
                    self.video_rx = None;
                    self.video_pid = None;
                    let succeeded = result.is_ok();
                    self.video_status = match result {
                        Ok(()) => "Saved to Tray".into(),
                        Err(_) if self.video_stopping => "Download cancelled".into(),
                        Err(error) => format!("Download failed: {error}"),
                    };
                    if succeeded {
                        if let Some(path) = self.video_saved_path.take() {
                            let size = path.metadata().map(|meta| meta.len()).unwrap_or(0);
                            let subtitle = format!("YouTube · {}",
                                downloads::format_bytes(size));
                            self.pending_result_card = Some(FileResultCard::new(path,
                                subtitle, Duration::from_secs(5)));
                        }
                    } else {
                        self.video_saved_path = None;
                    }
                    let message = self.video_status.clone();
                    self.notify(message);
                    self.video_stopping = false;
                }
            }
        }
        if let Some(result) = self.note_export_rx.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.note_export_rx = None;
            match result {
                Ok(()) => self.notify("Saved a copy in Apple Notes"),
                Err(error) => self.notify(format!("Apple Notes: {error}")),
            }
        }
        if let Some(receiver) = &self.agent_rx {
            if let Ok(agents) = receiver.try_recv() {
                let selected = self.agents.get(self.selected_agent)
                    .map(|agent| (agent.source.clone(), agent.path.clone()));
                if self.agents_initialized {
                    if let Some(done) = agents::newly_finished(&self.agents, &agents) {
                        self.agent_completion = Some(AgentCompletion {
                            source: done.source.clone(),
                            path: done.path.clone(),
                            created_at: Instant::now(),
                        });
                        self.pending_activity_bubble = Some(format!("{} finished", done.source));
                    }
                }
                if self.agent_completion.as_ref().is_some_and(|done|
                    !agents.iter().any(|agent| agent.path == done.path
                        && agent.source == done.source && agent.status == "Finished")) {
                    self.agent_completion = None;
                }
                self.agents_initialized = true;
                self.agents = agents;
                self.agents.extend(self.craft_agents.clone());
                self.agents.sort_by_key(|agent| std::cmp::Reverse(agent.updated));
                self.agents.truncate(8);
                self.selected_agent = selected_agent_index(selected, self.selected_agent, &self.agents);
                self.agent_rx = None;
            }
        }
        if let Some(receiver) = &self.craft_rx {
            if let Ok(craft_agents) = receiver.try_recv() {
                let selected = self.agents.get(self.selected_agent)
                    .map(|agent| (agent.source.clone(), agent.path.clone()));
                self.craft_agents = craft_agents;
                self.agents.retain(|agent| agent.source != "Craft Anything");
                self.agents.extend(self.craft_agents.clone());
                self.agents.sort_by_key(|agent| std::cmp::Reverse(agent.updated));
                self.agents.truncate(8);
                self.selected_agent = selected_agent_index(selected, self.selected_agent, &self.agents);
                self.craft_rx = None;
            }
        }
        if let Some(receiver) = &self.downloads_rx {
            if let Ok(result) = receiver.try_recv() {
                self.downloads_rx = None;
                match result {
                    Ok(mut current) => {
                        let completed = downloads::update_rates(&self.downloads,
                            &mut current, self.last_download_sample.elapsed());
                        self.downloads = current;
                        self.downloads_error = None;
                        self.last_download_sample = Instant::now();
                        if let Some(name) = completed.first() {
                            self.notify(format!("Download finished: {name}"));
                            if self.prefs.watch_downloads {
                                let result = self.downloads.iter().find(|item|
                                    !item.active && item.path.file_name().is_some_and(|file|
                                        file.to_string_lossy().as_ref() == name.as_str()))
                                    .map(|item| (item.path.clone(), item.bytes));
                                if let Some((path, bytes)) = result {
                                    let subtitle = format!("Download · {}",
                                        downloads::format_bytes(bytes));
                                    self.pending_result_card = Some(FileResultCard::new(path,
                                        subtitle, Duration::from_secs(3)));
                                } else {
                                    self.pending_activity_bubble = Some("Download ready".into());
                                }
                            } else {
                                self.pending_activity_bubble = Some("Download ready".into());
                            }
                        }
                    }
                    Err(error) => {
                        self.downloads.clear();
                        self.downloads_error = Some(error);
                    }
                }
            }
        }
        if let Some(receiver) = &self.weather_rx {
            if let Ok(weather) = receiver.try_recv() {
                self.weather = weather;
                self.weather_rx = None;
            }
        }
        #[cfg(target_os = "macos")]
        if self.airpods_events_attempted && !self.airpods_events_enabled
            && unsafe { bookland_airpods_watch_status() != 0 } {
            self.airpods_events_enabled = true;
            self.airpods_refresh_pending = true;
        }
        #[cfg(target_os = "macos")]
        if self.airpods_events_enabled && unsafe { bookland_airpods_events_take() != 0 } {
            self.airpods_refresh_pending = true;
        }
        if let Some(receiver) = &self.airpods_rx {
            if let Ok(status) = receiver.try_recv() {
                if self.airpods_baseline_seen {
                    if let Some(label) = airpods::connection_activity(&self.airpods, &status) {
                        self.airpods_activity = Some((label.clone(), Instant::now()));
                        self.pending_activity_bubble = Some(label);
                    }
                }
                self.airpods_baseline_seen = true;
                self.airpods = status;
                self.airpods_rx = None;
            }
        }
        if self.airpods_activity.as_ref().is_some_and(|(_, at)|
            at.elapsed() >= Duration::from_secs(4)) {
            self.airpods_activity = None;
        }
        let airpods_interval = if self.airpods_events_enabled {
            if self.airpods.name.is_some() { 60 } else { 300 }
        } else { 30 };
        if self.airpods_rx.is_none()
            && (self.airpods_refresh_pending ||
                self.last_airpods_poll.elapsed() >= Duration::from_secs(airpods_interval)) {
            let (tx, rx) = std::sync::mpsc::channel();
            self.airpods_rx = Some(rx);
            self.last_airpods_poll = Instant::now();
            self.airpods_refresh_pending = false;
            std::thread::spawn(move || { let _ = tx.send(airpods::fetch()); });
        }
        if self.prefs.widgets.contains(&Widget::Weather)
            && !self.prefs.weather_city.trim().is_empty()
            && self.weather_rx.is_none()
            && self.last_weather_poll.elapsed() >= Duration::from_secs(900) {
            let city = self.prefs.weather_city.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            self.weather_rx = Some(rx);
            self.last_weather_poll = Instant::now();
            std::thread::spawn(move || { let _ = tx.send(weather::fetch(&city)); });
        }
        if let Some(receiver) = &self.terminal_rx {
            if let Ok(output) = receiver.try_recv() {
                self.terminal_output = output;
                self.terminal_rx = None;
            }
        }
        if (self.prefs.widgets.contains(&Widget::Downloads)
            || self.prefs.watch_downloads) && self.downloads_rx.is_none() {
            let interval = if self.downloads.iter().any(|entry| entry.active) { 2 } else { 10 };
            if self.last_download_poll.elapsed() >= Duration::from_secs(interval) {
                if let Some(folder) = downloads_root(&self.prefs.downloads_folder) {
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.downloads_rx = Some(rx);
                    std::thread::spawn(move || {
                        let _ = tx.send(downloads::scan(&folder));
                    });
                }
                self.last_download_poll = Instant::now();
            }
        }
        if self.screenshot.is_none() && self.prefs.agent_monitor && self.agent_rx.is_none()
            && self.last_agent_poll.elapsed() >= Duration::from_secs(5) {
            if let Some(home) = std::env::var_os("HOME") {
                let (tx, rx) = std::sync::mpsc::channel();
                self.agent_rx = Some(rx);
                std::thread::spawn(move || {
                    let _ = tx.send(agents::scan_local(&PathBuf::from(home)));
                });
            }
            self.last_agent_poll = Instant::now();
        }
        if self.prefs.monitor_craft && self.craft_rx.is_none()
            && self.last_craft_poll.elapsed() >= Duration::from_secs(10) {
            if let Some(home) = std::env::var_os("HOME") {
                let (tx, rx) = std::sync::mpsc::channel();
                self.craft_rx = Some(rx);
                std::thread::spawn(move || {
                    let _ = tx.send(agents::scan_craft(&PathBuf::from(home)));
                });
            }
            self.last_craft_poll = Instant::now();
        }
        if !self.prefs.monitor_craft && !self.craft_agents.is_empty() {
            self.craft_agents.clear();
            self.agents.retain(|agent| agent.source != "Craft Anything");
        }
        let calendar_poll_interval = if self.calendar_status == 0 { 1 } else { 30 };
        if self.workspace == Workspace::Calendar
            && self.last_calendar_poll.elapsed() >= Duration::from_secs(calendar_poll_interval) {
            #[cfg(target_os = "macos")]
            unsafe {
                self.calendar_status = nooklet_calendar_status();
                if self.calendar_status == 1 {
                    let selected = CString::new(self.prefs.calendar_id.as_str()).ok();
                    self.calendar_snapshot_version = nooklet_calendar_snapshot_version();
                    nooklet_calendar_refresh(selected.as_ref().map_or(
                        std::ptr::null(), |value| value.as_ptr()), self.calendar_offset);
                    self.calendar_query_pending = true;
                } else {
                    self.calendar_choices.clear();
                    self.calendar_events.clear();
                    self.calendar_query_pending = false;
                }
            }
            self.last_calendar_poll = Instant::now();
        }
        #[cfg(target_os = "macos")]
        if self.workspace == Workspace::Calendar && self.calendar_query_pending {
            unsafe {
                let version = nooklet_calendar_snapshot_version();
                if version != self.calendar_snapshot_version {
                    let mut choices = vec![0_u8; 65_536];
                    let count = nooklet_calendar_calendars_copy(
                        choices.as_mut_ptr().cast(), choices.len());
                    self.calendar_choices = serde_json::from_slice(&choices[..count])
                        .unwrap_or_default();
                    let mut buffer = vec![0_u8; 131_072];
                    let size = nooklet_calendar_copy(buffer.as_mut_ptr().cast(), buffer.len());
                    self.calendar_events = serde_json::from_slice(&buffer[..size])
                        .unwrap_or_default();
                    self.calendar_snapshot_version = version;
                    self.calendar_query_pending = false;
                }
            }
        }
        let system_poll_interval = if self.expanded {
            self.prefs.pulse_interval.max(1)
        } else { 12 };
        let system_result = self.system_rx.as_ref().map(|receiver| receiver.try_recv());
        match system_result {
            Some(Ok(snapshot)) => {
                self.system_rx = None;
                self.battery = snapshot.battery;
                self.volume = snapshot.volume;
                self.disk = snapshot.disk;
                if let Some(bytes) = snapshot.network_bytes {
                    if let (Some(previous), Some(previous_at)) =
                        (self.last_network_bytes, self.last_network_sample_at) {
                        self.network = network_rate(
                            bytes.0.saturating_sub(previous.0)
                                + bytes.1.saturating_sub(previous.1),
                            snapshot.sampled_at.duration_since(previous_at).as_secs_f32(),
                        );
                    }
                    self.last_network_bytes = Some(bytes);
                    self.last_network_sample_at = Some(snapshot.sampled_at);
                }
                if let Some(cpu) = snapshot.cpu { self.cpu = cpu; }
                if let Some(ram) = snapshot.ram { self.ram = ram; }
                if let Some(gpu) = snapshot.gpu { self.gpu = gpu; }
                record_sample(&mut self.cpu_history, percent_value(&self.cpu));
                record_sample(&mut self.ram_history, percent_value(&self.ram));
                record_sample(&mut self.gpu_history, percent_value(&self.gpu));
                record_sample(&mut self.battery_history, percent_value(&self.battery));
                record_sample(&mut self.disk_history, percent_value(&self.disk));
            }
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.system_rx = None;
            }
            _ => {}
        }
        if self.system_rx.is_none()
            && self.last_system_poll.elapsed() >= Duration::from_secs(system_poll_interval) {
            let include_gpu = self.expanded && (self.prefs.pulse_left == "GPU"
                || self.prefs.pulse_right == "GPU");
            let memory_total = self.memory_total;
            let repaint = ctx.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            self.system_rx = Some(rx);
            self.last_system_poll = Instant::now();
            std::thread::spawn(move || {
                let _ = tx.send(collect_system_snapshot(include_gpu, memory_total));
                repaint.request_repaint();
            });
        }
        if self.prefs.capture_clipboard && self.last_clip_poll.elapsed() >= Duration::from_secs(2) {
            #[cfg(target_os = "macos")]
            let change_count = unsafe { bookland_clipboard_change_count() };
            #[cfg(not(target_os = "macos"))]
            let change_count = self.last_clip_change + 1;
            let new_copy = change_count != self.last_clip_change;
            self.last_clip_change = change_count;
            #[cfg(target_os = "macos")]
            let sensitive = self.prefs.clipboard_ignore_passwords
                && unsafe { bookland_clipboard_is_sensitive() != 0 };
            #[cfg(not(target_os = "macos"))]
            let sensitive = false;
            #[cfg(target_os = "macos")]
            let ignored_app = frontmost_bundle_id().is_some_and(|id|
                self.prefs.clipboard_ignore_apps.iter().any(|ignored| ignored == &id));
            #[cfg(not(target_os = "macos"))]
            let ignored_app = false;
            if new_copy && !sensitive && !ignored_app {
                #[cfg(target_os = "macos")]
                let native_files = clipboard_file_urls();
                #[cfg(not(target_os = "macos"))]
                let native_files: Vec<PathBuf> = Vec::new();
                if !native_files.is_empty() {
                    let favorite = self.clipboard_files.iter()
                        .find(|item| item.paths == native_files)
                        .is_some_and(|item| item.favorite);
                    self.clipboard_files.retain(|item| item.paths != native_files);
                    self.clipboard_files.insert(0, ClipboardFiles {
                        paths: native_files, favorite,
                    });
                    trim_clipboard_files(&mut self.clipboard_files,
                        self.prefs.clipboard_history_size.clamp(1, 1000));
                    if self.prefs.persist_clipboard { self.save_data(); }
                } else if let Ok(mut clip) = arboard::Clipboard::new() {
                let mut has_image = false;
                if let Ok(image) = clip.get_image() {
                    has_image = true;
                    let width = image.width;
                    let height = image.height;
                    let bytes = image.bytes.into_owned();
                    if width > 0 && height > 0 && width <= 4096 && height <= 4096
                        && bytes.len() == width.saturating_mul(height).saturating_mul(4)
                        && bytes.len() <= 8_000_000 {
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        width.hash(&mut hasher);
                        height.hash(&mut hasher);
                        bytes.hash(&mut hasher);
                        let fingerprint = hasher.finish();
                        if self.last_image_fingerprint != Some(fingerprint) {
                            self.last_image_fingerprint = Some(fingerprint);
                            if let Some(full) = image::RgbaImage::from_raw(width as u32, height as u32, bytes.clone()) {
                                let thumb = image::imageops::thumbnail(&full, 256, 160);
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    [thumb.width() as usize, thumb.height() as usize], thumb.as_raw());
                                let texture = ctx.load_texture(format!("clipboard-{fingerprint}"),
                                    color_image, egui::TextureOptions::LINEAR);
                                let favorite = self.clipboard_images.iter()
                                    .find(|item| item.fingerprint == fingerprint)
                                    .is_some_and(|item| item.favorite);
                                self.clipboard_images.retain(|item| item.fingerprint != fingerprint);
                                self.clipboard_images.insert(0, ClipboardImage {
                                    fingerprint, width, height, rgba: bytes, texture, favorite,
                                });
                                self.clipboard_images.truncate(clipboard_images::MAX_IMAGES);
                                if self.prefs.persist_clipboard { self.save_data(); }
                            }
                        }
                    }
                }
                if !has_image { if let Ok(text) = clip.get_text() {
                    let value = text.trim().to_owned();
                    if !value.is_empty() && value.len() <= 32_000 && value != self.last_clip {
                        self.last_clip = value.clone();
                        if let Some(url) = video::youtube_url(&value) {
                            self.video_offer = Some(url.clone());
                            self.video_url_draft = url;
                            self.clipboard_search.clear();
                        }
                        self.clipboard.retain(|item| item != &value);
                        self.clipboard.insert(0, value);
                        trim_clipboard_history(&mut self.clipboard,
                            &self.clipboard_favorites, self.prefs.clipboard_history_size.clamp(1, 1000));
                        if self.prefs.persist_clipboard { self.save_data(); }
                    }
                }}
                }
            }
            self.last_clip_poll = Instant::now();
        }
        let media_result = self.media_rx.as_ref().map(|receiver| receiver.try_recv());
        match media_result {
            Some(Ok(snapshot)) => {
                self.media_rx = None;
                self.media_app = snapshot.app;
                self.media_title = snapshot.title;
                self.media_artist = snapshot.artist;
                self.media_position = snapshot.position;
                self.media_duration = snapshot.duration;
                self.media_playing = snapshot.playing;
                self.last_media_poll = Instant::now();
                self.last_media_request = Instant::now();
            }
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.media_rx = None;
                self.last_media_request = Instant::now();
            }
            _ => {}
        }
        if self.media_rx.is_none() && self.last_media_request.elapsed() >= Duration::from_secs(5) {
            let source = self.prefs.media_source.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            self.media_rx = Some(receiver);
            self.last_media_request = Instant::now();
            std::thread::spawn(move || { let _ = sender.send(read_media(&source)); });
        }
        while let Ok((success, previous_playing)) = self.media_action_rx.try_recv() {
            self.media_actions_pending = self.media_actions_pending.saturating_sub(1);
            if !success {
                if let Some(previous_playing) = previous_playing {
                    self.media_playing = previous_playing;
                }
                self.notify("Playback control was unavailable");
            }
            self.media_rx = None;
            self.last_media_request = Instant::now() - Duration::from_secs(6);
        }
        if let Some(result) = self.artwork_rx.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.artwork_rx = None;
            self.artwork_texture = result.ok().flatten().map(|pixels|
                ctx.load_texture("current-album-artwork", pixels, egui::TextureOptions::LINEAR));
        }
        let artwork_key = (self.media_app.clone().unwrap_or_default(),
            self.media_title.clone(), self.media_artist.clone());
        let can_fetch_artwork = self.media_app.is_some() && self.media_title != "Nothing playing";
        if can_fetch_artwork && self.artwork_for.as_ref() != Some(&artwork_key) {
            self.artwork_for = Some(artwork_key.clone());
            self.artwork_texture = None;
            let (sender, receiver) = std::sync::mpsc::channel();
            self.artwork_rx = Some(receiver);
            std::thread::spawn(move || {
                let result = if artwork_key.0 == "Music" || artwork_key.0 == "Spotify" {
                    artwork::fetch(&artwork_key.0)
                } else {
                    system_artwork()
                }.map(|image| image.and_then(|bytes| artwork::decode_thumbnail(&bytes)));
                let _ = sender.send(result);
            });
        } else if !can_fetch_artwork && self.artwork_for.is_some() {
            self.artwork_for = None;
            self.artwork_rx = None;
            self.artwork_texture = None;
        }
        if let Some(result) = self.lyrics_rx.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.lyrics_rx = None;
            match result {
                Ok(lines) => { self.lyrics_lines = lines; self.lyrics_status.clear(); }
                Err(error) => { self.lyrics_lines.clear(); self.lyrics_status = error; }
            }
        }
        let lyric_key = (self.media_title.clone(), self.media_artist.clone());
        let can_lookup_lyrics = self.prefs.show_synced_lyrics && self.media_app.is_some()
            && self.media_title != "Nothing playing" && !self.media_artist.is_empty();
        if can_lookup_lyrics && self.lyrics_for.as_ref() != Some(&lyric_key) {
            self.lyrics_for = Some(lyric_key.clone());
            self.lyrics_lines.clear();
            self.lyrics_status = "Looking up synced lyrics…".into();
            let (sender, receiver) = std::sync::mpsc::channel();
            self.lyrics_rx = Some(receiver);
            std::thread::spawn(move || {
                let result = lyrics::fetch(&lyric_key.0, &lyric_key.1);
                let _ = sender.send(result);
            });
        } else if !can_lookup_lyrics && self.lyrics_for.is_some() {
            self.lyrics_for = None;
            self.lyrics_rx = None;
            self.lyrics_lines.clear();
            self.lyrics_status.clear();
            self.lyrics_open = false;
        }
        if self.queue_open && self.media_app.as_deref() == Some("Music")
            && self.media_title != "Nothing playing" {
            if let Some(result) = self.queue_rx.as_ref().map(|rx| rx.try_recv()) {
                match result {
                    Ok(preview) => {
                        self.queue_rx = None;
                        self.queue_preview = preview;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.queue_rx = None;
                        self.queue_preview = music_queue::Preview {
                            tracks: Vec::new(),
                            status: "Apple Music playlist read ended unexpectedly.".into(),
                        };
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
            }
            let key = (self.media_title.clone(), self.media_artist.clone());
            if self.queue_for.as_ref() != Some(&key) {
                self.queue_for = Some(key);
                self.queue_preview = music_queue::Preview {
                    tracks: Vec::new(), status: "Reading Apple Music playlist…".into(),
                };
                let (tx, rx) = std::sync::mpsc::channel();
                self.queue_rx = Some(rx);
                std::thread::spawn(move || { let _ = tx.send(music_queue::read()); });
            }
        } else if self.queue_for.is_some() || self.queue_rx.is_some() {
            self.queue_for = None;
            self.queue_rx = None;
            self.queue_preview = music_queue::Preview {
                tracks: Vec::new(), status: String::new(),
            };
        }
        if let Some(end) = self.timer_end {
            if Instant::now() >= end {
                self.timer_end = None;
                self.timer_done = true;
                self.notify("Pomodoro finished");
                if self.prefs.notify_timer {
                    let _ = Command::new("osascript")
                        .args(["-e", "display notification \"Time for a break\" with title \"Bookland\""])
                        .spawn();
                }
            }
        }
        for file in ctx.input(|i| i.raw.dropped_files.clone()) {
            if let Some(path) = file.path {
                self.add_pocket_file(path);
            }
        }
    }
    fn poll_audio_spectrum(&mut self, ctx: &egui::Context) {
        let wanted = self.prefs.audio_spectrum && self.expanded
            && self.workspace == Workspace::Music && !self.preview_mode
            && self.screenshot.is_none();
        #[cfg(target_os = "macos")]
        unsafe {
            if wanted != self.spectrum_capture_active {
                bookland_audio_spectrum_set(wanted as i32);
                self.spectrum_capture_active = wanted;
            }
            if wanted {
                self.spectrum_state = bookland_audio_spectrum_copy(
                    self.spectrum_levels.as_mut_ptr(), self.spectrum_levels.len());
                let audible = self.spectrum_state == 2
                    && self.spectrum_levels.iter().any(|level| *level > 0.015);
                // An idle system tap needs only occasional checks; the active
                // spectrum still follows audio at its normal cadence.
                ctx.request_repaint_after(Duration::from_millis(if audible { 50 } else { 250 }));
            } else {
                self.spectrum_state = 0;
                self.spectrum_levels.fill(0.0);
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (wanted, ctx);
    }
    fn paint_audio_spectrum(&self, ui: &mut egui::Ui, width: f32, height: f32) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
        if self.spectrum_state != 2
            || !self.spectrum_levels.iter().any(|level| *level > 0.015) {
            ui.painter().line_segment([
                egui::pos2(rect.left(), rect.bottom() - 1.0),
                egui::pos2(rect.right(), rect.bottom() - 1.0),
            ], Stroke::new(0.7, Color32::from_rgba_unmultiplied(167, 181, 205, 52)));
            return;
        }
        let width = rect.width() / self.spectrum_levels.len() as f32;
        for (index, raw) in self.spectrum_levels.iter().enumerate() {
            let level = if self.spectrum_state == 2 { raw.clamp(0.0, 1.0) } else { 0.0 };
            let bar_height = 2.0 + level * (rect.height() - 2.0);
            let x = rect.left() + index as f32 * width + width * 0.15;
            let bar = egui::Rect::from_min_size(
                egui::pos2(x, rect.bottom() - bar_height),
                Vec2::new(width * 0.7, bar_height));
            ui.painter().rect_filled(bar, 2.0,
                Color32::from_rgb(113, 180, 252));
        }
    }
    fn remaining(&self) -> u64 {
        self.timer_end
            .map(|t| t.saturating_duration_since(Instant::now()).as_secs())
            .unwrap_or(self.timer_secs)
    }
    fn stopwatch_elapsed(&self) -> Duration {
        self.stopwatch_saved + self.stopwatch_started.map(|start| start.elapsed()).unwrap_or_default()
    }
    fn toggle_stopwatch(&mut self) {
        if let Some(started) = self.stopwatch_started.take() {
            self.stopwatch_saved += started.elapsed();
        } else {
            self.timer_end = None;
            self.stopwatch_started = Some(Instant::now());
        }
    }
    fn toggle_volume_mute(&mut self) {
        if let Some(previous) = self.muted_volume.take() {
            self.volume = previous;
        } else {
            self.muted_volume = Some(self.volume.max(1.0));
            self.volume = 0.0;
        }
        set_system_volume(self.volume);
    }
    fn open_media_player(&mut self) {
        let player = if self.media_title != "Nothing playing" {
            self.media_app.as_deref().unwrap_or("Music")
        } else if self.prefs.media_source == "Spotify" {
            "Spotify"
        } else {
            "Music"
        };
        let opened = Command::new("/usr/bin/open").args(["-a", player])
            .stderr(Stdio::null()).status().is_ok_and(|status| status.success());
        if opened {
            if self.media_title == "Nothing playing" {
                self.notify(format!("Choose a song in {player}; control it here once it starts"));
            }
        } else {
            self.notify(format!("Could not open {player}"));
        }
    }
    fn media_action(&mut self, action: &str) -> bool {
        let source = self.prefs.media_source.clone();
        let system_source = source == "System" || source == "Auto";
        if self.media_title == "Nothing playing" {
            if action == "playpause" {
                if self.media_actions_pending > 0 { return false; }
                let player = if source == "Spotify" { "Spotify" } else { "Music" };
                let sender = self.media_action_tx.clone();
                self.media_actions_pending += 1;
                self.notify(format!("Opening {player}; playback will resume if a track is queued"));
                std::thread::spawn(move || {
                    let opened = Command::new("/usr/bin/open").args(["-a", player])
                        .stderr(Stdio::null()).status()
                        .is_ok_and(|status| status.success());
                    if opened { let _ = media_script(player, "play"); }
                    let _ = sender.send((opened, None));
                });
            } else {
                self.notify("Start a song before using playback controls");
            }
            return action == "playpause";
        }
        if self.media_app.is_none() && !system_source {
            self.notify("Play media to use playback controls");
            return false;
        }
        if self.media_actions_pending > 0 {
            self.notify("Playback is still responding");
            return false;
        }
        let previous_playing = (action == "playpause").then_some(self.media_playing);
        let action = if action == "playpause" {
            if self.media_playing {
                self.media_position = (self.media_position
                    + self.last_media_poll.elapsed().as_secs_f32())
                    .min(self.media_duration.max(0.0));
            }
            self.last_media_poll = Instant::now();
            self.media_playing = !self.media_playing;
            if self.media_playing { "play" } else { "pause" }.to_owned()
        } else { action.to_owned() };
        let app = self.media_app.clone();
        let sender = self.media_action_tx.clone();
        self.media_rx = None;
        self.media_actions_pending += 1;
        std::thread::spawn(move || {
            let success = if system_source {
                system_media_control(&action) || app.as_deref()
                    .filter(|app| *app == "Music" || *app == "Spotify")
                    .is_some_and(|app| media_script(app, &action).is_some())
            } else if let Some(app) = app {
                (system_media().is_some_and(|snapshot|
                    snapshot.app.as_deref() == Some(app.as_str()))
                    && system_media_control(&action))
                    || media_script(&app, &action).is_some()
            } else { false };
            let _ = sender.send((success, previous_playing));
        });
        true
    }
    fn skip_media(&mut self, backward: bool) {
        self.media_action(media_skip_command(backward));
    }
    fn seek_browser_media(&mut self, backward: bool) {
        let position = self.media_position
            + if self.media_playing { self.last_media_poll.elapsed().as_secs_f32() } else { 0.0 };
        if let Some(target) = browser_skip_target(position, self.media_duration, backward) {
            if self.media_action(&format!("set player position to {target}")) {
                self.media_position = target;
                self.last_media_poll = Instant::now();
            }
        } else {
            self.notify("This player did not provide a timeline");
        }
    }
    fn refresh_music_airplay(&mut self) {
        if self.airplay_rx.is_some() { return; }
        if running_media_app("Music").is_none() {
            self.airplay_devices.clear();
            self.airplay_status = "Open Apple Music to see its AirPlay outputs".into();
            return;
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        self.airplay_rx = Some(receiver);
        self.airplay_status = "Reading Apple Music outputs…".into();
        std::thread::spawn(move || { let _ = sender.send(music_airplay::list_music()); });
    }
    fn select_music_airplay(&mut self, name: String, selected: bool) {
        if self.airplay_rx.is_some() { return; }
        let (sender, receiver) = std::sync::mpsc::channel();
        self.airplay_rx = Some(receiver);
        self.airplay_status = if selected { format!("Connecting to {name}…") }
            else { format!("Disconnecting {name}…") };
        std::thread::spawn(move || {
            let result = music_airplay::set_music(&name, selected)
                .and_then(|()| music_airplay::list_music());
            let _ = sender.send(result);
        });
    }
    fn poll_music_airplay(&mut self) {
        use std::sync::mpsc::TryRecvError;
        let result = self.airplay_rx.as_ref().and_then(|receiver| match receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Disconnected) => Some(Err("Apple Music output request ended".into())),
            Err(TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            self.airplay_rx = None;
            match result {
                Ok(devices) => {
                    self.airplay_devices = devices;
                    self.airplay_status = if self.airplay_devices.iter().any(|device|
                        device.available && device.supports_audio) { String::new() }
                        else { "No available AirPlay audio outputs".into() };
                }
                Err(error) => self.airplay_status = format!("Apple Music: {error}"),
            }
        }
    }
    fn stop_mirror(&mut self) {
        #[cfg(target_os = "macos")]
        unsafe { bookland_mirror_stop(); }
        self.mirror_open = false;
        self.mirror_texture = None;
        self.mirror_sequence = 0;
        self.mirror_buffer.clear();
    }
    fn poll_mirror(&mut self, ctx: &egui::Context) {
        if !self.mirror_open { return; }
        if !self.expanded || self.workspace != Workspace::Nook
            || !self.prefs.widgets.contains(&Widget::Mirror) {
            self.stop_mirror();
            return;
        }
        #[cfg(target_os = "macos")]
        unsafe {
            if bookland_mirror_permission() == 3 && bookland_mirror_state() == 0 {
                let result = bookland_mirror_start();
                if result < 0 { self.mirror_error = "Camera could not start".into(); }
            }
            if bookland_mirror_state() < 0 {
                self.mirror_error = "Camera could not start".into();
                return;
            }
            if bookland_mirror_state() != 2 { return; }
            if self.mirror_buffer.is_empty() { self.mirror_buffer.resize(640 * 480 * 4, 0); }
            let mut width = 0;
            let mut height = 0;
            let mut sequence = 0;
            let mut length = bookland_mirror_copy(self.mirror_buffer.as_mut_ptr(),
                self.mirror_buffer.len(), &mut width, &mut height, &mut sequence);
            if length > self.mirror_buffer.len() {
                self.mirror_buffer.resize(length, 0);
                length = bookland_mirror_copy(self.mirror_buffer.as_mut_ptr(),
                    self.mirror_buffer.len(), &mut width, &mut height, &mut sequence);
            }
            if width <= 0 || height <= 0 || length != width as usize * height as usize * 4
                || sequence == self.mirror_sequence { return; }
            self.mirror_sequence = sequence;
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize], &self.mirror_buffer[..length]);
            if let Some(texture) = &mut self.mirror_texture {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.mirror_texture = Some(ctx.load_texture(
                    "bookland-mirror", image, egui::TextureOptions::LINEAR));
            }
        }
    }
    fn media_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let browser = is_browser_media_app(self.media_app.as_deref());
            if small_button(ui, "|◀").on_hover_text("Previous track").clicked() {
                self.skip_media(true);
            }
            if browser && small_button(ui, "−15")
                .on_hover_text("Back 15 seconds").clicked() {
                self.seek_browser_media(true);
            }
            let play = small_button(ui, if self.media_playing { "Ⅱ" } else { "▶" })
                .on_hover_text("Play or pause");
            play.widget_info(|| egui::WidgetInfo::labeled(
                egui::WidgetType::Button, true, "Play or pause"));
            if play.clicked() {
                self.media_action("playpause");
            }
            if browser && small_button(ui, "+15")
                .on_hover_text("Forward 15 seconds").clicked() {
                self.seek_browser_media(false);
            }
            if small_button(ui, "▶|").on_hover_text("Next track").clicked() {
                self.skip_media(false);
            }
            if self.prefs.extra_action == "Open app"
                || (browser && self.prefs.extra_action == "Shuffle") {
                if small_button(ui, "↗")
                    .on_hover_text("Open media player")
                    .clicked()
                {
                    self.open_media_player();
                }
            } else if self.prefs.extra_action == "Shuffle" {
                if small_button(ui, "Shuffle").on_hover_text("Toggle shuffle").clicked() {
                    if let Some(app) = &self.media_app {
                        let action = if app == "Music" {
                            "set shuffle enabled to not (shuffle enabled)"
                        } else if app == "Spotify" {
                            "set shuffling to not shuffling"
                        } else { "shuffle" };
                        self.media_action(action);
                    } else { self.notify("Play media to use shuffle"); }
                }
            }
        });
    }
    fn media_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.set_min_width(self.prefs.open_width - self.prefs.content_padding * 2.0 - 28.0);
            ui.horizontal(|ui| {
                label(ui, "•", 17.0, ACCENT);
                label(ui, "MEDIA PLAYER", 11.0, MUTED);
            });
            ui.add_space(7.0);
            let (art, _) = ui.allocate_exact_size(Vec2::splat(54.0), egui::Sense::hover());
            album_art(ui, art, self.artwork_texture.as_ref());
            music_text(ui, &self.media_title, 16.0, TEXT, false, false);
            music_text(ui, &self.media_artist, 12.0, MUTED, false, false);
            ui.add_space(9.0);
            self.media_controls(ui);
            if self.prefs.media_layout == "Expanded" {
                ui.add_space(8.0);
                label(
                    ui,
                    self.media_app.as_deref().unwrap_or("No playing app"),
                    11.0,
                    MUTED,
                );
                let mut position = self.media_position;
                let slider = ui.add_enabled(
                    self.media_duration > 0.0,
                    egui::Slider::new(&mut position, 0.0..=self.media_duration.max(1.0))
                        .show_value(false),
                );
                if slider.changed() {
                    self.media_position = position;
                    self.last_media_poll = Instant::now();
                }
                if self.media_app.is_some()
                    && (slider.drag_stopped() || (slider.changed() && !slider.dragged())) {
                    self.media_action(&format!("set player position to {}", position));
                }
            }
        });
    }
    fn pulse_metric(&self, name: &str) -> String {
        match name {
            "CPU" => self.cpu.clone(),
            "RAM" | "Memory" => self.ram.clone(),
            "GPU" => self.gpu.clone(),
            "Battery" => self.battery.clone(),
            "Disk" => self.disk.clone(),
            "Network" => self.network.clone(),
            _ => "—".into(),
        }
    }
    fn pulse_history(&self, name: &str) -> Option<&VecDeque<f32>> {
        match name {
            "CPU" => Some(&self.cpu_history),
            "RAM" | "Memory" => Some(&self.ram_history),
            "GPU" => Some(&self.gpu_history),
            "Battery" => Some(&self.battery_history),
            "Disk" => Some(&self.disk_history),
            _ => None,
        }
    }
    fn pulse_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "•", 16.0, ACCENT);
                label(ui, "PULSE", 11.0, MUTED);
            });
            ui.add_space(6.0);
            if self.prefs.pulse_style == "List" {
                for name in [&self.prefs.pulse_left, &self.prefs.pulse_right] {
                    ui.horizontal(|ui| {
                        label(ui, name, 13.0, MUTED);
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            label(ui, &self.pulse_metric(name), 14.0, TEXT)
                        });
                    });
                }
            } else {
                ui.columns(2, |cols| {
                    for (col, name) in cols
                        .iter_mut()
                        .zip([&self.prefs.pulse_left, &self.prefs.pulse_right])
                    {
                        label(col, name, 11.0, MUTED);
                        label(
                            col,
                            &self.pulse_metric(name),
                            if self.prefs.pulse_style == "Rings" {
                                21.0
                            } else {
                                24.0
                            },
                            TEXT,
                        );
                    }
                });
            }
            if self.prefs.pulse_explain {
                ui.label(
                    RichText::new("Live Mac readings; unavailable metrics show —")
                        .size(10.0)
                        .color(MUTED),
                )
                .on_hover_text("GPU usage appears on supported Macs; desktops may not report a battery.");
            }
        });
    }
    fn pomodoro_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "•", 16.0, ACCENT);
                label(ui, "POMODORO", 11.0, MUTED);
            });
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                label(ui, &format_time(self.remaining()), 27.0, TEXT);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if self.timer_end.is_some() {
                        if small_button(ui, "Pause").clicked() {
                            self.timer_secs = self.remaining();
                            self.timer_end = None;
                        }
                    } else if small_button(ui, "Start").clicked() {
                        self.timer_end =
                            Some(Instant::now() + Duration::from_secs(self.timer_secs));
                        self.timer_done = false;
                    }
                    if small_button(ui, "Reset").clicked() {
                        self.timer_end = None;
                        self.timer_secs = 25 * 60;
                        self.timer_done = false;
                    }
                });
            });
            ui.horizontal(|ui| {
                for (mins, text) in [(5, "5m"), (15, "15m"), (25, "25m")] {
                    if ui.small_button(text).clicked() {
                        self.timer_secs = mins * 60;
                        self.timer_end = None;
                        self.timer_done = false;
                    }
                }
            });
        });
    }
    fn todos_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "•", 16.0, ACCENT);
                label(ui, "TO-DOS", 11.0, MUTED);
            });
            let mut remove = None;
            for (i, todo) in self.prefs.todos.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut todo.done, &todo.text);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("×").on_hover_text("Remove to-do").clicked() {
                            remove = Some(i);
                        }
                    });
                });
            }
            if let Some(i) = remove {
                self.prefs.todos.remove(i);
            }
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.todo_draft)
                        .hint_text("Add a to-do")
                        .desired_width((ui.available_width() - 58.0).max(120.0)),
                );
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.small_button("Add").clicked() || enter) && !self.todo_draft.trim().is_empty()
                {
                    self.prefs.todos.push(TodoItem {
                        text: self.todo_draft.trim().into(),
                        done: false,
                    });
                    self.todo_draft.clear();
                }
            });
        });
    }
    fn notes_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            section_title(ui, "NOTES");
            if ui.add(egui::TextEdit::multiline(&mut self.notes)
                .hint_text("Keep a note here…").desired_rows(5).desired_width(f32::INFINITY)).changed() {
                self.save_data();
            }
            let can_export = !self.notes.trim().is_empty() && self.note_export_rx.is_none();
            if ui.add_enabled(can_export, egui::Button::new("Save copy to Apple Notes")).clicked() {
                let note = self.notes.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                self.note_export_rx = Some(rx);
                std::thread::spawn(move || { let _ = tx.send(export_to_apple_notes(&note)); });
            }
        });
    }
    fn clipboard_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            self.clipboard_browser(ui, true);
        });
    }
    fn mirror_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                section_title(ui, "MIRROR");
                if self.mirror_open {
                    if small_button(ui, "Stop camera").clicked() { self.stop_mirror(); }
                } else if small_button(ui, "Start camera").clicked() {
                    #[cfg(target_os = "macos")]
                    {
                        let result = unsafe { bookland_mirror_start() };
                        self.mirror_open = result >= 0;
                        self.mirror_error = match result {
                            -1 => "Camera access is unavailable".into(),
                            -2 => "No camera is connected".into(),
                            -3 => "Camera could not start".into(),
                            _ => String::new(),
                        };
                    }
                }
            });
            #[cfg(target_os = "macos")]
            {
                let available = unsafe { bookland_mirror_camera_available() };
                let permission = unsafe { bookland_mirror_permission() };
                if !available { label(ui, "No camera is connected", 12.0, MUTED); }
                else if permission == 2 { label(ui, "Camera access is off in System Settings → Privacy & Security.", 12.0, MUTED); }
                else if permission == 1 { label(ui, "Camera access is restricted on this Mac.", 12.0, MUTED); }
                else if !self.mirror_error.is_empty() { label(ui, &self.mirror_error, 12.0, MUTED); }
                else if let Some(texture) = &self.mirror_texture {
                    let width = ui.available_width().min(480.0).max(220.0);
                    let size = texture.size();
                    let height = width * size[1] as f32 / size[0] as f32;
                    ui.add(egui::Image::from_texture(texture)
                        .fit_to_exact_size(Vec2::new(width, height)).corner_radius(14));
                    label(ui, "Live mirrored camera", 11.0, MUTED);
                } else if self.mirror_open {
                    label(ui, if permission == 0 { "Waiting for camera permission…" }
                        else { "Starting camera…" }, 12.0, MUTED);
                } else {
                    label(ui, "Preview your camera in the Nook.", 12.0, MUTED);
                }
            }
            #[cfg(not(target_os = "macos"))]
            label(ui, "Mirror is available on macOS.", 12.0, MUTED);
        });
    }
    fn clipboard_toolbar_buttons(&mut self, ui: &mut egui::Ui, compact: bool) {
        let caption = if self.prefs.capture_clipboard { "Pause capture" } else { "Resume capture" };
        if ui.small_button(if self.prefs.capture_clipboard { "Pause" } else { "Resume" })
            .on_hover_text(caption).clicked() {
            self.prefs.capture_clipboard = !self.prefs.capture_clipboard;
            self.save_prefs();
        }
        if !compact && ui.small_button(if self.clipboard_bottom_sheet { "Top" } else { "Bottom" })
            .on_hover_text("Move Clipboard between the island and bottom of screen").clicked() {
            self.clipboard_bottom_sheet = !self.clipboard_bottom_sheet;
            self.prefs.clipboard_open_bottom = self.clipboard_bottom_sheet;
        }
    }
    fn clipboard_search_field(&mut self, ui: &mut egui::Ui, width: f32) {
        let search = ui.add(egui::TextEdit::singleline(&mut self.clipboard_search)
            .hint_text("Search clipboard")
            .desired_width(width));
        if self.clipboard_focus_pending {
            ui.memory_mut(|memory| memory.request_focus(search.id));
            self.clipboard_focus_pending = false;
        }
    }
    fn clipboard_filters(&mut self, ui: &mut egui::Ui, icons: bool) {
        for (filter, icon) in [
            ("All Items", "All"), ("Favorites", "★"), ("Text", "T"),
            ("Links", "↗"), ("Colors", "C"), ("Images", "I"), ("Files", "F"),
        ] {
            let active = self.clipboard_filter == filter;
            let button = egui::Button::new(RichText::new(if icons { icon } else { filter }).size(11.0)
                .color(if active { TEXT } else { MUTED }))
                .fill(if active { CARD_2 } else { Color32::TRANSPARENT })
                .corner_radius(12);
            if ui.add(button).on_hover_text(filter).clicked() {
                self.clipboard_filter = filter.into();
            }
        }
    }
    fn clipboard_card_header(ui: &mut egui::Ui, title: &str, width: f32, tint: Color32, shelf: bool) {
        if shelf {
            let (bar, _) = ui.allocate_exact_size(Vec2::new(width, 22.0), egui::Sense::hover());
            ui.painter().rect_filled(bar, CornerRadius { nw: 9, ne: 9, sw: 2, se: 2 }, tint);
            ui.painter().text(bar.left_center() + egui::vec2(9.0, 0.0), Align2::LEFT_CENTER,
                title, FontId::proportional(11.0), TEXT);
        } else {
            label(ui, &title.to_uppercase(), 10.0, ACCENT);
        }
    }
    fn clipboard_browser(&mut self, ui: &mut egui::Ui, compact: bool) {
        let shelf = self.clipboard_bottom_sheet && !compact;
        if shelf {
            ui.horizontal(|ui| {
                self.clipboard_search_field(ui, 230.0);
                ui.add_space(8.0);
                self.clipboard_filters(ui, true);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    self.clipboard_toolbar_buttons(ui, false);
                });
            });
        } else {
            ui.horizontal(|ui| {
                label(ui, "Clipboard", if compact { 14.0 } else { 20.0 }, TEXT);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    self.clipboard_toolbar_buttons(ui, compact);
                    label(ui, &format!("{} items", self.clipboard.len() + self.clipboard_images.len()
                        + self.clipboard_files.len()), 11.0, MUTED);
                });
            });
            ui.add_space(7.0);
            self.clipboard_search_field(ui, ui.available_width());
            ui.add_space(7.0);
            egui::ScrollArea::horizontal().id_salt(if compact { "clip_filters_widget" } else { "clip_filters" })
                .show(ui, |ui| {
                    ui.horizontal(|ui| self.clipboard_filters(ui, false));
                });
        }
        ui.add_space(8.0);
        let needle = self.clipboard_search.trim().to_lowercase();
        let items: Vec<String> = self.clipboard.iter()
            .filter(|item| (needle.is_empty() || item.to_lowercase().contains(&needle))
                && match self.clipboard_filter.as_str() {
                    "All Items" => true,
                    "Favorites" => self.clipboard_favorites.contains(item),
                    filter => clipboard_kind(item) == filter,
                })
            .cloned().collect();
        let image_indices: Vec<usize> = if (needle.is_empty() || "image".contains(&needle))
            && matches!(self.clipboard_filter.as_str(), "All Items" | "Favorites" | "Images") {
            self.clipboard_images.iter().enumerate()
                .filter(|(_, image)| self.clipboard_filter != "Favorites" || image.favorite)
                .map(|(index, _)| index).collect()
        } else { Vec::new() };
        let file_indices: Vec<usize> = if matches!(self.clipboard_filter.as_str(),
            "All Items" | "Favorites" | "Files") {
            self.clipboard_files.iter().enumerate()
                .filter(|(_, group)| (self.clipboard_filter != "Favorites" || group.favorite)
                    && (needle.is_empty() || group.paths.iter().any(|path|
                        path.to_string_lossy().to_lowercase().contains(&needle))))
                .map(|(index, _)| index).collect()
        } else { Vec::new() };
        if items.is_empty() && image_indices.is_empty() && file_indices.is_empty() {
            label(ui, if self.clipboard.is_empty() && self.clipboard_images.is_empty()
                && self.clipboard_files.is_empty() { "Copy something to start your history" }
                else { "No matching clipboard items" }, 12.0, MUTED);
            return;
        }
        let mut copy = None;
        let mut favorite = None;
        let mut remove = None;
        let mut copy_image = None;
        let mut favorite_image = None;
        let mut remove_image = None;
        let mut copy_files = None;
        let mut favorite_files = None;
        let mut remove_files = None;
        let (card_width, card_height) = if compact { (126.0, 43.0) } else {
            match self.prefs.clipboard_card_size.as_str() {
                "Small" => (126.0, 42.0),
                "Medium" => (148.0, 50.0),
                _ => (166.0, 58.0),
            }
        };
        egui::ScrollArea::horizontal().id_salt(if compact { "clip_cards_widget" } else { "clip_cards" })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for &index in &file_indices {
                        let group = &self.clipboard_files[index];
                        card().show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_min_width(card_width);
                                ui.set_max_width(card_width);
                                Self::clipboard_card_header(ui,
                                    if group.paths.len() == 1 { "File" } else { "Files" }, card_width,
                                    Color32::from_rgb(121, 92, 196), shelf);
                                ui.add_space(5.0);
                                let names = group.paths.iter().take(3).map(|path|
                                    path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().to_string())
                                    .collect::<Vec<_>>().join("\n");
                                ui.add_sized([ui.available_width(), card_height],
                                    egui::Label::new(RichText::new(names).size(12.0).color(TEXT)).wrap());
                                ui.horizontal(|ui| {
                                    if ui.small_button("Copy").clicked() { copy_files = Some(index); }
                                    if ui.small_button(if group.favorite { "★" } else { "☆" })
                                        .on_hover_text("Toggle favorite").clicked() { favorite_files = Some(index); }
                                    if ui.small_button("×").on_hover_text("Remove from history").clicked() {
                                        remove_files = Some(index);
                                    }
                                });
                            });
                        });
                    }
                    for &index in &image_indices {
                        let image = &self.clipboard_images[index];
                        card().show(ui, |ui| {
                            ui.vertical(|ui| {
                                let width = card_width;
                                ui.set_min_width(width);
                                ui.set_max_width(width);
                                Self::clipboard_card_header(ui, "Image", width,
                                    Color32::from_rgb(55, 154, 175), shelf);
                                ui.add_space(5.0);
                                ui.add(egui::Image::from_texture(&image.texture)
                                    .fit_to_exact_size(Vec2::new(width, card_height)));
                                ui.horizontal(|ui| {
                                    if ui.small_button("Copy").clicked() { copy_image = Some(index); }
                                    if ui.small_button(if image.favorite { "★" } else { "☆" })
                                        .on_hover_text("Toggle favorite").clicked() { favorite_image = Some(index); }
                                    if ui.small_button("×").on_hover_text("Remove from history").clicked() {
                                        remove_image = Some(index);
                                    }
                                });
                            });
                        });
                    }
                    for item in &items {
                        card().show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_min_width(card_width);
                                ui.set_max_width(card_width);
                                let kind = clipboard_kind(item);
                                let tint = match kind {
                                    "Colors" => Color32::from_rgb(205, 84, 103),
                                    "Files" => Color32::from_rgb(121, 92, 196),
                                    "Text" => Color32::from_rgb(62, 105, 202),
                                    _ => Color32::from_rgb(52, 119, 193),
                                };
                                let title = match kind {
                                    "Colors" => "Color", "Links" => "Link", "Files" => "File",
                                    "Text" => "Text", _ => kind,
                                };
                                Self::clipboard_card_header(ui, title, card_width, tint, shelf);
                                ui.add_space(5.0);
                                let preview = item.lines().take(3).collect::<Vec<_>>().join(" ");
                                let preview: String = preview.chars().take(if compact { 54 } else { 86 }).collect();
                                ui.add_sized([ui.available_width(), card_height],
                                    egui::Label::new(RichText::new(preview).size(12.0).color(TEXT)).wrap());
                                ui.horizontal(|ui| {
                                    if ui.small_button("Copy").clicked() { copy = Some(item.clone()); }
                                    let starred = self.clipboard_favorites.contains(item);
                                    if ui.small_button(if starred { "★" } else { "☆" })
                                        .on_hover_text(if starred { "Remove favorite" } else { "Add favorite" }).clicked() {
                                        favorite = Some(item.clone());
                                    }
                                    if ui.small_button("×").on_hover_text("Remove from history").clicked() {
                                        remove = Some(item.clone());
                                    }
                                });
                            });
                        });
                    }
                });
            });
        if let Some(index) = copy_files {
            if let Some(group) = self.clipboard_files.get(index) {
                #[cfg(target_os = "macos")]
                let copied = copy_file_urls(&group.paths);
                #[cfg(not(target_os = "macos"))]
                let copied = false;
                if copied {
                    #[cfg(target_os = "macos")]
                    { self.last_clip_change = unsafe { bookland_clipboard_change_count() }; }
                    self.notify("Files copied to clipboard");
                    if self.clipboard_bottom_sheet && self.prefs.clipboard_hide_after_copy {
                        self.expanded = false;
                    }
                } else { self.notify("Could not copy files"); }
            }
        }
        if let Some(index) = favorite_files {
            if let Some(group) = self.clipboard_files.get_mut(index) {
                group.favorite = !group.favorite;
            }
            if self.prefs.persist_clipboard { self.save_data(); }
        }
        if let Some(index) = remove_files {
            self.clipboard_files.remove(index);
            if self.prefs.persist_clipboard { self.save_data(); }
        }
        if let Some(index) = copy_image {
            if let Some(image) = self.clipboard_images.get(index) {
                let payload = arboard::ImageData {
                    width: image.width, height: image.height,
                    bytes: std::borrow::Cow::Owned(image.rgba.clone()),
                };
                if arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_image(payload)).is_ok() {
                    self.last_image_fingerprint = Some(image.fingerprint);
                    self.notify("Image copied to clipboard");
                    if self.clipboard_bottom_sheet && self.prefs.clipboard_hide_after_copy {
                        self.expanded = false;
                    }
                }
            }
        }
        if let Some(index) = favorite_image {
            if let Some(image) = self.clipboard_images.get_mut(index) { image.favorite = !image.favorite; }
            if self.prefs.persist_clipboard { self.save_data(); }
        }
        if let Some(index) = remove_image {
            let image = self.clipboard_images.remove(index);
            if self.last_image_fingerprint == Some(image.fingerprint) {
                self.last_image_fingerprint = None;
            }
            if self.prefs.persist_clipboard { self.save_data(); }
        }
        if let Some(item) = copy {
            if arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(item.clone())).is_ok() {
                self.last_clip = item.clone();
                self.clipboard.retain(|value| value != &item);
                self.clipboard.insert(0, item);
                trim_clipboard_history(&mut self.clipboard,
                    &self.clipboard_favorites, self.prefs.clipboard_history_size.clamp(1, 1000));
                if self.prefs.persist_clipboard { self.save_data(); }
                self.notify("Copied to clipboard");
                if self.clipboard_bottom_sheet && self.prefs.clipboard_hide_after_copy {
                    self.expanded = false;
                }
            }
        }
        if let Some(item) = favorite {
            if self.clipboard_favorites.contains(&item) {
                self.clipboard_favorites.retain(|value| value != &item);
            } else {
                self.clipboard_favorites.push(item);
            }
            if self.prefs.persist_clipboard { self.save_data(); }
        }
        if let Some(item) = remove {
            self.clipboard.retain(|value| value != &item);
            self.clipboard_favorites.retain(|value| value != &item);
            if self.last_clip == item { self.last_clip.clear(); }
            if self.prefs.persist_clipboard { self.save_data(); }
        }
    }
    fn downloads_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            section_title(ui, "DOWNLOADS");
            if let Some(error) = &self.downloads_error {
                label(ui, &format!("Downloads unavailable: {error}"), 12.0, MUTED);
            } else if self.downloads.is_empty() {
                label(ui, "No recent files", 12.0, MUTED);
            }
            for item in self.downloads.iter().take(6) {
                let name = item.path.file_name()
                    .map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
                if small_button(ui, &format!("↓  {name}")).clicked() {
                    let _ = Command::new("open").arg("-R").arg(&item.path).spawn();
                }
                let detail = if item.active {
                    let speed = item.bytes_per_second
                        .map(|rate| format!(" · {}/s", downloads::format_bytes(rate)))
                        .unwrap_or_default();
                    format!("Downloading · {}{speed}", downloads::format_bytes(item.bytes))
                } else { downloads::format_bytes(item.bytes) };
                label(ui, &detail, 11.0, if item.active { ACCENT } else { MUTED });
            }
        });
    }
    fn terminal_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            section_title(ui, "TERMINAL");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.terminal_draft)
                    .hint_text("Command to run on this Mac").desired_width((ui.available_width() - 70.0).max(140.0)));
                let busy = self.terminal_rx.is_some();
                if ui.add_enabled(!busy, egui::Button::new("Run")).clicked() {
                    let command = self.terminal_draft.trim().to_owned();
                    if !command.is_empty() {
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.terminal_rx = Some(rx);
                        std::thread::spawn(move || {
                            let result = Command::new("/bin/zsh").args(["-lc", &command]).output();
                            let output = match result {
                                Ok(output) => {
                                    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
                                    text.push_str(&String::from_utf8_lossy(&output.stderr));
                                    if text.is_empty() { text = format!("Exited with {}", output.status); }
                                    text.chars().take(4000).collect::<String>()
                                }
                                Err(error) => error.to_string(),
                            };
                            let _ = tx.send(output);
                        });
                    }
                }
            });
            if self.terminal_rx.is_some() { label(ui, "Running…", 12.0, MUTED); }
            if !self.terminal_output.is_empty() {
                ui.add(egui::Label::new(RichText::new(&self.terminal_output)
                    .monospace().size(11.0).color(TEXT)).wrap());
            }
        });
    }
    fn weather_widget(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            section_title(ui, "WEATHER");
            if self.prefs.weather_city.trim().is_empty() {
                label(ui, "Set a city in Nook → Weather settings", 12.0, MUTED);
            } else if !self.weather.error.is_empty() {
                label(ui, &self.weather.error, 12.0, MUTED);
            } else if self.weather.temperature.is_empty() {
                label(ui, "Loading weather…", 12.0, MUTED);
            } else {
                label(ui, &format!("{}  {}", self.weather.place, self.weather.temperature), 22.0, TEXT);
                label(ui, &format!("{} · {}", self.weather.condition, self.weather.humidity), 12.0, MUTED);
            }
        });
    }
    fn prompter_workspace(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            section_title(ui, "TELEPROMPTER");
            let previous_mode = self.prefs.prompter_mode.clone();
            ui.horizontal(|ui| {
                for mode in ["Scroll", "Voice"] {
                    ui.selectable_value(&mut self.prefs.prompter_mode, mode.into(), mode);
                }
            });
            if previous_mode != self.prefs.prompter_mode {
                #[cfg(target_os = "macos")]
                if self.prompter_voice_started { unsafe { bookland_speech_stop(); } }
                self.prompter_voice_started = false;
                self.prompter_playing = false;
                self.save_prefs();
            }
            if self.prefs.prompter_mode == "Voice" {
                #[cfg(target_os = "macos")]
                {
                    let state = unsafe { bookland_speech_state() };
                    ui.horizontal(|ui| {
                        if small_button(ui, if self.prompter_voice_started && (1..=4).contains(&state)
                            { "Stop listening" } else { "Start listening" }).clicked() {
                            if self.prompter_voice_started && (1..=4).contains(&state) {
                                unsafe { bookland_speech_stop(); }
                                self.prompter_voice_started = false;
                            } else if self.prompter.trim().is_empty() {
                                self.notify("Write a script in Settings → Prompter first");
                            } else if let Ok(locale) = std::ffi::CString::new(self.prefs.prompter_language.trim()) {
                                let flags = unsafe { bookland_speech_capabilities(locale.as_ptr()) };
                                if flags & 1 == 0 {
                                    self.notify("Speech is unavailable for this language code");
                                } else if flags & 2 == 0 && !self.prefs.prompter_allow_network {
                                    self.notify("Enable Apple's speech service in Prompter settings for this language");
                                } else {
                                    self.prompter_voice_started = unsafe {
                                        bookland_speech_start(locale.as_ptr(), self.prefs.prompter_allow_network as i32) != 0
                                    };
                                    self.prompter_last_transcript.clear();
                                }
                            } else { self.notify("Invalid speech language code"); }
                        }
                        if small_button(ui, "Restart script").clicked() {
                            self.prompter_voice_word = 0;
                            self.prompter_last_transcript.clear();
                        }
                    });
                    let message = speech_string(bookland_speech_message);
                    label(ui, &message, 11.0, if state == 4 { ACCENT } else { MUTED });
                    let transcript = speech_string(bookland_speech_transcript);
                    let moved = if transcript != self.prompter_last_transcript {
                        let next = prompter::advance(&self.prompter, &transcript, self.prompter_voice_word);
                        let moved = next > self.prompter_voice_word;
                        self.prompter_voice_word = next;
                        self.prompter_last_transcript = transcript.clone();
                        moved
                    } else { false };
                    if !transcript.is_empty() {
                        label(ui, &format!("Heard: {}", transcript.chars().rev().take(140)
                            .collect::<String>().chars().rev().collect::<String>()), 11.0, MUTED);
                    }
                    ui.add_space(10.0);
                    let chunks = prompter::chunks(&self.prompter);
                    egui::ScrollArea::vertical().id_salt("voice_prompter").max_height(320.0)
                        .show(ui, |ui| {
                            if chunks.is_empty() { label(ui, "Write your script in Settings → Prompter.", 16.0, MUTED); }
                            for chunk in &chunks {
                                let active = chunk.start_word <= self.prompter_voice_word
                                    && self.prompter_voice_word < chunk.end_word;
                                let response = ui.label(RichText::new(if chunk.text.is_empty() { " " } else { &chunk.text })
                                    .size(self.prefs.prompter_text_size).color(if active { TEXT } else { MUTED }));
                                if active && moved { ui.scroll_to_rect(response.rect, Some(Align::Center)); }
                                ui.add_space(8.0);
                            }
                        });
                    if (1..=4).contains(&state) { ui.ctx().request_repaint_after(Duration::from_millis(120)); }
                }
                #[cfg(not(target_os = "macos"))]
                label(ui, "Voice following is available on macOS.", 12.0, MUTED);
            } else {
                ui.horizontal(|ui| {
                    if small_button(ui, if self.prompter_playing { "Pause" } else { "Play" }).clicked() {
                        self.prompter_playing = !self.prompter_playing;
                        self.last_prompter_frame = Instant::now();
                    }
                    if small_button(ui, "Restart").clicked() { self.prompter_scroll = 0.0; }
                    ui.add(egui::Slider::new(&mut self.prefs.prompter_speed, 10.0..=120.0).text("Speed"));
                });
                if self.prompter_playing {
                    self.prompter_scroll += self.prefs.prompter_speed * self.last_prompter_frame.elapsed().as_secs_f32().min(0.1);
                    self.last_prompter_frame = Instant::now();
                    ui.ctx().request_repaint();
                }
                ui.add_space(12.0);
                egui::ScrollArea::vertical().max_height(360.0)
                    .vertical_scroll_offset(self.prompter_scroll)
                    .show(ui, |ui| {
                        label(ui, if self.prompter.trim().is_empty() { "Write your script in Settings → Prompter." } else { &self.prompter }, self.prefs.prompter_text_size, TEXT);
                        ui.add_space(300.0);
                    });
            }
        });
    }
    fn nook(&mut self, ui: &mut egui::Ui) {
        if !self.prefs.nook_enabled {
            label(ui, "Nook is disabled in Settings.", 14.0, MUTED);
            return;
        }
        for (i, widget) in self.prefs.widgets.clone().iter().enumerate() {
            if i > 0 {
                ui.add_space(9.0);
                if self.prefs.dividers {
                    ui.separator();
                    ui.add_space(4.0);
                }
            }
            match widget {
                Widget::MediaPlayer => self.media_widget(ui),
                Widget::Pulse => self.pulse_widget(ui),
                Widget::Pomodoro => self.pomodoro_widget(ui),
                Widget::Todos => self.todos_widget(ui),
                Widget::Notes => self.notes_widget(ui),
                Widget::Clipboard => self.clipboard_widget(ui),
                Widget::Downloads => self.downloads_widget(ui),
                Widget::Terminal => self.terminal_widget(ui),
                Widget::Weather => self.weather_widget(ui),
                Widget::Mirror => self.mirror_widget(ui),
            }
        }
    }
    fn playing_next_preview(&self, ui: &mut egui::Ui, compact: bool) {
        if compact { label(ui, "PLAYING NEXT", 10.0, MUTED); }
        let status = if self.media_app.as_deref() != Some("Music") {
            "Playlist preview is available for Apple Music."
        } else if self.media_title == "Nothing playing" {
            "Play a song in Apple Music to see the playlist order."
        } else { &self.queue_preview.status };
        if !status.is_empty() { label(ui, status, if compact { 10.0 } else { 11.0 }, MUTED); }
        for (index, track) in self.queue_preview.tracks.iter().enumerate() {
            ui.horizontal(|ui| {
                label(ui, &format!("{:02}", index + 1), 11.0, ACCENT);
                ui.vertical(|ui| {
                    music_text(ui, &track.title, if compact { 12.0 } else { 13.0 },
                        TEXT, true, true);
                    if !track.artist.is_empty() {
                        music_text(ui, &track.artist, 10.0, MUTED, false, true);
                    }
                });
            });
        }
    }
    fn music(&mut self, ui: &mut egui::Ui) {
        let apple_music = self.media_app.as_deref() == Some("Music");
        card().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(14.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(128.0), egui::Sense::hover());
                album_art(ui, rect, self.artwork_texture.as_ref());
                ui.add_space(15.0);
                music_text(ui, &self.media_title, 20.0, TEXT, false, false);
                music_text(ui, &self.media_artist, 13.0, MUTED, false, false);
                if self.prefs.audio_spectrum {
                    ui.add_space(8.0);
                    self.paint_audio_spectrum(ui, 260.0, 25.0);
                    if self.spectrum_state < 0 {
                        label(ui, "System Audio Recording is unavailable or not allowed.",
                            11.0, MUTED);
                    }
                }
                if !self.lyrics_lines.is_empty()
                    && small_button(ui, if self.lyrics_open { "L · Hide lyrics" } else { "L · Lyrics" })
                        .on_hover_text("Show synced lyrics").clicked() {
                    self.lyrics_open = !self.lyrics_open;
                    if self.lyrics_open {
                        self.queue_open = false;
                        self.airplay_open = false;
                        self.last_media_poll = Instant::now() - Duration::from_secs(6);
                    }
                } else if self.lyrics_for.is_some() && !self.lyrics_status.is_empty() {
                    label(ui, &self.lyrics_status, 11.0, MUTED);
                }
                ui.add_space(10.0);
                self.media_controls(ui);
                if self.media_duration.is_finite() && self.media_duration > 0.0 {
                    ui.add_space(9.0);
                    let displayed = (self.media_position
                        + if self.media_playing { self.last_media_poll.elapsed().as_secs_f32() } else { 0.0 })
                        .clamp(0.0, self.media_duration);
                    let mut seek_to = displayed;
                    let slider = ui.add_sized([380.0, 20.0], egui::Slider::new(
                        &mut seek_to, 0.0..=self.media_duration).show_value(false));
                    if slider.changed() {
                        self.media_position = seek_to;
                        self.last_media_poll = Instant::now();
                    }
                    if self.media_app.is_some()
                        && (slider.drag_stopped() || (slider.changed() && !slider.dragged())) {
                        self.media_action(&format!("set player position to {seek_to}"));
                    }
                    label(ui, &format!("{} / {}",
                        format_time(displayed as u64), format_time(self.media_duration as u64)),
                        11.0, MUTED);
                }
            });
        });
        if apple_music {
        ui.add_space(12.0);
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                section_title(ui, "APPLE MUSIC AIRPLAY");
                if small_button(ui, if self.airplay_open { "Hide" } else { "Choose output" }).clicked() {
                    self.airplay_open = !self.airplay_open;
                    if self.airplay_open {
                        self.queue_open = false;
                        self.lyrics_open = false;
                        self.refresh_music_airplay();
                    }
                }
            });
            if self.airplay_open {
                ui.add_space(8.0);
                if small_button(ui, "Refresh outputs").clicked() { self.refresh_music_airplay(); }
                if !self.airplay_status.is_empty() {
                    label(ui, &self.airplay_status, 12.0, MUTED);
                }
                let devices = self.airplay_devices.clone();
                for device in devices.into_iter().filter(|item| item.supports_audio) {
                    let mut selected = device.selected;
                    let response = ui.add_enabled(device.available && self.airplay_rx.is_none(),
                        egui::Checkbox::new(&mut selected, &device.name));
                    if response.changed() { self.select_music_airplay(device.name, selected); }
                    if !device.available { label(ui, "Unavailable", 11.0, MUTED); }
                }
            }
        });
        ui.add_space(12.0);
        card().show(ui, |ui| {
            if small_button(ui, if self.queue_open { "Hide Playing Next" }
                else { "Playing Next" }).clicked() {
                self.queue_open = !self.queue_open;
                if self.queue_open {
                    self.lyrics_open = false;
                    self.airplay_open = false;
                }
            }
            if self.queue_open {
                ui.add_space(7.0);
                self.playing_next_preview(ui, false);
            }
        });
        }
        if self.lyrics_open && !self.lyrics_lines.is_empty() {
            ui.add_space(12.0);
            card().show(ui, |ui| {
                section_title(ui, "SYNCED LYRICS");
                let position = self.media_position
                    + if self.media_playing { self.last_media_poll.elapsed().as_secs_f32() } else { 0.0 };
                let active = lyrics::active_line(&self.lyrics_lines, position)
                    .unwrap_or(0);
                let start = active.saturating_sub(2);
                let end = (active + 5).min(self.lyrics_lines.len());
                for (index, line) in self.lyrics_lines[start..end].iter().enumerate() {
                    let actual = start + index;
                    let words = if line.text.is_empty() { "♪" } else { &line.text };
                    music_text(ui, words, if actual == active { 17.0 } else { 13.0 },
                        if actual == active { TEXT } else { MUTED }, false, false);
                }
                ui.add_space(5.0);
                label(ui, "Lyrics by LRCLIB", 10.0, MUTED);
            });
        }
        ui.add_space(12.0);
        card().show(ui, |ui| {
            label(
                ui,
                &format!("Output volume  {}%", self.volume.round() as i32),
                12.0,
                MUTED,
            );
            let old = self.volume;
            ui.add(egui::Slider::new(&mut self.volume, 0.0..=100.0).show_value(false));
            if (old - self.volume).abs() > 0.1 {
                set_system_volume(self.volume);
            }
        });
        ui.add_space(12.0);
        card().show(ui, |ui| {
            section_title(ui, "AIRPODS");
            if let Some(name) = &self.airpods.name {
                label(ui, name, 17.0, TEXT);
                let mut levels = Vec::new();
                if let Some(level) = self.airpods.left { levels.push(format!("Left {level}%")); }
                if let Some(level) = self.airpods.right { levels.push(format!("Right {level}%")); }
                if let Some(level) = self.airpods.case { levels.push(format!("Case {level}%")); }
                if levels.is_empty() {
                    if let Some(level) = self.airpods.overall { levels.push(format!("Battery {level}%")); }
                }
                let battery_text = if levels.is_empty() { "Connected · battery level unavailable".into() }
                    else { levels.join(" · ") };
                label(ui, &battery_text, 12.0, MUTED);
            } else if let Some(error) = &self.airpods.error {
                label(ui, error, 12.0, MUTED);
            } else {
                label(ui, "No AirPods connected", 12.0, MUTED);
            }
        });
        ui.add_space(12.0);
        label(
            ui,
            "Playback follows the current Mac media session.",
            12.0,
            MUTED,
        );
    }
    fn calendar(&mut self, ui: &mut egui::Ui) {
        #[cfg(target_os = "macos")]
        let current_month = unsafe { nooklet_calendar_month_index() };
        #[cfg(not(target_os = "macos"))]
        let current_month = 0;
        let key = (current_month, self.calendar_offset);
        if self.calendar_month_key != Some(key) {
            self.calendar_month = load_calendar_month(self.calendar_offset).unwrap_or_else(||
                CalendarMonth { title: "Calendar".into(), grid: "Calendar is unavailable".into(),
                    ..Default::default() });
            self.calendar_month_key = Some(key);
            self.calendar_selected_day = None;
        }
        let month_year = self.calendar_month.title.clone();
        let month = self.calendar_month.clone();
        let day_counts = calendar_day_counts(&self.calendar_events, month.year, month.month);
        if self.calendar_status == 0 {
            let saved_source = !self.prefs.calendar_id.is_empty();
            #[cfg(target_os = "macos")]
            let request_state = unsafe { nooklet_calendar_request_pending() };
            #[cfg(not(target_os = "macos"))]
            let request_state = 0;
            ui.horizontal(|ui| {
                label(ui, if saved_source {
                    "Calendar source saved · macOS access unavailable."
                } else {
                    "Connect Bookland to calendars already on this Mac."
                }, 12.0, MUTED);
                if request_state == 1 {
                    label(ui, "Waiting for macOS permission…", 12.0, MUTED);
                } else if request_state == 2 {
                    label(ui, "macOS has not answered yet.", 12.0, MUTED);
                } else if small_button(ui, if saved_source {
                    "Retry Access"
                } else { "Connect Calendar" }).clicked() {
                    #[cfg(target_os = "macos")]
                    unsafe { nooklet_calendar_connect(); }
                    self.last_calendar_poll = Instant::now() - Duration::from_secs(35);
                }
            });
            if request_state == 1 {
                label(ui, "If no macOS prompt appears, check Bookland's Calendar access in System Settings.", 11.0, MUTED);
            } else if request_state == 2 {
                label(ui, "A rebuilt Bookland may need a fresh Calendar grant. Check Full Access for this build in System Settings, then reopen Bookland.", 11.0, MUTED);
            }
            if small_button(ui, "Open Calendar Permissions").clicked() {
                let _ = Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars")
                    .spawn();
            }
            ui.add_space(8.0);
        }
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, &month_year, 20.0, TEXT);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if small_button(ui, "›").clicked() {
                        self.calendar_offset = (self.calendar_offset + 1).min(24);
                        self.calendar_events.clear();
                        self.calendar_query_pending = true;
                        self.last_calendar_poll = Instant::now() - Duration::from_secs(35);
                    }
                    if small_button(ui, "‹").clicked() {
                        self.calendar_offset = (self.calendar_offset - 1).max(-24);
                        self.calendar_events.clear();
                        self.calendar_query_pending = true;
                        self.last_calendar_poll = Instant::now() - Duration::from_secs(35);
                    }
                });
            });
            ui.add_space(12.0);
            if month.days_in_month == 0 {
                label(ui, "Calendar is unavailable", 13.0, MUTED);
            } else {
                let width = ((ui.available_width() - 5.0 * 6.0) / 7.0).max(32.0);
                let first = month.first_weekday as i32;
                let days = month.days_in_month as i32;
                let weeks = ((first + days + 6) / 7).clamp(4, 6);
                egui::Grid::new("bookland-calendar-days")
                    .spacing([5.0, 3.0])
                    .show(ui, |ui| {
                        for name in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
                            ui.add_sized([width, 16.0], egui::Label::new(
                                RichText::new(name).size(11.0).color(MUTED)).halign(Align::Center));
                        }
                        ui.end_row();
                        for row in 0..weeks {
                            for column in 0..7 {
                                let day = row * 7 + column - first + 1;
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(width, 32.0), egui::Sense::click());
                                if day >= 1 && day <= days {
                                    let day = day as u32;
                                    let selected = self.calendar_selected_day == Some(day);
                                    let today = month.today == day;
                                    if selected || today || response.hovered() {
                                        let fill = if selected {
                                            Color32::from_rgba_unmultiplied(198, 217, 255, 92)
                                        } else if today {
                                            Color32::from_rgba_unmultiplied(198, 217, 255, 40)
                                        } else {
                                            Color32::from_rgba_unmultiplied(255, 255, 255, 16)
                                        };
                                        ui.painter().rect_filled(rect.shrink(1.0), 9.0, fill);
                                    }
                                    ui.painter().text(
                                        rect.center() - Vec2::new(0.0, 3.0), Align2::CENTER_CENTER,
                                        day.to_string(), FontId::proportional(14.0),
                                        if selected || today { TEXT } else { MUTED });
                                    if day_counts[day as usize] != 0 {
                                        ui.painter().circle_filled(
                                            egui::pos2(rect.center().x, rect.bottom() - 5.0),
                                            2.5, ACCENT);
                                    }
                                    response.widget_info(|| egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button, true,
                                        &format!("{} {}, {} event{}", day, month_year,
                                            day_counts[day as usize],
                                            if day_counts[day as usize] == 1 { "" } else { "s" })));
                                    if response.clicked() {
                                        self.calendar_selected_day = if selected { None } else { Some(day) };
                                    }
                                    if day_counts[day as usize] != 0 {
                                        response.on_hover_text(format!("{} event{}", day_counts[day as usize],
                                            if day_counts[day as usize] == 1 { "" } else { "s" }));
                                    }
                                }
                            }
                            ui.end_row();
                        }
                    });
            }
        });
        ui.add_space(12.0);
        if small_button(ui, "Open Calendar").clicked() {
            let _ = Command::new("open").args(["-a", "Calendar"]).spawn();
        }
        if self.calendar_status == 1 {
            ui.add_space(10.0);
            let previous_id = self.prefs.calendar_id.clone();
            let selected = self.calendar_choices.iter()
                .find(|choice| choice.id == self.prefs.calendar_id);
            let selected_label = selected.map(|choice|
                format!("{} · {}", choice.source, choice.title))
                .unwrap_or_else(|| if self.prefs.calendar_id.is_empty() {
                    "All Mac calendars".into()
                } else { "Selected calendar unavailable".into() });
            card().show(ui, |ui| {
                section_title(ui, "CALENDAR SOURCE");
                egui::ComboBox::from_id_salt("calendar-source")
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.prefs.calendar_id,
                            String::new(), "All Mac calendars");
                        for choice in &self.calendar_choices {
                            let label = format!("{} · {}{}", choice.source,
                                choice.title, if choice.writable { "" } else { " (read-only)" });
                            ui.selectable_value(&mut self.prefs.calendar_id,
                                choice.id.clone(), label);
                        }
                    });
                label(ui, "Google calendars added to macOS Calendar appear here.", 11.0, MUTED);
                if self.calendar_choices.is_empty() {
                    label(ui, if self.calendar_query_pending { "Loading calendars…" }
                        else { "No synced calendars found. Add your Google account in macOS Calendar." },
                        11.0, MUTED);
                }
            });
            if self.prefs.calendar_id != previous_id {
                self.calendar_events.clear();
                self.calendar_query_pending = true;
                self.last_calendar_poll = Instant::now() - Duration::from_secs(35);
            }
        }
        ui.add_space(12.0);
        card().show(ui, |ui| {
            let selected_key = self.calendar_selected_day.map(|day|
                format!("{:04}-{:02}-{:02}", month.year, month.month, day));
            let heading = self.calendar_selected_day.map(|day|
                format!("EVENTS · {} {}", day, month_year))
                .unwrap_or_else(|| "UPCOMING EVENTS".into());
            section_title(ui, &heading);
            match self.calendar_status {
                1 => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|time| time.as_secs_f64()).unwrap_or(0.0);
                    let visible: Vec<_> = self.calendar_events.iter()
                        .filter(|event| selected_key.as_ref().map_or(
                            event.end >= now && event.start <= now + 86400.0 * 45.0,
                            |key| event.occurs_on_day(key)))
                        .take(8).collect();
                    if self.calendar_query_pending && visible.is_empty() {
                        label(ui, "Loading events…", 13.0, MUTED);
                    } else if visible.is_empty() {
                        label(ui, if selected_key.is_some() { "No events on this day" }
                            else { "No events in the next 45 days" }, 13.0, MUTED);
                    }
                    for event in visible {
                        ui.horizontal(|ui| {
                            label(ui, &event.display_time, 12.0, MUTED);
                            ui.add_space(9.0);
                            music_text(ui, &event.title, 13.0, TEXT, false, true);
                        });
                        label(ui, &event.calendar, 11.0, MUTED);
                        ui.add_space(8.0);
                    }
                }
                2 => {
                    label(ui, "Calendar access is off for Bookland.", 12.0, MUTED);
                    if small_button(ui, "Open Calendar Permissions").clicked() {
                        let _ = Command::new("open")
                            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars")
                            .spawn();
                    }
                }
                _ => {
                    label(ui, if self.prefs.calendar_id.is_empty() {
                        "Connect Calendar above to show events from your Mac calendars, including Google calendars already added to macOS."
                    } else {
                        "Your calendar selection is saved. Restore macOS access above to show its events."
                    }, 12.0, MUTED);
                }
            }
        });
        if self.calendar_status == 1 {
            ui.add_space(10.0);
            let selected_writable = !self.prefs.calendar_id.is_empty()
                && self.calendar_choices.iter().find(|choice|
                    choice.id == self.prefs.calendar_id).is_some_and(|choice| choice.writable);
            card().show(ui, |ui| {
                section_title(ui, "ADD EVENT");
                if !selected_writable {
                    label(ui, "Choose a writable Google calendar to add events.", 11.0, MUTED);
                }
                ui.add(egui::TextEdit::singleline(&mut self.new_event_title).hint_text("Event title"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.new_event_date).desired_width(110.0).hint_text("YYYY-MM-DD"));
                    ui.add(egui::TextEdit::singleline(&mut self.new_event_time).desired_width(70.0).hint_text("HH:MM"));
                    ui.add(egui::DragValue::new(&mut self.new_event_minutes).range(5..=1440).suffix(" min"));
                });
                if ui.add_enabled(selected_writable, egui::Button::new("Add to Calendar")).clicked() {
                    let when = format!("{} {}", self.new_event_date.trim(), self.new_event_time.trim());
                    let start = run_output("date", &["-j", "-f", "%Y-%m-%d %H:%M", &when, "+%s"])
                        .and_then(|value| value.parse::<f64>().ok());
                    #[cfg(target_os = "macos")]
                    let selected_id = CString::new(self.prefs.calendar_id.as_str()).ok();
                    #[cfg(target_os = "macos")]
                    let saved = start.and_then(|start| CString::new(self.new_event_title.trim()).ok()
                        .map(|title| unsafe { nooklet_calendar_add(title.as_ptr(), start,
                            self.new_event_minutes as f64 * 60.0,
                            selected_id.as_ref().map_or(std::ptr::null(), |value| value.as_ptr())) != 0 }))
                        .unwrap_or(false);
                    #[cfg(not(target_os = "macos"))]
                    let saved = false;
                    if saved {
                        self.new_event_title.clear();
                        self.notify("Event added to Calendar");
                        self.last_calendar_poll = Instant::now() - Duration::from_secs(35);
                    } else {
                        self.notify("Check the title, date, time and Calendar access");
                    }
                }
            });
        }
    }
    fn video_downloader_ui(&mut self, ui: &mut egui::Ui) {
        let mut start = None;
        if let Some(url) = self.video_offer.clone() {
            card().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        label(ui, "Video link detected", 14.0, TEXT);
                        label(ui, "YouTube", 11.0, MUTED);
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Dismiss").clicked() { self.video_offer = None; }
                        if small_button(ui, "Download").clicked() { start = Some(url.clone()); }
                    });
                });
            });
            ui.add_space(10.0);
        }
        card().show(ui, |ui| {
            section_title(ui, "VIDEO DOWNLOADS");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.video_url_draft)
                    .hint_text("Paste a YouTube video link")
                    .desired_width((ui.available_width() - 95.0).max(150.0)));
                if ui.add_enabled(self.video_rx.is_none(), egui::Button::new("Download")).clicked() {
                    start = Some(self.video_url_draft.clone());
                }
            });
            if self.video_rx.is_some() {
                ui.add_space(7.0);
                let fraction = self.video_progress.percent.unwrap_or(0.0) / 100.0;
                ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0))
                    .text(self.video_progress.percent.map(|value| format!("{value:.0}%"))
                        .unwrap_or_else(|| "Downloading".into())));
                let mut detail = self.video_progress.downloaded
                    .map(downloads::format_bytes).unwrap_or_else(|| "Preparing…".into());
                if let Some(total) = self.video_progress.total {
                    detail.push_str(&format!(" of {}", downloads::format_bytes(total)));
                }
                if let Some(speed) = self.video_progress.speed {
                    detail.push_str(&format!(" · {}/s", downloads::format_bytes(speed)));
                }
                if let Some(eta) = self.video_progress.eta {
                    detail.push_str(&format!(" · {eta}s left"));
                }
                label(ui, &detail, 11.0, MUTED);
                if ui.add_enabled(self.video_pid.is_some(), egui::Button::new("Cancel download")).clicked() {
                    if let Some(pid) = self.video_pid {
                        let _ = Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
                        self.video_stopping = true;
                        self.video_status = "Stopping download…".into();
                    }
                }
            }
            if !self.video_status.is_empty() {
                label(ui, &self.video_status, 11.0, if self.video_status.starts_with("Download failed") { Color32::LIGHT_RED } else { MUTED });
            }
        });
        if let Some(url) = start { self.start_video_download(&url); }
    }
    fn pipeline_editor(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "POCKET PIPELINES");
        card().show(ui, |ui| {
            label(ui, "Save a sequence of native file conversions and run it from Pocket. Share it as a .booklandpipeline file.", 12.0, MUTED);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                label(ui, "Name", 12.0, TEXT);
                ui.add(egui::TextEdit::singleline(&mut self.pipeline_draft_name)
                    .hint_text("Photos to WebP").desired_width(220.0));
            });
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("pipeline_draft_action")
                    .selected_text(if self.pipeline_draft_action == pocket::Action::Zip {
                        "ZIP"
                    } else {
                        pipelines::CONVERSIONS.iter()
                            .find(|(action, _)| *action == self.pipeline_draft_action)
                            .map(|(_, title)| *title).unwrap_or("Choose format")
                    })
                    .show_ui(ui, |ui| {
                        for (action, title) in pipelines::CONVERSIONS {
                            ui.selectable_value(&mut self.pipeline_draft_action,
                                action, title);
                        }
                        ui.selectable_value(&mut self.pipeline_draft_action,
                            pocket::Action::Zip, "ZIP");
                    });
                if ui.add_enabled(pipelines::can_append(&self.pipeline_draft_steps,
                    self.pipeline_draft_action), egui::Button::new("Add step")).clicked() {
                    self.pipeline_draft_steps.push(self.pipeline_draft_action);
                }
                if ui.add_enabled(!self.pipeline_draft_steps.is_empty(),
                    egui::Button::new("Remove last")).clicked() {
                    self.pipeline_draft_steps.pop();
                }
            });
            if !self.pipeline_draft_steps.is_empty() {
                let draft = pipelines::Pipeline { name: self.pipeline_draft_name.clone(),
                    steps: self.pipeline_draft_steps.clone() };
                label(ui, &format!("Steps: {}", draft.summary()), 12.0, TEXT);
            }
            if ui.button("Save pipeline").clicked() {
                let pipeline = pipelines::Pipeline {
                    name: self.pipeline_draft_name.trim().into(),
                    steps: self.pipeline_draft_steps.clone(),
                };
                match pipeline.validate() {
                    Err(error) => self.notify(error),
                    Ok(()) => {
                        if let Some(existing) = self.pipelines.iter_mut().find(|item|
                            item.name.eq_ignore_ascii_case(&pipeline.name)) {
                            *existing = pipeline.clone();
                        } else if self.pipelines.len() < 20 {
                            self.pipelines.push(pipeline.clone());
                        } else {
                            self.notify("Remove a pipeline before saving another");
                            return;
                        }
                        self.pipeline_draft_name.clear();
                        self.pipeline_draft_steps.clear();
                        self.save_data();
                        self.notify(format!("Saved pipeline: {}", pipeline.name));
                    }
                }
            }
            if ui.button("Import pipeline…").clicked() {
                #[cfg(target_os = "macos")]
                {
                    let mut buffer = vec![0_u8; 8192];
                    let length = unsafe { bookland_choose_pocket_file(
                        buffer.as_mut_ptr().cast(), buffer.len()) };
                    if length > 0 {
                        self.install_pipeline(&PathBuf::from(
                            String::from_utf8_lossy(&buffer[..length]).to_string()));
                    }
                }
            }
        });
        for pipeline in self.pipelines.clone() {
            ui.add_space(5.0);
            card().show(ui, |ui| {
                label(ui, &pipeline.name, 14.0, TEXT);
                label(ui, &pipeline.summary(), 11.0, MUTED);
                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        if let Some(root) = preferences_path().and_then(|path| path.parent()
                            .map(|parent| parent.join("Pocket"))) {
                            match pipelines::export(&pipeline, &root) {
                                Ok(path) => {
                                    let _ = Command::new("open").arg("-R").arg(&path).spawn();
                                    self.notify(format!("Exported {}", path.display()));
                                }
                                Err(error) => self.notify(format!("Export failed: {error}")),
                            }
                        }
                    }
                    if ui.button("Remove").clicked() {
                        self.pipelines.retain(|item| item.name != pipeline.name);
                        self.save_data();
                        self.notify(format!("Removed pipeline: {}", pipeline.name));
                    }
                });
            });
        }
    }
    fn tray(&mut self, ui: &mut egui::Ui) {
        if !self.prefs.show_extra_workspaces && small_button(ui, "Clipboard history").clicked() {
            self.workspace = Workspace::Clipboard;
            self.clipboard_bottom_sheet = false;
            self.clipboard_focus_pending = self.prefs.clipboard_focus_search;
        }
        self.video_downloader_ui(ui);
        ui.add_space(13.0);
        card().show(ui, |ui| {
            label(ui, "Pocket", 18.0, TEXT);
            label(ui, "Drop a file on the island, or choose one here. Keep a copy, create a ZIP, convert media, or share with AirDrop.", 12.0, MUTED);
            ui.horizontal(|ui| {
                if ui.button("Add file or folder").clicked() {
                    #[cfg(target_os = "macos")]
                    {
                        let mut buffer = vec![0_u8; 8192];
                        let length = unsafe { bookland_choose_pocket_file(
                            buffer.as_mut_ptr().cast(), buffer.len()) };
                        if length > 0 {
                            self.add_pocket_file(PathBuf::from(
                                String::from_utf8_lossy(&buffer[..length]).to_string()));
                        }
                    }
                }
                if ui.button("Open Pocket folder").clicked() {
                    if let Some(root) = preferences_path().and_then(|path| path.parent()
                        .map(|parent| parent.join("Pocket"))) {
                        let _ = std::fs::create_dir_all(&root);
                        let _ = Command::new("open").arg(root).spawn();
                    }
                }
                if ui.button("Float Pocket").clicked() {
                    self.open_pocket_near_pointer(ui.ctx());
                }
            });
            if !self.pipelines.is_empty() {
                ui.add_space(6.0);
                label(ui, &format!("{} saved pipelines", self.pipelines.len()), 11.0, MUTED);
            }
            if ui.button(if self.pipeline_editor_open {
                "Hide pipelines"
            } else { "Manage pipelines" }).clicked() {
                self.pipeline_editor_open = !self.pipeline_editor_open;
            }
        });
        if self.pipeline_editor_open {
            ui.add_space(12.0);
            self.pipeline_editor(ui);
        }
        ui.add_space(13.0);
        section_title(ui, "RECENT FILES · POCKET");
        if !self.pocket_status.is_empty() { label(ui, &self.pocket_status, 12.0, MUTED); }
        if self.files.is_empty() {
            label(ui, "No files in Pocket yet", 13.0, MUTED);
        }
        let root = preferences_path().and_then(|path| path.parent()
            .map(|parent| parent.join("Pocket")));
        for path in self.files.clone() {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let kept = root.as_ref().is_some_and(|root| pocket::is_kept(&path, root));
            card().show(ui, |ui| {
                label(ui, &format!("▣  {name}"), 14.0, TEXT);
                label(ui, if !path.exists() { "File missing" } else if kept { "Kept in Bookland Pocket" }
                    else { "Recent file · source stays in place" }, 11.0, MUTED);
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(path.exists(), egui::Button::new("Reveal")).clicked() {
                        let _ = Command::new("open").arg("-R").arg(&path).spawn();
                    }
                    if ui.add_enabled(path.exists() && !kept && self.pocket_job.is_none(),
                        egui::Button::new("Keep copy")).clicked() {
                        self.start_pocket_action(path.clone(), pocket::Action::Keep);
                    }
                    if pocket::can_convert(&path) {
                        ui.menu_button("Convert…", |ui| {
                            for (label, action) in [("PNG", pocket::Action::Png),
                                ("JPEG", pocket::Action::Jpeg),
                                ("HEIC", pocket::Action::Heic),
                                ("TIFF", pocket::Action::Tiff),
                                ("WebP (lossless)", pocket::Action::Webp),
                                ("AIFF", pocket::Action::Aiff),
                                ("ALAC (.m4a)", pocket::Action::Alac),
                                ("FLAC", pocket::Action::Flac),
                                ("DOCX", pocket::Action::Docx),
                                ("HTML", pocket::Action::Html)] {
                                if pocket::can_convert_to(&path, action) &&
                                    ui.add_enabled(self.pocket_job.is_none(),
                                        egui::Button::new(label)).clicked() {
                                    self.start_pocket_action(path.clone(), action);
                                    ui.close();
                                }
                            }
                            for pipeline in self.pipelines.clone() {
                                if pipeline.can_run(&path) && ui.add_enabled(
                                    self.pocket_job.is_none(), egui::Button::new(
                                        format!("{} · {}", pipeline.name, pipeline.summary())))
                                    .clicked() {
                                    self.start_pocket_pipeline(path.clone(), pipeline);
                                    ui.close();
                                }
                            }
                        });
                    }
                    if ui.add_enabled(path.exists() && self.pocket_job.is_none(),
                        egui::Button::new("Create ZIP")).clicked() {
                        self.start_pocket_action(path.clone(), pocket::Action::Zip);
                    }
                    if ui.add_enabled(path.exists(), egui::Button::new("AirDrop")).clicked() {
                        #[cfg(target_os = "macos")]
                        {
                            let opened = CString::new(path.to_string_lossy().as_bytes())
                                .ok().is_some_and(|name| unsafe { bookland_airdrop_file(name.as_ptr()) });
                            self.notify(if opened { "Choose an AirDrop recipient in macOS" }
                                else { "AirDrop is unavailable for this file" });
                        }
                    }
                    if ui.button("Forget").clicked() {
                        self.files.retain(|item| item != &path);
                        if self.files.is_empty() { self.pocket_status.clear(); }
                        self.pocket_activity = None;
                        self.save_data();
                    }
                });
            });
            ui.add_space(5.0);
        }
    }
    fn floating_pocket_ui(&mut self, ctx: &egui::Context) {
        for file in ctx.input(|input| input.raw.dropped_files.clone()) {
            if let Some(path) = file.path { self.remember_pocket_file(path, false); }
        }
        ctx.set_visuals(egui::Visuals::dark());
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Color32::TRANSPARENT).inner_margin(8))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgba_premultiplied(18, 20, 27, 240))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(94, 101, 121)))
                    .corner_radius(22)
                    .inner_margin(18)
                    .show(ui, |ui| {
                        ui.set_width(390.0);
                        ui.horizontal(|ui| {
                            label(ui, "Pocket", 21.0, TEXT);
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("×").clicked() {
                                    self.pocket_floating = false;
                                    ctx.send_viewport_cmd(ViewportCommand::Close);
                                }
                            });
                        });
                        label(ui, "Drop a file here to keep, ZIP, convert, or share it.", 12.0, MUTED);
                        ui.add_space(12.0);
                        if let Some(path) = self.files.first().cloned() {
                            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("File");
                            let root = preferences_path().and_then(|path| path.parent()
                                .map(|parent| parent.join("Pocket")));
                            let kept = root.as_ref().is_some_and(|root| pocket::is_kept(&path, root));
                            label(ui, &format!("▣  {name}"), 15.0, TEXT);
                            label(ui, if kept { "Kept in Bookland Pocket" } else { "Recent file" },
                                11.0, MUTED);
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui.add_enabled(path.exists() && !kept && self.pocket_job.is_none(),
                                    egui::Button::new("Keep copy")).clicked() {
                                    self.start_pocket_action(path.clone(), pocket::Action::Keep);
                                }
                                if pocket::can_convert(&path) {
                                    ui.menu_button("Convert…", |ui| {
                                        for (label, action) in [("PNG", pocket::Action::Png),
                                            ("JPEG", pocket::Action::Jpeg),
                                            ("HEIC", pocket::Action::Heic),
                                            ("TIFF", pocket::Action::Tiff),
                                            ("WebP (lossless)", pocket::Action::Webp),
                                            ("AIFF", pocket::Action::Aiff),
                                            ("ALAC (.m4a)", pocket::Action::Alac),
                                            ("FLAC", pocket::Action::Flac),
                                            ("DOCX", pocket::Action::Docx),
                                            ("HTML", pocket::Action::Html)] {
                                            if pocket::can_convert_to(&path, action) &&
                                                ui.add_enabled(self.pocket_job.is_none(),
                                                    egui::Button::new(label)).clicked() {
                                                self.start_pocket_action(path.clone(), action);
                                                ui.close();
                                            }
                                        }
                                        for pipeline in self.pipelines.clone() {
                                            if pipeline.can_run(&path) && ui.add_enabled(
                                                self.pocket_job.is_none(), egui::Button::new(
                                                    format!("{} · {}", pipeline.name,
                                                        pipeline.summary()))).clicked() {
                                                self.start_pocket_pipeline(path.clone(), pipeline);
                                                ui.close();
                                            }
                                        }
                                    });
                                }
                                if ui.add_enabled(path.exists() && self.pocket_job.is_none(),
                                    egui::Button::new("Create ZIP")).clicked() {
                                    self.start_pocket_action(path.clone(), pocket::Action::Zip);
                                }
                                if ui.add_enabled(path.exists(), egui::Button::new("AirDrop")).clicked() {
                                    #[cfg(target_os = "macos")]
                                    {
                                        let opened = CString::new(path.to_string_lossy().as_bytes())
                                            .ok().is_some_and(|name| unsafe { bookland_airdrop_file(name.as_ptr()) });
                                        self.notify(if opened { "Choose an AirDrop recipient in macOS" }
                                            else { "AirDrop is unavailable for this file" });
                                    }
                                }
                                if ui.button("Forget").clicked() {
                                    self.files.retain(|item| item != &path);
                                    if self.files.is_empty() { self.pocket_status.clear(); }
                                    self.pocket_activity = None;
                                    self.save_data();
                                }
                            });
                        } else {
                            Frame::new().fill(Color32::from_rgba_premultiplied(39, 43, 55, 180))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(87, 94, 112)))
                                .corner_radius(14).inner_margin(18)
                                .show(ui, |ui| {
                                    ui.set_min_width(350.0);
                                    ui.vertical_centered(|ui| label(ui, "Drop file or folder", 15.0, ACCENT));
                                });
                        }
                        if !self.pocket_status.is_empty() {
                            ui.add_space(8.0);
                            label(ui, &self.pocket_status, 11.0, MUTED);
                        }
                        ui.add_space(9.0);
                        if ui.button("Open all in Tray  ↗").clicked() {
                            self.workspace = Workspace::Tray;
                            self.expanded = true;
                            self.pocket_floating = false;
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                    });
            });
    }
    fn codex_chat(&mut self, ui: &mut egui::Ui) {
        let empty_chat = self.codex_messages.is_empty();
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "Codex", 19.0, TEXT);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    label(ui, if self.prefs.codex_allow_edits { "PROJECT EDITS" } else { "READ ONLY" },
                        10.0, ACCENT);
                });
            });
            ui.add_space(if empty_chat { 3.0 } else { 7.0 });
            if !empty_chat {
                egui::ScrollArea::vertical().id_salt("bookland_codex_chat")
                    .max_height(75.0).stick_to_bottom(true).show(ui, |ui| {
                        for message in &self.codex_messages {
                            let color = if message.role == "Codex" { ACCENT } else { MUTED };
                            label(ui, &message.role, 11.0, color);
                            ui.label(RichText::new(&message.text).size(13.0).color(TEXT));
                            ui.add_space(8.0);
                        }
                    });
            }
            if !self.codex_status.is_empty() {
                ui.add_space(4.0);
                label(ui, &self.codex_status, 11.0, MUTED);
            }
            ui.add_space(7.0);
            let input = ui.add(egui::TextEdit::multiline(&mut self.codex_draft)
                .id_salt("bookland_codex_prompt")
                .hint_text("Message Codex…")
                .desired_rows(if empty_chat { 1 } else { 2 }).desired_width(f32::INFINITY));
            let shortcut = input.has_focus() && ui.input(|i|
                i.modifiers.command && i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                let ready = self.codex_rx.is_none() && !self.codex_draft.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new("Send  ↗")).clicked() || (ready && shortcut) {
                    self.send_codex();
                }
                if self.codex_rx.is_some() {
                    if ui.add_enabled(self.codex_pid.is_some() && !self.codex_stopping,
                        egui::Button::new("Stop")).clicked() {
                        if let Some(pid) = self.codex_pid {
                            match codex_bridge::stop(pid) {
                                Ok(()) => { self.codex_stopping = true; self.codex_status = "Stopping Codex…".into(); }
                                Err(error) => self.codex_status = error,
                            }
                        }
                    }
                } else if ui.button("New chat").clicked() {
                    self.codex_session_id = None;
                    self.codex_messages.clear();
                    self.codex_status.clear();
                    self.save_data();
                }
                if ui.button("Open Codex app").clicked() {
                    let fallback = if self.codex_status.starts_with("Codex:")
                        || self.codex_status.starts_with("Codex connection") {
                        self.codex_messages.iter().rev()
                            .find(|message| message.role == "You")
                            .map(|message| message.text.clone())
                    } else { None };
                    let to_copy = if self.codex_draft.trim().is_empty() {
                        fallback
                    } else { Some(self.codex_draft.clone()) };
                    let copied = to_copy.is_some_and(|text|
                        arboard::Clipboard::new().and_then(|mut clipboard|
                            clipboard.set_text(text)).is_ok());
                    let _ = Command::new("open").args(["-b", "com.openai.codex"]).spawn();
                    self.notify(if copied { "Codex opened; paste the copied draft" }
                        else { "Codex opened" });
                }
                if !self.prefs.show_extra_workspaces && ui.button("Prompt builder").clicked() {
                    self.workspace = Workspace::Prompts;
                }
                if ui.button("Activity").clicked() {
                    self.agent_chat_open = false;
                }
            });
            if !empty_chat {
                label(ui, "⌘ Return to send · Uses the Codex CLI on this Mac", 10.0, MUTED);
            }
        });
    }
    fn sheet_prompt_builder(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            label(ui, "Sheet prompt builder", 21.0, TEXT);
            label(ui, "Describe the task. Bookland will write a prompt for a spreadsheet chat.", 12.0, MUTED);
            ui.add_space(9.0);
            label(ui, "What should happen?", 12.0, ACCENT);
            let mut changed = ui.add(egui::TextEdit::multiline(&mut self.sheet_goal)
                .hint_text("Example: find duplicate leads and mark the newest row")
                .desired_rows(2).desired_width(f32::INFINITY)).changed();
            label(ui, "Data or columns", 12.0, ACCENT);
            changed |= ui.add(egui::TextEdit::singleline(&mut self.sheet_source)
                .hint_text("Example: Leads tab, columns A–H")
                .desired_width(f32::INFINITY)).changed();
            label(ui, "Desired result", 12.0, ACCENT);
            changed |= ui.add(egui::TextEdit::singleline(&mut self.sheet_result)
                .hint_text("Example: a clean tab with duplicates highlighted")
                .desired_width(f32::INFINITY)).changed();
            label(ui, "Constraints", 12.0, ACCENT);
            changed |= ui.add(egui::TextEdit::singleline(&mut self.sheet_constraints)
                .hint_text("Example: keep original rows and existing formulas")
                .desired_width(f32::INFINITY)).changed();
            if changed { self.save_data(); }
            ui.add_space(8.0);
            if ui.add_enabled(!self.sheet_goal.trim().is_empty(),
                egui::Button::new("Build prompt")).clicked() {
                self.sheet_prompt = compose_sheet_prompt(&self.sheet_goal, &self.sheet_source,
                    &self.sheet_result, &self.sheet_constraints);
                self.save_data();
            }
        });
        if !self.sheet_prompt.is_empty() {
            ui.add_space(9.0);
            card().show(ui, |ui| {
                section_title(ui, "READY PROMPT · EDIT BEFORE USING");
                if ui.add(egui::TextEdit::multiline(&mut self.sheet_prompt)
                    .desired_rows(7).desired_width(f32::INFINITY)).changed() {
                    self.save_data();
                }
                ui.horizontal(|ui| {
                    if ui.button("Copy prompt").clicked() {
                        match arboard::Clipboard::new().and_then(|mut clipboard|
                            clipboard.set_text(self.sheet_prompt.clone())) {
                            Ok(()) => self.notify("Prompt copied"),
                            Err(_) => self.notify("Could not copy prompt"),
                        }
                    }
                    if ui.button("Use in Codex").clicked() {
                        self.codex_draft = self.sheet_prompt.clone();
                        self.workspace = Workspace::Agents;
                        self.agent_chat_desktop = false;
                        self.agent_chat_open = true;
                    }
                });
            });
        }
    }
    fn agents(&mut self, ui: &mut egui::Ui) {
        self.agent_completion = None;
        self.codex_chat(ui);
        ui.add_space(9.0);
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "Agents", 21.0, TEXT);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    label(ui, "LOCAL ACTIVITY", 10.0, ACCENT);
                });
            });
            ui.add_space(8.0);
            if !self.prefs.agent_monitor {
                label(ui, "Agent monitoring is off in Settings.", 13.0, MUTED);
            } else if self.agents.is_empty() {
                label(ui, "No local Codex, Claude Code, or Craft activity found", 14.0, TEXT);
            } else {
                ui.horizontal_wrapped(|ui| {
                    for (index, agent) in self.agents.iter().enumerate() {
                        let selected = index == self.selected_agent;
                        let indicator = match agent.source.as_str() {
                            "Codex" => Color32::from_rgb(91, 145, 248),
                            "Claude Code" => Color32::from_rgb(219, 150, 112),
                            _ => Color32::from_rgb(163, 135, 223),
                        };
                        let response = ui.add(egui::Button::new("")
                            .fill(if selected { CARD_2 } else { Color32::TRANSPARENT })
                            .corner_radius(13)
                            .min_size(Vec2::splat(42.0)))
                            .on_hover_text(format!("{} · {}", agent.source, agent.title));
                        ui.painter().circle_stroke(response.rect.center(), 8.0,
                            Stroke::new(1.5, indicator));
                        ui.painter().circle_filled(response.rect.center(), 3.5, indicator);
                        if response.clicked() {
                            self.selected_agent = index;
                        }
                    }
                });
                ui.add_space(9.0);
                let agent = &self.agents[self.selected_agent];
                label(ui, &format!("{} · {}", agent.source, agent.status), 13.0, ACCENT);
                label(ui, &agent.title, 17.0, TEXT);
                if let Some(usage) = &agent.usage {
                    ui.add_space(8.0);
                    section_title(ui, "AI USAGE · THIS SESSION");
                    label(ui, &format!("Input {}  ·  Output {}  ·  Cached {}",
                        compact_tokens(usage.input_tokens),
                        compact_tokens(usage.output_tokens),
                        compact_tokens(usage.cached_input_tokens)), 12.0, TEXT);
                    if let Some(percent) = usage.limit_used_percent {
                        let window = match usage.limit_window_minutes {
                            Some(10_080) => "7-day",
                            Some(300) => "5-hour",
                            _ => "Account",
                        };
                        label(ui, &format!("{window} limit used · {percent:.0}%"), 11.0, MUTED);
                        ui.add(egui::ProgressBar::new(percent / 100.0)
                            .desired_width(ui.available_width()));
                    }
                }
            }
        });
        ui.add_space(9.0);
        card().show(ui, |ui| {
            section_title(ui, "PROGRESS");
            if let Some(agent) = self.agents.get(self.selected_agent) {
                if agent.events.is_empty() {
                    label(ui, "Waiting for the next activity update", 13.0, MUTED);
                }
                for line in &agent.events {
                    label(ui, &format!("●  {line}"), 12.0, TEXT);
                    ui.add_space(6.0);
                }
            } else {
                label(ui, "No activity to show", 13.0, MUTED);
            }
            ui.add_space(18.0);
        });
        ui.add_space(9.0);
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(103, 105, 113, 165))
            .corner_radius(16)
            .inner_margin(16)
            .show(ui, |ui| {
                label(ui, "Summary", 13.0, TEXT);
                label(
                    ui,
                    "Reads recent Codex, Claude Code, and Craft Anything task records on this Mac.",
                    12.0,
                    MUTED,
                );
            });
        ui.add_space(9.0);
        ui.horizontal(|ui| {
            label(ui, &format!("◷  {}", format_time(self.remaining())), 12.0, MUTED);
            if let Some(agent) = self.agents.get(self.selected_agent) {
                label(ui, &format!("•  {}", agent.status), 12.0, MUTED);
                if small_button(ui, "Open source").clicked() {
                    if agent.source == "Codex" {
                        let _ = Command::new("open").args(["-b", "com.openai.codex"]).spawn();
                    } else {
                        let _ = Command::new("open").arg(&agent.path).spawn();
                    }
                }
            }
        });
    }
    fn glass_nook(&mut self, ui: &mut egui::Ui, card_height: f32) {
        let frame = || Frame::new()
            .fill(Color32::from_rgba_unmultiplied(19, 23, 30, 100))
            .corner_radius(23)
            .stroke(Stroke::new(0.9, Color32::from_rgba_unmultiplied(235, 241, 249, 72)))
            .shadow(egui::epaint::Shadow { offset: [0, 3], blur: 15, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 95) })
            .inner_margin(Margin::same(15));
        ui.columns(4, |cols| {
            let mut media_sheen = None;
            let media = frame().show(&mut cols[0], |ui| {
                media_sheen = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_width(ui.available_width());
                ui.set_min_height(card_height);
                glass_card_title(ui, "Media", 0);
                ui.add_space(15.0);
                ui.horizontal(|ui| {
                    let (art, _) = ui.allocate_exact_size(Vec2::splat(72.0), egui::Sense::hover());
                    album_art(ui, art, self.artwork_texture.as_ref());
                    ui.vertical(|ui| {
                        ui.add_space(12.0);
                        if self.media_title == "Nothing playing" {
                            ui.painter().rect_filled(egui::Rect::from_min_size(ui.cursor().min, Vec2::new(91.0, 8.0)), 4.0,
                                Color32::from_rgba_unmultiplied(240, 241, 255, 110));
                            ui.add_space(14.0);
                            ui.painter().rect_filled(egui::Rect::from_min_size(ui.cursor().min, Vec2::new(68.0, 8.0)), 4.0,
                                Color32::from_rgba_unmultiplied(240, 241, 255, 65));
                            ui.add_space(12.0);
                        } else {
                            music_text(ui, &self.media_title, 12.0, TEXT, false, false);
                            music_text(ui, &self.media_artist, 10.0, MUTED, false, false);
                        }
                    });
                });
                ui.add_space(15.0);
                ui.horizontal(|ui| {
                    ui.add_space(28.0);
                    let browser = is_browser_media_app(self.media_app.as_deref());
                    if ui.add(egui::Button::new(RichText::new(if browser { "−15" } else { "◀◀" }).size(17.0)).frame(false))
                        .on_hover_text(if browser { "Back 15 seconds" } else { "Previous track" }).clicked() {
                        if browser { self.seek_browser_media(true); } else { self.skip_media(true); }
                    }
                    if ui.add(egui::Button::new(RichText::new(
                        if self.media_playing { "Ⅱ" } else { "▶" }).size(17.0))
                        .fill(Color32::from_rgba_unmultiplied(222, 230, 250, 66))
                        .corner_radius(20).min_size(Vec2::splat(35.0))).clicked() { self.media_action("playpause"); }
                    if ui.add(egui::Button::new(RichText::new(if browser { "+15" } else { "▶▶" }).size(17.0)).frame(false))
                        .on_hover_text(if browser { "Forward 15 seconds" } else { "Next track" }).clicked() {
                        if browser { self.seek_browser_media(false); } else { self.skip_media(false); }
                    }
                });
                ui.add_space(11.0);
                let old = self.volume;
                ui.add(egui::Slider::new(&mut self.volume, 0.0..=100.0).show_value(false));
                if (old - self.volume).abs() > 0.1 { set_system_volume(self.volume); }
            });
            if let Some(idx) = media_sheen { cols[0].painter().set(idx, glass_sheen(media.response.rect, 35)); }
            self.glass_rects[2] = Some(media.response.rect);

            let mut pulse_sheen = None;
            let pulse = frame().show(&mut cols[1], |ui| {
                pulse_sheen = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_min_height(card_height);
                glass_card_title(ui, "Pulse", 1);
                ui.add_space(12.0);
                for (name, value, history, tint) in [
                    ("CPU", self.cpu.as_str(), &self.cpu_history, Color32::from_rgb(95, 224, 186)),
                    ("MEM", self.ram.as_str(), &self.ram_history, Color32::from_rgb(120, 175, 255)),
                    ("SSD", self.disk.as_str(), &self.disk_history, Color32::from_rgb(201, 157, 255)),
                ] {
                    let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), egui::Sense::hover());
                    ui.painter().rect_filled(row, 12.0, Color32::from_rgba_unmultiplied(205, 215, 230, 16));
                    ui.painter().rect_stroke(row, 12.0, Stroke::new(0.6,
                        Color32::from_rgba_unmultiplied(240, 246, 255, 35)), egui::StrokeKind::Inside);
                    ui.painter().text(row.left_center() + Vec2::new(12.0, -5.0), Align2::LEFT_CENTER,
                        name, FontId::proportional(12.0), TEXT);
                    ui.painter().text(row.right_center() + Vec2::new(-10.0, -5.0), Align2::RIGHT_CENTER,
                        value, FontId::proportional(12.0), TEXT);
                    let points: Vec<_> = if history.len() < 2 {
                        let y = row.bottom() - 13.0;
                        vec![egui::pos2(row.right() - 104.0, y), egui::pos2(row.right() - 10.0, y)]
                    } else {
                        let low = history.iter().copied().fold(f32::INFINITY, f32::min);
                        let high = history.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                        let span = (high - low).max(8.0);
                        let midpoint = (high + low) * 0.5;
                        history.iter().enumerate().map(|(step, value)| {
                            let x = row.right() - 104.0 + step as f32 / (history.len() - 1) as f32 * 94.0;
                            let y = row.bottom() - 13.0 - (value - midpoint) / span * 10.0;
                            egui::pos2(x, y)
                        }).collect()
                    };
                    ui.painter().add(egui::Shape::line(points, Stroke::new(1.5, tint)));
                    ui.add_space(4.0);
                }
            });
            if let Some(idx) = pulse_sheen { cols[1].painter().set(idx, glass_sheen(pulse.response.rect, 35)); }
            self.glass_rects[3] = Some(pulse.response.rect);

            let mut timer_sheen = None;
            let timer = frame().show(&mut cols[2], |ui| {
                timer_sheen = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_min_height(card_height);
                glass_card_title(ui, "Pomodoro", 2);
                ui.add_space(2.0);
                ui.vertical_centered(|ui| {
                    let (ring, _) = ui.allocate_exact_size(Vec2::splat(132.0), egui::Sense::hover());
                    let center = ring.center();
                    let arc = |fraction: f32| (0..=64).map(|step| {
                        let angle = std::f32::consts::PI * 0.75
                            + step as f32 / 64.0 * std::f32::consts::PI * 1.5 * fraction;
                        center + Vec2::angled(angle) * 57.0
                    }).collect();
                    ui.painter().add(egui::Shape::line(arc(1.0), Stroke::new(7.0,
                        Color32::from_rgba_unmultiplied(199, 175, 242, 125))));
                    let progress = self.remaining() as f32 / self.timer_secs.max(1) as f32;
                    ui.painter().add(egui::Shape::line(arc(progress.clamp(0.0, 1.0)),
                        Stroke::new(7.0, Color32::from_rgb(210, 184, 255))));
                    ui.painter().text(center, Align2::CENTER_CENTER, format_time(self.remaining()), FontId::proportional(27.0), TEXT);
                    let play = egui::Rect::from_center_size(
                        egui::pos2(center.x, ring.bottom() - 2.0), Vec2::new(53.0, 29.0));
                    if ui.put(play, egui::Button::new(if self.timer_end.is_some() { "Ⅱ" } else { "▶" })
                        .corner_radius(17).fill(Color32::from_rgba_unmultiplied(218, 213, 246, 75)))
                        .clicked() {
                        if self.timer_end.is_some() {
                            self.timer_secs = self.remaining(); self.timer_end = None;
                        } else { self.timer_end = Some(Instant::now() + Duration::from_secs(self.timer_secs)); }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.small_button("↺").on_hover_text("Reset timer").clicked() {
                        self.timer_end = None; self.timer_secs = 25 * 60;
                    }
                    for minutes in [5, 15, 25] {
                        if ui.small_button(format!("{minutes}m")).clicked() {
                            self.timer_secs = minutes * 60; self.timer_end = None;
                        }
                    }
                });
            });
            if let Some(idx) = timer_sheen { cols[2].painter().set(idx, glass_sheen(timer.response.rect, 35)); }
            self.glass_rects[4] = Some(timer.response.rect);

            let mut todo_sheen = None;
            let todos = frame().show(&mut cols[3], |ui| {
                todo_sheen = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_min_height(card_height);
                ui.horizontal(|ui| {
                    let (icon, _) = ui.allocate_exact_size(Vec2::splat(21.0), egui::Sense::hover());
                    glass_card_icon(ui, icon, 3);
                    label(ui, "To-dos", 17.0, TEXT);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("+").size(20.0).color(TEXT)).frame(false))
                            .on_hover_text("Add task").clicked() {
                            if self.todo_editing && !self.todo_draft.trim().is_empty() {
                                self.prefs.todos.push(TodoItem { text: self.todo_draft.trim().into(), done: false });
                                self.todo_draft.clear();
                                self.todo_editing = false;
                            } else {
                                self.todo_editing = !self.todo_editing;
                                self.todo_focus_pending = self.todo_editing;
                            }
                        }
                    });
                });
                ui.add_space(7.0);
                ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(1.2,
                    Color32::from_rgba_unmultiplied(235, 240, 255, 135));
                let mut remove = None;
                for (index, item) in self.prefs.todos.iter_mut().take(5).enumerate() {
                    ui.horizontal(|ui| {
                        let done = item.done;
                        let mut text = RichText::new(&item.text).color(if done { MUTED } else { TEXT });
                        if done { text = text.strikethrough(); }
                        let check = ui.checkbox(&mut item.done, text);
                        let color = if check.hovered() { MUTED } else { Color32::TRANSPARENT };
                        if ui.add(egui::Button::new(RichText::new("×").color(color)).frame(false))
                            .on_hover_text("Remove task").clicked() { remove = Some(index); }
                    });
                    ui.add_space(1.0);
                }
                if let Some(index) = remove { self.prefs.todos.remove(index); }
                if self.prefs.todos.is_empty() {
                    ui.add_space(22.0);
                    label(ui, "Add your first task", 13.0, MUTED);
                }
                if self.todo_editing {
                    ui.add_space(3.0);
                    let added = ui.add(egui::TextEdit::singleline(&mut self.todo_draft).hint_text("New task"));
                    if self.todo_focus_pending {
                        ui.memory_mut(|memory| memory.request_focus(added.id));
                        self.todo_focus_pending = false;
                    }
                    if added.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.todo_draft.trim().is_empty() {
                        self.prefs.todos.push(TodoItem { text: self.todo_draft.trim().into(), done: false });
                        self.todo_draft.clear();
                        self.todo_editing = false;
                    }
                }
            });
            if let Some(idx) = todo_sheen { cols[3].painter().set(idx, glass_sheen(todos.response.rect, 35)); }
            self.glass_rects[5] = Some(todos.response.rect);
        });
    }
    fn compact_glass_nook(&mut self, ui: &mut egui::Ui, height: f32) {
        let available = ui.available_width();
        let widths = [available * 0.26, available * 0.202,
            available * 0.315, available * 0.223];
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            for (column, width) in widths.into_iter().enumerate() {
                if column > 0 {
                    let (divider, _) = ui.allocate_exact_size(Vec2::new(1.0, height), egui::Sense::hover());
                    ui.painter().line_segment([divider.center_top(), divider.center_bottom()],
                        Stroke::new(0.7, Color32::from_rgba_unmultiplied(235, 237, 244, 90)));
                }
                ui.allocate_ui_with_layout(Vec2::new(width - 1.0, height),
                    egui::Layout::top_down(Align::Min), |ui| {
                    ui.set_width(width - 13.0);
                    ui.add_space(5.0);
                    match column {
                        0 => {
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                let (art, _) = ui.allocate_exact_size(Vec2::splat(101.0), egui::Sense::hover());
                                if self.artwork_texture.is_some() {
                                    album_art(ui, art, self.artwork_texture.as_ref());
                                } else {
                                    ui.painter().rect_filled(art, 15.0,
                                        Color32::from_rgba_unmultiplied(27, 28, 33, 210));
                                    ui.painter().text(art.center(), Align2::CENTER_CENTER, "♫",
                                        FontId::proportional(35.0), MUTED);
                                }
                                ui.add_space((width - 265.0).max(12.0));
                                ui.vertical(|ui| {
                                    ui.add_space(15.0);
                                    let title = if self.media_title == "Nothing playing" {
                                        "Nothing playing".to_owned()
                                    } else { self.media_title.chars().take(20).collect() };
                                    music_text(ui, &title, 12.0, TEXT, true, false);
                                    music_text(ui, &self.media_artist.chars().take(23).collect::<String>(),
                                        10.0, MUTED, false, false);
                                    ui.add_space(12.0);
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 12.0;
                                        let browser = is_browser_media_app(self.media_app.as_deref());
                                        if ui.add(egui::Button::new(
                                            RichText::new(if browser { "−15" } else { "◀◀" }).size(18.0).color(Color32::WHITE))
                                            .frame(false).min_size(Vec2::new(30.0, 27.0)))
                                            .on_hover_text(if browser { "Back 15 seconds" } else { "Previous track" }).clicked() {
                                            self.skip_media(true);
                                        }
                                        if ui.add(egui::Button::new(
                                            RichText::new(if self.media_playing { "Ⅱ" } else { "▶" })
                                                .size(22.0).color(Color32::WHITE))
                                            .frame(false).min_size(Vec2::new(30.0, 27.0)))
                                            .on_hover_text("Play or pause").clicked() {
                                            self.media_action("playpause");
                                        }
                                        if ui.add(egui::Button::new(
                                            RichText::new(if browser { "+15" } else { "▶▶" }).size(18.0).color(Color32::WHITE))
                                            .frame(false).min_size(Vec2::new(30.0, 27.0)))
                                            .on_hover_text(if browser { "Forward 15 seconds" } else { "Next track" }).clicked() {
                                            self.skip_media(false);
                                        }
                                    });
                                });
                            });
                        }
                        1 => {
                            let (slot, _) = ui.allocate_exact_size(Vec2::new(width - 1.0, height - 1.0),
                                egui::Sense::hover());
                            let area = slot.shrink2(Vec2::new(11.0, 0.0));
                            let gap = 8.0;
                            let card_width = (area.width() - gap) * 0.5;
                            let cards = [self.prefs.pulse_left.clone(), self.prefs.pulse_right.clone()]
                                .map(|name| {
                                    let value = self.pulse_metric(&name);
                                    let history = self.pulse_history(&name).cloned();
                                    let tint = match name.as_str() {
                                        "CPU" | "Battery" => Color32::from_rgb(115, 216, 163),
                                        "RAM" | "Memory" => Color32::from_rgb(242, 187, 92),
                                        "GPU" => Color32::from_rgb(179, 153, 245),
                                        "Disk" => Color32::from_rgb(135, 190, 244),
                                        _ => Color32::from_rgb(132, 210, 225),
                                    };
                                    (name, value, history, tint)
                                });
                            for (index, (name, value, history, tint)) in cards.into_iter().enumerate() {
                                let rect = egui::Rect::from_min_size(
                                    area.min + Vec2::new(index as f32 * (card_width + gap), 0.0),
                                    Vec2::new(card_width, area.height()));
                                ui.painter().rect_filled(rect, 17.0,
                                    Color32::from_rgba_unmultiplied(4, 6, 8, 235));
                                ui.painter().rect_stroke(rect, 17.0,
                                    Stroke::new(0.7, Color32::from_rgba_unmultiplied(225, 231, 241, 43)),
                                    egui::StrokeKind::Inside);
                                if name == "CPU" || name == "RAM" {
                                    ui.painter().line_segment([
                                        egui::pos2(rect.left() + 10.0, rect.top() + 36.0),
                                        egui::pos2(rect.right() - 10.0, rect.top() + 36.0),
                                    ], Stroke::new(0.7, Color32::from_rgba_unmultiplied(230, 235, 240, 31)));
                                }
                                let icon = egui::Rect::from_min_size(rect.min + Vec2::new(8.0, 9.0),
                                    Vec2::splat(11.0));
                                if name == "CPU" {
                                    ui.painter().rect_stroke(icon.shrink(1.5), 1.0,
                                        Stroke::new(1.1, TEXT), egui::StrokeKind::Inside);
                                    for offset in [2.5, 5.5, 8.5] {
                                        for side in [icon.left(), icon.right()] {
                                            let direction = if side == icon.left() { -2.0 } else { 2.0 };
                                            let y = icon.top() + offset;
                                            ui.painter().line_segment([
                                                egui::pos2(side, y), egui::pos2(side + direction, y),
                                            ], Stroke::new(0.8, TEXT));
                                        }
                                        for side in [icon.top(), icon.bottom()] {
                                            let direction = if side == icon.top() { -2.0 } else { 2.0 };
                                            let x = icon.left() + offset;
                                            ui.painter().line_segment([
                                                egui::pos2(x, side), egui::pos2(x, side + direction),
                                            ], Stroke::new(0.8, TEXT));
                                        }
                                    }
                                } else if name == "RAM" || name == "Memory" {
                                    ui.painter().rect_stroke(icon.shrink2(Vec2::new(0.0, 2.0)), 1.0,
                                        Stroke::new(1.1, TEXT), egui::StrokeKind::Inside);
                                    for offset in [2.5, 5.5, 8.5] {
                                        let x = icon.left() + offset;
                                        ui.painter().line_segment([
                                            egui::pos2(x, icon.bottom() - 2.0),
                                            egui::pos2(x, icon.bottom() + 1.0),
                                        ], Stroke::new(0.8, TEXT));
                                    }
                                } else {
                                    let short = match name.as_str() {
                                        "Battery" => "BAT", "Network" => "NET",
                                        "Disk" => "SSD", _ => "GPU",
                                    };
                                    ui.painter().text(icon.left_top(), Align2::LEFT_TOP,
                                        short, FontId::proportional(8.0), TEXT);
                                }
                                ui.painter().text(rect.right_top() + Vec2::new(-8.0, 9.0),
                                    Align2::RIGHT_TOP, &value,
                                    FontId::proportional(if value.chars().count() > 5 { 11.0 } else { 15.0 }),
                                    tint);
                                if let Some(history) = history.as_ref().filter(|history| history.len() > 1) {
                                    let graph_left = match name.as_str() {
                                        "CPU" => rect.right() - 32.0,
                                        "RAM" => rect.center().x + 4.0,
                                        _ => rect.left() + 9.0,
                                    };
                                    let graph_right = rect.right() - 9.0;
                                    let recent_low = history.iter().copied().fold(f32::INFINITY, f32::min);
                                    let points: Vec<_> = history.iter().enumerate().map(|(i, v)| {
                                        let y = if name == "CPU" {
                                            rect.bottom() - 15.0 - ((v - recent_low) / 12.0).clamp(0.0, 1.0) * 6.0
                                        } else {
                                            rect.bottom() - 9.0 - (v / 100.0).clamp(0.0, 1.0) *
                                                (rect.height() - 49.0)
                                        };
                                        egui::pos2(graph_left + i as f32 /
                                            (history.len() - 1) as f32 * (graph_right - graph_left), y)
                                    }).collect();
                                    if name == "RAM" {
                                        let top = points.iter().map(|point| point.y).fold(f32::INFINITY, f32::min);
                                        ui.painter().rect_filled(egui::Rect::from_min_max(
                                            egui::pos2(graph_left, top),
                                            egui::pos2(graph_right, rect.bottom() - 9.0)), 0.0,
                                            Color32::from_rgba_unmultiplied(242, 187, 92, 18));
                                    }
                                    ui.painter().add(egui::Shape::line(points,
                                        Stroke::new(1.5, tint)));
                                }
                                if self.prefs.pulse_explain {
                                    let explanation = match name.as_str() {
                                        "RAM" | "Memory" => "System-wide physical memory used, including macOS compression. The orange line shows recent readings; it is not Bookland's memory use or a memory-pressure warning.",
                                        "CPU" => "System-wide CPU activity. The green line shows recent readings.",
                                        "GPU" => "System-wide GPU activity when this Mac reports it.",
                                        "Battery" => "Current Mac battery charge.",
                                        "Disk" => "Current disk use.",
                                        _ => "Current system reading.",
                                    };
                                    ui.interact(rect, ui.make_persistent_id(("nook-pulse", index)),
                                        egui::Sense::hover()).on_hover_text(explanation);
                                }
                                self.glass_rects[2 + index] = Some(rect);
                            }
                        }
                        2 => {
                            let orange = Color32::from_rgb(255, 168, 45);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                ui.add_space(25.0);
                                if ui.add(egui::Button::new(RichText::new("Focus").size(13.0).color(orange))
                                    .corner_radius(17).min_size(Vec2::new(69.0, 28.0))
                                    .fill(Color32::from_rgba_unmultiplied(163, 91, 12, 135)))
                                    .clicked() {
                                    self.timer_secs = 10 * 60;
                                    self.timer_end = None;
                                    self.stopwatch_started = None;
                                    self.stopwatch_saved = Duration::ZERO;
                                }
                                if ui.add(egui::Button::new(RichText::new("Break").size(13.0).color(TEXT))
                                    .corner_radius(17).min_size(Vec2::new(68.0, 28.0))
                                    .fill(Color32::from_rgba_unmultiplied(53, 53, 58, 185)))
                                    .clicked() {
                                    self.timer_secs = 5 * 60;
                                    self.timer_end = None;
                                    self.stopwatch_started = None;
                                    self.stopwatch_saved = Duration::ZERO;
                                }
                                let seconds = if self.stopwatch_started.is_some() || !self.stopwatch_saved.is_zero() {
                                    self.stopwatch_elapsed().as_secs()
                                } else { self.remaining() };
                                ui.label(RichText::new(format_time(seconds))
                                    .size(31.0).color(orange));
                            });
                            let (tick_slot, _) = ui.allocate_exact_size(Vec2::new(width - 22.0, 15.0),
                                egui::Sense::hover());
                            let tick_labels = egui::Rect::from_min_size(
                                tick_slot.min + Vec2::new(50.0, 0.0),
                                Vec2::new(width - 75.0, 15.0));
                            for (index, minutes) in [5, 10, 15, 20].into_iter().enumerate() {
                                let x = tick_labels.left() + (index + 1) as f32 / 4.0 * tick_labels.width();
                                ui.painter().text(egui::pos2(x, tick_labels.center().y), Align2::CENTER_CENTER,
                                    minutes.to_string(), FontId::proportional(11.0),
                                    if minutes == 10 { TEXT } else { Color32::from_rgb(177, 139, 76) });
                            }
                            let (tick_slot, _) = ui.allocate_exact_size(Vec2::new(width - 22.0, 26.0),
                                egui::Sense::hover());
                            let ticks = egui::Rect::from_min_size(
                                tick_slot.min + Vec2::new(50.0, 0.0),
                                Vec2::new(width - 75.0, 26.0));
                            for index in 0..21 {
                                let x = ticks.left() + index as f32 / 20.0 * ticks.width();
                                let color = if index <= 10 { Color32::from_rgb(242, 164, 62) }
                                    else { Color32::from_rgba_unmultiplied(218, 165, 95, 92) };
                                ui.painter().line_segment([
                                    egui::pos2(x, ticks.top() + 3.0),
                                    egui::pos2(x, ticks.bottom() - 3.0)], Stroke::new(1.5, color));
                            }
                            let marker = egui::pos2(ticks.center().x, ticks.bottom() - 1.0);
                            ui.painter().add(egui::Shape::convex_polygon(vec![
                                marker + Vec2::new(0.0, -5.0),
                                marker + Vec2::new(-4.0, 1.0),
                                marker + Vec2::new(4.0, 1.0)], orange, Stroke::NONE));
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                ui.add_space(25.0);
                                let timer_caption = if self.timer_end.is_some() {
                                    "Pause Timer" } else { "Start Timer" };
                                if ui.add(egui::Button::new(RichText::new(timer_caption)
                                    .size(13.5).strong().color(orange))
                                    .corner_radius(19).min_size(Vec2::new(112.0, 34.0))
                                    .fill(Color32::from_rgba_unmultiplied(224, 166, 99, 145)))
                                    .clicked() {
                                    if self.timer_end.is_some() {
                                        self.timer_secs = self.remaining(); self.timer_end = None;
                                    } else {
                                        self.stopwatch_started = None;
                                        self.stopwatch_saved = Duration::ZERO;
                                        self.timer_end = Some(Instant::now() +
                                            Duration::from_secs(self.timer_secs));
                                    }
                                }
                                let volume = ui.add(egui::Button::new("").corner_radius(18)
                                    .min_size(Vec2::splat(34.0))
                                    .fill(Color32::from_rgba_unmultiplied(213, 170, 116, 105)))
                                    .on_hover_text(if self.muted_volume.is_some() { "Unmute" } else { "Mute" });
                                let center = volume.rect.center();
                                ui.painter().add(egui::Shape::convex_polygon(vec![
                                    center + Vec2::new(-8.0, -3.0), center + Vec2::new(-4.0, -3.0),
                                    center + Vec2::new(1.0, -7.0), center + Vec2::new(1.0, 7.0),
                                    center + Vec2::new(-4.0, 3.0), center + Vec2::new(-8.0, 3.0),
                                ], orange, Stroke::NONE));
                                for radius in [7.0_f32, 10.0] {
                                    let arc: Vec<_> = (0..=12).map(|step| {
                                        let angle = -0.7 + step as f32 / 12.0 * 1.4;
                                        center + Vec2::new(1.0 + radius * angle.cos(), radius * angle.sin())
                                    }).collect();
                                    ui.painter().add(egui::Shape::line(arc, Stroke::new(1.3, orange)));
                                }
                                if volume.clicked() { self.toggle_volume_mute(); }
                                let stopwatch = ui.add(egui::Button::new("").corner_radius(18)
                                    .min_size(Vec2::splat(34.0))
                                    .fill(Color32::from_rgba_unmultiplied(205, 206, 210, 63)))
                                    .on_hover_text(if self.stopwatch_started.is_some() {
                                        "Pause stopwatch" } else { "Start stopwatch" });
                                let center = stopwatch.rect.center();
                                ui.painter().circle_stroke(center + Vec2::new(0.0, 1.5), 7.0,
                                    Stroke::new(1.6, TEXT));
                                ui.painter().line_segment([center + Vec2::new(0.0, -9.0),
                                    center + Vec2::new(0.0, -6.0)], Stroke::new(2.0, TEXT));
                                ui.painter().line_segment([center + Vec2::new(0.0, 1.5),
                                    center + Vec2::new(3.0, -1.5)], Stroke::new(1.4, TEXT));
                                if stopwatch.clicked() { self.toggle_stopwatch(); }
                                stopwatch.context_menu(|ui| {
                                    if ui.button("Reset stopwatch").clicked() {
                                        self.stopwatch_started = None;
                                        self.stopwatch_saved = Duration::ZERO;
                                        ui.close();
                                    }
                                });
                            });
                        }
                        _ => {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.spacing_mut().item_spacing.y = 2.0;
                                let (icon, _) = ui.allocate_exact_size(Vec2::new(24.0, 16.0),
                                    egui::Sense::hover());
                                let orange = Color32::from_rgb(243, 178, 87);
                                for row in 0..2 {
                                    let y = icon.top() + 5.0 + row as f32 * 9.0;
                                    ui.painter().circle_stroke(egui::pos2(icon.left() + 5.0, y),
                                        2.4, Stroke::new(1.3, orange));
                                    ui.painter().line_segment([
                                        egui::pos2(icon.left() + 12.0, y),
                                        egui::pos2(icon.right() - 1.0, y)], Stroke::new(1.4, orange));
                                }
                                if self.prefs.todos.is_empty() {
                                    ui.label(RichText::new("No tasks yet").size(12.0).color(MUTED));
                                } else {
                                    let mut remove = None;
                                    for (index, item) in self.prefs.todos.iter_mut().take(3).enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut item.done, &item.text);
                                            if ui.small_button("×").clicked() { remove = Some(index); }
                                        });
                                    }
                                    if let Some(index) = remove { self.prefs.todos.remove(index); }
                                }
                                if ui.add(egui::Button::new(RichText::new("+ Add a task")
                                    .size(13.0).strong().color(Color32::from_rgb(255, 168, 45)))
                                    .corner_radius(15)
                                    .fill(Color32::from_rgba_unmultiplied(241, 179, 94, 65)))
                                    .clicked() {
                                    self.todo_editing = true;
                                    self.todo_focus_pending = true;
                                }
                                if self.todo_editing {
                                    let response = ui.add(egui::TextEdit::singleline(&mut self.todo_draft)
                                        .hint_text("New task"));
                                    if self.todo_focus_pending {
                                        ui.memory_mut(|memory| memory.request_focus(response.id));
                                        self.todo_focus_pending = false;
                                    }
                                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        && !self.todo_draft.trim().is_empty() {
                                        self.prefs.todos.push(TodoItem {
                                            text: self.todo_draft.trim().into(), done: false });
                                        self.todo_draft.clear(); self.todo_editing = false;
                                    }
                                }
                            });
                        }
                    }
                });
            }
        });
    }
    fn visible_workspaces(&self) -> Vec<Workspace> {
        let mut workspaces = if self.prefs.show_extra_workspaces {
            Workspace::DEFAULT.to_vec()
        } else {
            Workspace::ISLAND.to_vec()
        };
        if self.prefs.prompter_enabled { workspaces.push(Workspace::Prompter); }
        workspaces
    }
    fn menu_workspace_active(&self, workspace: Workspace) -> bool {
        self.workspace == workspace
            || (!self.prefs.show_extra_workspaces && self.workspace == Workspace::Clipboard
                && workspace == Workspace::Tray)
            || (!self.prefs.show_extra_workspaces && self.workspace == Workspace::Prompts
                && workspace == Workspace::Agents)
    }
    fn glass_workspace_menu(&mut self, ui: &mut egui::Ui) {
        let workspaces = self.visible_workspaces();
        let menu = Frame::new()
            .fill(Color32::from_rgb(10, 11, 14))
            .corner_radius(16)
            .stroke(Stroke::new(0.7, Color32::from_rgba_unmultiplied(216, 231, 249, 90)))
            .shadow(egui::epaint::Shadow { offset: [0, 5], blur: 17, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 5, 16, 92) })
            .inner_margin(Margin::symmetric(3, 1))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.horizontal(|ui| {
                    for workspace in workspaces {
                        let active = self.menu_workspace_active(workspace);
                        let labeled = active && self.prefs.menu_labels;
                        let menu_name = if workspace == Workspace::Nook {
                            "Bookland"
                        } else { workspace.name() };
                        let response = ui.add(egui::Button::new("")
                            .fill(if active { Color32::from_rgb(57, 58, 62) }
                                else { Color32::TRANSPARENT })
                            .corner_radius(17).min_size(Vec2::new(if labeled { 96.0 } else { 30.0 }, 29.0)))
                            .on_hover_text(if self.workspace == workspace {
                                "Collapse island"
                            } else { menu_name });
                        response.widget_info(|| egui::WidgetInfo::labeled(
                            egui::WidgetType::Button, true, menu_name));
                        paint_workspace_icon(ui, response.rect, workspace, self.bookland_mark.as_ref(),
                            if active { TEXT } else { MUTED }, self.prefs.menu_style == "Dots");
                        if labeled {
                            ui.painter().text(egui::pos2(response.rect.left() + 38.0, response.rect.center().y),
                                Align2::LEFT_CENTER, menu_name, FontId::proportional(12.0), TEXT);
                        }
                        if response.clicked() {
                            if self.workspace == workspace {
                                self.expanded = false;
                                self.opened_by_hover = false;
                                self.hover_blocked_until_exit = true;
                                self.clipboard_bottom_sheet = false;
                            } else {
                                self.workspace = workspace;
                                self.workspace_chosen_by_user = true;
                                if workspace == Workspace::Agents {
                                    self.agent_chat_open = false;
                                    self.agent_usage_view = false;
                                }
                                if workspace == Workspace::Clipboard {
                                    self.clipboard_bottom_sheet = self.prefs.clipboard_open_bottom;
                                    self.clipboard_focus_pending = self.prefs.clipboard_focus_search;
                                }
                            }
                        }
                    }
                });
            });
        self.glass_rects[1] = Some(menu.response.rect);
    }
    fn glass_menu_below(&mut self, ui: &mut egui::Ui, width: f32) {
        ui.add_space(10.0);
        let menu_items = self.visible_workspaces();
        let menu_width = menu_items.iter().map(|workspace|
            if self.menu_workspace_active(*workspace) && self.prefs.menu_labels { 96.0 } else { 30.0 })
            .sum::<f32>() + 6.0;
        ui.horizontal(|ui| {
            // The transparent host is 44 points wider than the painted panel.
            ui.add_space(((width - menu_width) * 0.5 + 22.0).max(0.0));
            self.glass_workspace_menu(ui);
        });
    }
    fn compact_glass_panel(&mut self, ui: &mut egui::Ui, width: f32, height: f32) {
        let mut gradient = None;
        let panel = Frame::new()
            .fill(glass_surface_fill())
            .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 29, se: 29 })
            .shadow(egui::epaint::Shadow { offset: [0, 9], blur: 28, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 125) })
            .inner_margin(Margin::symmetric(17, 13))
            .show(ui, |ui| {
                gradient = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_width(width - 34.0);
                ui.set_min_height(height - 26.0);
                ui.add_space(13.0);
                self.compact_glass_nook(ui, height - 45.0);
            });
        if let Some(index) = gradient {
            ui.painter().set(index, island_upper_shade(panel.response.rect));
        }
        self.glass_rects[0] = Some(panel.response.rect);
        self.glass_menu_below(ui, width);
    }
    fn compact_glass_music_panel(&mut self, ui: &mut egui::Ui, width: f32, height: f32) {
        let apple_music = self.media_app.as_deref() == Some("Music");
        let mut gradient = None;
        let panel = Frame::new()
            .fill(glass_surface_fill())
            .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 29, se: 29 })
            .shadow(egui::epaint::Shadow { offset: [0, 9], blur: 28, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 125) })
            .inner_margin(Margin::symmetric(20, 13))
            .show(ui, |ui| {
                gradient = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_width(width - 40.0);
                ui.set_min_height(height - 26.0);
                ui.add_space(31.0);
                ui.horizontal(|ui| {
                    let (art, _) = ui.allocate_exact_size(Vec2::splat(96.0), egui::Sense::hover());
                    album_art(ui, art, self.artwork_texture.as_ref());
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.set_width(width - 155.0);
                        label(ui, "MUSIC", 10.0, MUTED);
                        music_text(ui, &self.media_title, 17.0, TEXT, true, true);
                        music_text(ui, &self.media_artist, 12.0, MUTED, false, true);
                        ui.add_space(9.0);
                        self.media_controls(ui);
                    });
                });
                ui.add_space(11.0);
                if self.media_duration.is_finite() && self.media_duration > 0.0 {
                    let displayed = (self.media_position
                        + if self.media_playing { self.last_media_poll.elapsed().as_secs_f32() }
                            else { 0.0 }).clamp(0.0, self.media_duration);
                    let mut seek_to = displayed;
                    let old_slider_width = ui.spacing().slider_width;
                    ui.spacing_mut().slider_width = width - 42.0;
                    let seek = ui.add(egui::Slider::new(
                        &mut seek_to, 0.0..=self.media_duration).show_value(false));
                    ui.spacing_mut().slider_width = old_slider_width;
                    if seek.changed() {
                        self.media_position = seek_to;
                        self.last_media_poll = Instant::now();
                    }
                    if self.media_app.is_some()
                        && (seek.drag_stopped() || (seek.changed() && !seek.dragged())) {
                        self.media_action(&format!("set player position to {seek_to}"));
                    }
                    ui.horizontal(|ui| {
                        label(ui, &format_time(displayed as u64), 10.0, MUTED);
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            label(ui, &format_time(self.media_duration as u64), 10.0, MUTED);
                        });
                    });
                } else {
                    ui.add_space(31.0);
                }
                if self.prefs.audio_spectrum {
                    self.paint_audio_spectrum(ui, (width - 44.0).max(100.0), 19.0);
                    if self.spectrum_state < 0 {
                        label(ui, "Allow System Audio Recording to show the spectrum.",
                            10.0, MUTED);
                    }
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if !self.lyrics_lines.is_empty()
                        && small_button(ui, if self.lyrics_open { "Hide lyrics" } else { "Lyrics" })
                            .on_hover_text("Show synced lyrics").clicked() {
                        self.lyrics_open = !self.lyrics_open;
                        self.airplay_open = false;
                        self.queue_open = false;
                    }
                    if apple_music && small_button(ui,
                        if self.airplay_open { "Hide outputs" } else { "AirPlay" })
                        .clicked() {
                        self.airplay_open = !self.airplay_open;
                        self.lyrics_open = false;
                        self.queue_open = false;
                        if self.airplay_open { self.refresh_music_airplay(); }
                    }
                    if apple_music && small_button(ui,
                        if self.queue_open { "Hide next" } else { "Playing Next" })
                        .clicked() {
                        self.queue_open = !self.queue_open;
                        self.airplay_open = false;
                        self.lyrics_open = false;
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let old = self.volume;
                        ui.add_sized([105.0, 20.0], egui::Slider::new(
                            &mut self.volume, 0.0..=100.0).show_value(false));
                        if (old - self.volume).abs() > 0.1 { set_system_volume(self.volume); }
                        label(ui, &format!("VOL {:.0}%", self.volume), 10.0, MUTED);
                    });
                });
                ui.add_space(9.0);
                if apple_music && self.airplay_open {
                    if small_button(ui, "Refresh outputs").clicked() { self.refresh_music_airplay(); }
                    if !self.airplay_status.is_empty() {
                        label(ui, &self.airplay_status, 11.0, MUTED);
                    }
                    egui::ScrollArea::vertical().max_height(115.0).show(ui, |ui| {
                        for device in self.airplay_devices.clone().into_iter()
                            .filter(|item| item.supports_audio) {
                            let mut selected = device.selected;
                            let response = ui.add_enabled(device.available && self.airplay_rx.is_none(),
                                egui::Checkbox::new(&mut selected, &device.name));
                            if response.changed() { self.select_music_airplay(device.name, selected); }
                        }
                    });
                } else if apple_music && self.queue_open {
                    self.playing_next_preview(ui, true);
                } else if self.lyrics_open && !self.lyrics_lines.is_empty() {
                    let position = self.media_position
                        + if self.media_playing { self.last_media_poll.elapsed().as_secs_f32() }
                            else { 0.0 };
                    let active = lyrics::active_line(&self.lyrics_lines, position).unwrap_or(0);
                    let end = (active + 3).min(self.lyrics_lines.len());
                    for (index, line) in self.lyrics_lines[active..end].iter().enumerate() {
                        music_text(ui, if line.text.is_empty() { "♪" } else { &line.text },
                            if index == 0 { 14.0 } else { 12.0 },
                            if index == 0 { TEXT } else { MUTED }, false, true);
                    }
                    label(ui, "Lyrics by LRCLIB", 10.0, MUTED);
                } else {
                    let source = self.airpods.compact_label().unwrap_or_else(||
                        self.media_app.as_deref().map(|app| format!("SOURCE  ·  {app}"))
                            .unwrap_or_else(|| "Play music on your Mac".into()));
                    label(ui, &source, 12.0, MUTED);
                }
            });
        if let Some(index) = gradient {
            ui.painter().set(index, agents_upper_shade(panel.response.rect));
        }
        self.glass_rects[0] = Some(panel.response.rect);
        self.glass_menu_below(ui, width);
    }
    fn compact_glass_workspace_panel(&mut self, ui: &mut egui::Ui,
                                     width: f32, height: f32) {
        let mut gradient = None;
        let panel = Frame::new()
            .fill(glass_surface_fill())
            .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 29, se: 29 })
            .shadow(egui::epaint::Shadow { offset: [0, 9], blur: 28, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 125) })
            .inner_margin(Margin::symmetric(18, 13))
            .show(ui, |ui| {
                gradient = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_width(width - 36.0);
                ui.set_min_height(height - 26.0);
                ui.add_space(28.0);
                ui.horizontal(|ui| {
                    label(ui, self.workspace.name(), 17.0, TEXT);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("⚙").on_hover_text("Settings").clicked() {
                            self.settings_open = true;
                        }
                    });
                });
                ui.add_space(8.0);
                egui::ScrollArea::both().max_height(height - 85.0).show(ui, |ui| {
                    ui.set_min_width(width - 48.0);
                    match self.workspace {
                        Workspace::Calendar => self.calendar(ui),
                        Workspace::Tray => self.tray(ui),
                        Workspace::Clipboard => self.clipboard_browser(ui, false),
                        Workspace::Prompts => self.sheet_prompt_builder(ui),
                        Workspace::Prompter => self.prompter_workspace(ui),
                        _ => {}
                    }
                });
            });
        if let Some(index) = gradient {
            ui.painter().set(index, agents_upper_shade(panel.response.rect));
        }
        self.glass_rects[0] = Some(panel.response.rect);
        self.glass_menu_below(ui, width);
    }
    fn compact_glass_agents_panel(&mut self, ui: &mut egui::Ui, width: f32, height: f32) {
        self.agent_completion = None;
        let agent = self.agents.get(self.selected_agent).or_else(|| self.agents.first()).cloned();
        let decision_pending = agent.as_ref().is_some_and(|agent|
            agent.source == "Codex"
                && desktop_codex_thread_id(&agent.path) == self.desktop_codex_thread_id
                && (self.desktop_codex_approval.is_some() || self.desktop_codex_question.is_some()));
        let mut gradient = None;
        let panel = Frame::new()
            .fill(glass_surface_fill())
            .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 29, se: 29 })
            .shadow(egui::epaint::Shadow { offset: [0, 9], blur: 28, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 125) })
            .inner_margin(Margin::symmetric(20, 13))
            .show(ui, |ui| {
                gradient = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_width(width - 40.0);
                ui.set_min_height(height - 26.0);
                ui.add_space(32.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    let (icon, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                    let c = icon.center();
                    if self.agent_usage_view {
                        paint_agent_usage_icon(ui.painter(), c, MUTED);
                    } else {
                        paint_agent_progress_icon(ui.painter(), c, MUTED);
                    }
                    label(ui, if self.agent_usage_view { "Usage" } else { "Progress" }, 14.0, MUTED);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(8.0);
                        let usage = ui.add(egui::Button::new("")
                            .fill(if self.agent_usage_view { Color32::from_rgb(36, 93, 67) }
                                else { Color32::from_rgb(45, 46, 51) })
                            .corner_radius(16).min_size(Vec2::splat(28.0)))
                            .on_hover_text("Usage");
                        usage.widget_info(|| egui::WidgetInfo::labeled(
                            egui::WidgetType::Button, true, "Usage"));
                        paint_agent_usage_icon(ui.painter(), usage.rect.center(),
                            if self.agent_usage_view { Color32::from_rgb(82, 223, 145) } else { TEXT });
                        if usage.clicked() {
                            self.agent_usage_view = true;
                        }
                        let progress = ui.add(egui::Button::new("")
                            .fill(if !self.agent_usage_view { Color32::from_rgb(36, 93, 67) }
                                else { Color32::from_rgb(45, 46, 51) })
                            .corner_radius(16).min_size(Vec2::splat(28.0)))
                            .on_hover_text("Progress");
                        progress.widget_info(|| egui::WidgetInfo::labeled(
                            egui::WidgetType::Button, true, "Progress"));
                        paint_agent_progress_icon(ui.painter(), progress.rect.center(),
                            if !self.agent_usage_view { Color32::from_rgb(82, 223, 145) } else { TEXT });
                        if progress.clicked() {
                            self.agent_usage_view = false;
                        }
                        let active = self.agents.iter().enumerate()
                            .filter(|(_, item)| item.status == "Working")
                            .map(|(index, item)| (index, item.source.clone(), item.title.clone()))
                            .collect::<Vec<_>>();
                        let show_selectors = active.len() > 1 || active.first()
                            .is_some_and(|(index, _, _)| *index != self.selected_agent);
                        if show_selectors {
                            ui.add_space(8.0);
                            ui.menu_button(
                                RichText::new(format!("{} active ▾", active.len()))
                                    .size(12.0).color(MUTED),
                                |ui| {
                                    ui.set_min_width(250.0);
                                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                        for (index, source, title) in active {
                                            if ui.selectable_label(index == self.selected_agent,
                                                format!("{source} · {title}")).clicked() {
                                                self.selected_agent = index;
                                                ui.close();
                                            }
                                        }
                                    });
                                },
                            ).response.on_hover_text("Choose a working task");
                        }
                        if let Some(thread_id) = agent.as_ref()
                            .filter(|item| item.source == "Codex")
                            .and_then(|item| desktop_codex_thread_id(&item.path)) {
                            let can_switch = self.desktop_codex_rx.is_none()
                                || self.desktop_codex_thread_id.as_ref() == Some(&thread_id);
                            if ui.add_enabled(can_switch, egui::Button::new("This turn")
                                .corner_radius(12))
                                .on_hover_text("Reply to the selected Codex task in Bookland")
                                .clicked() {
                                self.desktop_codex_thread_id = Some(thread_id);
                                self.agent_chat_desktop = true;
                                self.agent_chat_open = true;
                            }
                        }
                        if ui.add(egui::Button::new("Bookland chat").corner_radius(12))
                            .on_hover_text("Open Bookland's connected Codex chat")
                            .clicked() {
                            self.agent_chat_desktop = false;
                            self.agent_chat_open = true;
                        }
                    });
                });
                ui.add_space(4.0);
                if let Some(agent) = agent.as_ref() {
                    if self.agent_usage_view {
                        let (area, _) = ui.allocate_exact_size(Vec2::new(width - 40.0, 142.0),
                            egui::Sense::hover());
                        let center = area.center_top() + Vec2::new(0.0, 33.0);
                        let blue = Color32::from_rgb(87, 145, 248);
                        let native_ring = agent.status == "Working" && self.native_installed
                            && self.transition == 1.0 && !self.preview_mode
                            && self.screenshot.is_none();
                        if native_ring {
                            self.native_usage_ring_center = Some(center);
                        } else {
                            ui.painter().circle_stroke(center, 33.0,
                                Stroke::new(3.0, Color32::from_rgb(27, 48, 82)));
                            let time = ui.input(|input| input.time) as f32;
                            let arc: Vec<_> = (0..=32).map(|step| {
                                let angle = if agent.status == "Working" { time * 1.8 }
                                    else { -1.57 }
                                    + step as f32 / 32.0 * std::f32::consts::TAU * 0.39;
                                center + Vec2::angled(angle) * 33.0
                            }).collect();
                            ui.painter().add(egui::Shape::line(arc, Stroke::new(3.0, blue)));
                        }
                        if agent.status == "Working" {
                            let satellite = center + Vec2::new(24.0, -24.0);
                            ui.painter().circle_filled(satellite, 6.5,
                                Color32::from_rgb(4, 8, 15));
                            ui.painter().circle_stroke(satellite, 5.8,
                                Stroke::new(1.5, blue));
                        }
                        ui.painter().rect_filled(egui::Rect::from_center_size(center,
                            Vec2::splat(33.0)), 8.0, Color32::from_rgb(57, 91, 194));
                        paint_codex_logo(ui.painter(), center, 16.0, TEXT,
                            self.codex_logo.as_ref());
                        ui.painter().text(center + Vec2::new(0.0, 46.0), Align2::CENTER_CENTER,
                            &agent.source, FontId::proportional(16.0), TEXT);
                        ui.painter().text(center + Vec2::new(0.0, 62.0), Align2::CENTER_CENTER,
                            if agent.status == "Working" { "Thinking…" } else { &agent.status },
                            FontId::proportional(13.0), blue);
                    } else {
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.allocate_ui_with_layout(Vec2::new(290.0, 141.0),
                                egui::Layout::top_down(Align::Min), |ui| {
                                ui.set_width(290.0);
                                ui.horizontal(|ui| {
                                    let (icon, _) = ui.allocate_exact_size(Vec2::splat(18.0),
                                        egui::Sense::hover());
                                    if agent.source == "Codex" {
                                        paint_codex_logo(ui.painter(), icon.center(), 17.0,
                                            Color32::from_rgb(87, 145, 248), self.codex_logo.as_ref());
                                    } else {
                                        ui.painter().circle_stroke(icon.center(), 7.0,
                                            Stroke::new(1.5, Color32::from_rgb(87, 145, 248)));
                                    }
                                    ui.label(RichText::new(&agent.source)
                                        .size(16.0).strong().color(TEXT));
                                    label(ui, "•", 13.0, Color32::from_rgb(91, 147, 255));
                                    let mut title: String = agent.title.chars().take(25).collect();
                                    if agent.title.chars().count() > 25 { title.push('…'); }
                                    let title_button = ui.add(egui::Label::new(RichText::new(&title)
                                        .size(13.0).color(Color32::from_rgb(91, 147, 255)))
                                        .sense(egui::Sense::click()))
                                        .on_hover_text("Right click to choose a recent task");
                                    title_button.context_menu(|ui| {
                                        for (index, item) in self.agents.iter().enumerate() {
                                            let caption = format!("{} · {}", item.source,
                                                item.title.chars().take(48).collect::<String>());
                                            if ui.selectable_label(index == self.selected_agent,
                                                caption).clicked() && self.desktop_codex_rx.is_none() {
                                                self.selected_agent = index;
                                                self.agent_chat_open = false;
                                                ui.close();
                                            }
                                        }
                                    });
                                });
                                ui.add_space(11.0);
                                let start = agent.events.len().saturating_sub(4);
                                for line in &agent.events[start..] {
                                    let failed = line.to_ascii_lowercase().contains("fail");
                                    ui.horizontal(|ui| {
                                        let (icon, _) = ui.allocate_exact_size(Vec2::splat(15.0),
                                            egui::Sense::hover());
                                        let color = if failed { Color32::from_rgb(241, 95, 108) }
                                            else { Color32::from_rgb(87, 145, 248) };
                                        let center = icon.center();
                                        if failed {
                                            for slope in [-1.0_f32, 1.0] {
                                                ui.painter().line_segment([
                                                    center + Vec2::new(-4.0, slope * 4.0),
                                                    center + Vec2::new(4.0, -slope * 4.0)],
                                                    Stroke::new(1.8, color));
                                            }
                                        } else {
                                            ui.painter().line_segment([
                                                center + Vec2::new(-5.0, 0.0),
                                                center + Vec2::new(-1.0, 4.0),
                                            ], Stroke::new(1.8, color));
                                            ui.painter().line_segment([
                                                center + Vec2::new(-1.0, 4.0),
                                                center + Vec2::new(6.0, -5.0),
                                            ], Stroke::new(1.8, color));
                                        }
                                        let text: String = line.chars().take(34).collect();
                                        ui.label(RichText::new(&text)
                                            .size(13.0).strong().color(TEXT));
                                    });
                                }
                                if agent.status == "Working" {
                                    ui.horizontal(|ui| {
                                        let (icon, _) = ui.allocate_exact_size(Vec2::splat(15.0),
                                            egui::Sense::hover());
                                        let center = icon.center();
                                        let blue = Color32::from_rgb(87, 145, 248);
                                        ui.painter().add(egui::Shape::convex_polygon(vec![
                                            center + Vec2::new(-4.0, -5.0),
                                            center + Vec2::new(-4.0, 5.0),
                                            center + Vec2::new(5.0, 0.0),
                                        ], blue, Stroke::NONE));
                                        ui.label(RichText::new("Thinking…")
                                            .size(13.0).strong().color(TEXT));
                                    });
                                }
                                if agent.events.is_empty() && agent.status != "Working" {
                                    label(ui, "Waiting for activity", 13.0, MUTED);
                                }
                            });
                            ui.add_space(4.0);
                            let full_message = agent.last_message.as_ref().unwrap_or(&agent.title);
                            let message = if decision_pending {
                                if self.desktop_codex_approval.is_some() {
                                    "Approval needed · Review in Bookland".to_owned()
                                } else { "Codex has a question · Review in Bookland".to_owned() }
                            } else {
                                let mut message: String = full_message.chars().take(104).collect();
                                if full_message.chars().count() > 104 { message.push('…'); }
                                message
                            };
                            ui.vertical(|ui| {
                                ui.add_space(27.0);
                                let bubble = Frame::new()
                                .fill(Color32::from_rgba_unmultiplied(215, 219, 226, 72))
                                .stroke(Stroke::new(0.8,
                                    Color32::from_rgba_unmultiplied(245, 247, 252, 120)))
                                .corner_radius(18)
                                .inner_margin(Margin::symmetric(12, 10))
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.set_width(200.0);
                                        ui.set_max_width(200.0);
                                        ui.set_min_height(58.0);
                                        ui.add_space(5.0);
                                        ui.add(egui::Label::new(RichText::new(message)
                                            .size(12.0).strong().color(TEXT))
                                            .halign(Align::Center).wrap());
                                    });
                                });
                                let hint = if decision_pending { "Review Codex request in Bookland" }
                                    else if agent.source == "Codex" { "Open Codex app" }
                                    else { "Latest activity" };
                                if bubble.response.interact(egui::Sense::click())
                                    .on_hover_text(hint).clicked() {
                                    if decision_pending {
                                        self.agent_chat_desktop = true;
                                        self.agent_chat_open = true;
                                    } else if agent.source == "Codex" {
                                        let _ = Command::new("open").args(["-b", "com.openai.codex"]).spawn();
                                    }
                                }
                            });
                        });
                    }
                } else {
                    ui.vertical_centered(|ui| label(ui,
                        "No local Codex, Claude Code, or Craft activity", 13.0, MUTED));
                }
            });
        if let Some(index) = gradient {
            ui.painter().set(index, agents_upper_shade(panel.response.rect));
        }
        self.glass_rects[0] = Some(panel.response.rect);
        self.glass_menu_below(ui, width);
    }
    fn compact_glass_codex_chat_panel(&mut self, ui: &mut egui::Ui, width: f32, height: f32) {
        let mut gradient = None;
        let panel = Frame::new()
            .fill(glass_surface_fill())
            .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 29, se: 29 })
            .shadow(egui::epaint::Shadow { offset: [0, 9], blur: 28, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 125) })
            .inner_margin(Margin::symmetric(20, 13))
            .show(ui, |ui| {
                gradient = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_width(width - 40.0);
                ui.set_min_height(height - 26.0);
                ui.add_space(25.0);
                ui.horizontal(|ui| {
                    let (icon, _) = ui.allocate_exact_size(Vec2::splat(18.0),
                        egui::Sense::hover());
                    paint_codex_logo(ui.painter(), icon.center(), 17.0,
                        Color32::from_rgb(87, 145, 248), self.codex_logo.as_ref());
                    ui.label(RichText::new("Codex").size(16.0).strong().color(TEXT));
                    ui.label(RichText::new(if self.prefs.codex_allow_edits {
                        "PROJECT EDITS"
                    } else { "READ ONLY" }).size(10.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new("×").frame(false))
                            .on_hover_text("Close island").clicked() {
                            self.expanded = false;
                            self.hover_blocked_until_exit = true;
                        }
                        if ui.add(egui::Button::new("Activity")
                            .fill(Color32::from_rgb(45, 46, 51))
                            .corner_radius(15)).clicked() {
                            self.agent_chat_open = false;
                        }
                    });
                });
                ui.add_space(9.0);
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(25, 29, 39, 75))
                    .stroke(Stroke::new(0.7,
                        Color32::from_rgba_unmultiplied(245, 247, 252, 65)))
                    .corner_radius(16).inner_margin(Margin::symmetric(12, 9))
                    .show(ui, |ui| {
                        ui.set_width(width - 64.0);
                        ui.set_min_height(120.0);
                        ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                            egui::ScrollArea::vertical().id_salt("bookland_compact_codex_chat")
                            .max_height(135.0).stick_to_bottom(true).show(ui, |ui| {
                                if self.codex_messages.is_empty() {
                                    ui.add_space(42.0);
                                    ui.vertical_centered(|ui| label(ui,
                                        "Ask Codex from the island", 13.0, MUTED));
                                } else {
                                    for message in &self.codex_messages {
                                        let role_color = if message.role == "Codex" {
                                            Color32::from_rgb(87, 145, 248)
                                        } else { MUTED };
                                        ui.label(RichText::new(&message.role)
                                            .size(11.0).strong().color(role_color));
                                        ui.add(egui::Label::new(RichText::new(&message.text)
                                            .size(12.0).color(TEXT)).wrap());
                                        ui.add_space(6.0);
                                    }
                                }
                            });
                        });
                    });
                if !self.codex_status.is_empty() {
                    ui.add_space(4.0);
                    let status: String = self.codex_status.chars().take(140).collect();
                    ui.add(egui::Label::new(RichText::new(status).size(11.0)
                        .color(MUTED))
                        .wrap());
                }
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    let input = ui.add(egui::TextEdit::multiline(&mut self.codex_draft)
                        .id_salt("bookland_compact_codex_prompt")
                        .hint_text("Message Codex…")
                        .desired_rows(2).desired_width(width - 126.0));
                    let shortcut = input.has_focus() && ui.input(|input|
                        input.modifiers.command && input.key_pressed(egui::Key::Enter));
                    let ready = self.codex_rx.is_none() && !self.codex_draft.trim().is_empty();
                    if ui.add_enabled(ready, egui::Button::new("Send ↗")
                        .fill(Color32::from_rgb(36, 93, 67))
                        .corner_radius(12)).clicked() || (ready && shortcut) {
                        self.send_codex();
                    }
                });
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    if self.codex_rx.is_some() {
                        if ui.add_enabled(self.codex_pid.is_some() && !self.codex_stopping,
                            egui::Button::new("Stop")).clicked() {
                            if let Some(pid) = self.codex_pid {
                                match codex_bridge::stop(pid) {
                                    Ok(()) => {
                                        self.codex_stopping = true;
                                        self.codex_status = "Stopping Codex…".into();
                                    }
                                    Err(error) => self.codex_status = error,
                                }
                            }
                        }
                    } else if ui.add(egui::Button::new("New chat")
                        .fill(Color32::from_rgba_unmultiplied(42, 45, 53, 160))
                        .corner_radius(12)).clicked() {
                        self.codex_session_id = None;
                        self.codex_messages.clear();
                        self.codex_status.clear();
                        self.save_data();
                    }
                    if ui.add(egui::Button::new("Open Codex app")
                        .fill(Color32::from_rgba_unmultiplied(42, 45, 53, 160))
                        .corner_radius(12)).clicked() {
                        let to_copy = if self.codex_draft.trim().is_empty() {
                            self.codex_messages.iter().rev()
                                .find(|message| message.role == "You")
                                .map(|message| message.text.clone())
                        } else { Some(self.codex_draft.clone()) };
                        let copied = to_copy.is_some_and(|text|
                            arboard::Clipboard::new().and_then(|mut clipboard|
                                clipboard.set_text(text)).is_ok());
                        let _ = Command::new("open").args(["-b", "com.openai.codex"]).spawn();
                        self.notify(if copied { "Codex opened; paste the copied draft" }
                            else { "Codex opened" });
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        label(ui, "⌘ Return to send", 10.0,
                            MUTED);
                    });
                });
            });
        if let Some(index) = gradient {
            ui.painter().set(index, agents_upper_shade(panel.response.rect));
        }
        self.glass_rects[0] = Some(panel.response.rect);
        self.glass_menu_below(ui, width);
    }
    fn compact_desktop_codex_chat_panel(&mut self, ui: &mut egui::Ui, width: f32, height: f32) {
        let preview = self.agents.iter().find(|agent|
            desktop_codex_thread_id(&agent.path) == self.desktop_codex_thread_id)
            .and_then(|agent| agent.last_message.clone());
        let busy = self.desktop_codex_busy();
        let approval = self.desktop_codex_approval.clone();
        let question = self.desktop_codex_question.clone();
        let mut gradient = None;
        let panel = Frame::new()
            .fill(glass_surface_fill())
            .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 29, se: 29 })
            .shadow(egui::epaint::Shadow { offset: [0, 9], blur: 28, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 125) })
            .inner_margin(Margin::symmetric(20, 13))
            .show(ui, |ui| {
                gradient = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_width(width - 40.0);
                ui.set_min_height(height - 26.0);
                ui.add_space(22.0);
                ui.horizontal(|ui| {
                    let (icon, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                    paint_codex_logo(ui.painter(), icon.center(), 17.0,
                        Color32::from_rgb(87, 145, 248), self.codex_logo.as_ref());
                    ui.label(RichText::new("Codex").size(16.0).strong().color(TEXT));
                    ui.label(RichText::new(if self.desktop_codex_allow_edits {
                        "DESKTOP CHAT · EDIT MODE"
                    } else { "DESKTOP CHAT · READ ONLY" }).size(10.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new("×").frame(false))
                            .on_hover_text("Close island").clicked() {
                            self.expanded = false;
                            self.hover_blocked_until_exit = true;
                        }
                        if ui.add(egui::Button::new("Activity")
                            .fill(Color32::from_rgb(45, 46, 51))
                            .corner_radius(15)).clicked() { self.agent_chat_open = false; }
                    });
                });
                ui.add_space(8.0);
                Frame::new().fill(Color32::from_rgba_unmultiplied(25, 29, 39, 75))
                    .stroke(Stroke::new(0.7,
                        Color32::from_rgba_unmultiplied(245, 247, 252, 65)))
                    .corner_radius(16).inner_margin(Margin::symmetric(12, 9))
                    .show(ui, |ui| {
                        ui.set_width(width - 64.0);
                        egui::ScrollArea::vertical().id_salt("bookland_desktop_codex_chat")
                            .max_height(if question.is_some() { 180.0 } else { 100.0 })
                            .stick_to_bottom(question.is_none()).show(ui, |ui| {
                                if let Some(request) = &question {
                                    for (index, item) in request.questions.iter().enumerate() {
                                        if index > 0 { ui.separator(); }
                                        if !item.header.is_empty() {
                                            ui.label(RichText::new(&item.header)
                                                .size(11.0).strong().color(ACCENT));
                                        }
                                        ui.add(egui::Label::new(RichText::new(&item.prompt)
                                            .size(12.0).color(TEXT)).wrap());
                                        for (option, description) in &item.options {
                                            ui.radio_value(&mut self.desktop_codex_question_answers[index],
                                                option.clone(), option);
                                            if !description.is_empty() {
                                                label(ui, description, 10.0, MUTED);
                                            }
                                        }
                                        if item.allow_other || item.options.is_empty() {
                                            ui.add(egui::TextEdit::singleline(
                                                &mut self.desktop_codex_question_answers[index])
                                                .id_salt(("bookland_question", &item.id))
                                                .password(item.secret)
                                                .hint_text(if item.secret { "Private answer" }
                                                    else { "Type an answer…" }));
                                        }
                                    }
                                } else if let Some(request) = &approval {
                                    ui.label(RichText::new(&request.title)
                                        .size(12.0).strong().color(ACCENT));
                                    ui.add(egui::Label::new(RichText::new(&request.detail)
                                        .size(12.0).color(TEXT)).wrap());
                                } else if self.desktop_codex_messages.is_empty() {
                                    ui.label(RichText::new(&self.desktop_codex_title)
                                        .size(12.0).strong().color(ACCENT));
                                    if let Some(message) = &preview {
                                        ui.add(egui::Label::new(RichText::new(message)
                                            .size(12.0).color(TEXT)).wrap());
                                    } else { label(ui, "Continue this Codex chat from the island", 12.0, MUTED); }
                                } else {
                                    for message in &self.desktop_codex_messages {
                                        let color = if message.role == "Codex" { ACCENT } else { MUTED };
                                        ui.label(RichText::new(&message.role).size(11.0)
                                            .strong().color(color));
                                        ui.add(egui::Label::new(RichText::new(&message.text)
                                            .size(12.0).color(TEXT)).wrap());
                                        ui.add_space(5.0);
                                    }
                                }
                            });
                    });
                ui.add_space(5.0);
                if let Some(request) = &question {
                    ui.horizontal(|ui| {
                        let ready = !request.is_blocking
                            || self.desktop_codex_question_answers.iter()
                                .all(|answer| !answer.trim().is_empty());
                        if ui.add_enabled(ready, egui::Button::new("Submit answer")
                            .fill(Color32::from_rgb(36, 93, 67))).clicked() {
                            let answers = request.questions.iter()
                                .zip(&self.desktop_codex_question_answers)
                                .map(|(item, answer)| (item.id.clone(), answer.clone()))
                                .collect();
                            if let Some(tx) = &self.desktop_codex_decision_tx {
                                let _ = tx.send(codex_desktop::Decision::Answer(answers));
                            }
                            self.desktop_codex_question = None;
                            self.desktop_codex_question_answers.clear();
                        }
                        if !request.is_blocking && ui.button("Skip").clicked() {
                            if let Some(tx) = &self.desktop_codex_decision_tx {
                                let _ = tx.send(codex_desktop::Decision::Answer(Vec::new()));
                            }
                            self.desktop_codex_question = None;
                            self.desktop_codex_question_answers.clear();
                        }
                    });
                }
                else if approval.is_some() {
                    ui.horizontal(|ui| {
                        label(ui, "Allow this action?", 11.0, MUTED);
                        if ui.add(egui::Button::new("Allow once")
                            .fill(Color32::from_rgb(36, 93, 67))).clicked() {
                            if let Some(tx) = &self.desktop_codex_decision_tx {
                                let _ = tx.send(codex_desktop::Decision::AllowOnce);
                            }
                            self.desktop_codex_approval = None;
                        }
                        if ui.button("Deny").clicked() {
                            if let Some(tx) = &self.desktop_codex_decision_tx {
                                let _ = tx.send(codex_desktop::Decision::Deny);
                            }
                            self.desktop_codex_approval = None;
                        }
                    });
                }
                else if busy {
                    ui.horizontal_wrapped(|ui| {
                        label(ui, "This desktop turn is active. Review its requests in Codex.",
                            11.0, MUTED);
                        if ui.small_button("Open Codex ↗")
                            .on_hover_text("Show the Codex app to review its active turn")
                            .clicked() {
                            let _ = Command::new("open")
                                .args(["-b", "com.openai.codex"]).spawn();
                        }
                    });
                }
                else if !self.desktop_codex_status.is_empty() {
                    let status: String = self.desktop_codex_status.chars().take(125).collect();
                    ui.add(egui::Label::new(RichText::new(status).size(11.0).color(MUTED)).wrap());
                }
                if approval.is_none() && question.is_none() {
                    ui.add_space(7.0);
                    ui.horizontal(|ui| {
                        let input = ui.add(egui::TextEdit::singleline(&mut self.desktop_codex_draft)
                            .id_salt("bookland_desktop_codex_prompt")
                            .hint_text("Reply to this Codex chat…")
                            .desired_width(width - 150.0));
                        let shortcut = input.has_focus() && ui.input(|input|
                            input.key_pressed(egui::Key::Enter));
                        let ready = !busy && self.desktop_codex_rx.is_none()
                            && !self.desktop_codex_draft.trim().is_empty();
                        if ui.add_enabled(ready, egui::Button::new("Send ↗")
                            .fill(Color32::from_rgb(36, 93, 67))
                            .corner_radius(12)).clicked() || (ready && shortcut) {
                            self.send_desktop_codex();
                        }
                    });
                }
                ui.add_space(5.0);
                if approval.is_none() && question.is_none() {
                ui.horizontal(|ui| {
                    if self.desktop_codex_rx.is_some() {
                        if ui.add_enabled(self.desktop_codex_pid.is_some() && !self.desktop_codex_stopping,
                            egui::Button::new("Stop")).clicked() {
                            if let Some(pid) = self.desktop_codex_pid {
                                if let Some(tx) = &self.desktop_codex_decision_tx {
                                    let _ = tx.send(codex_desktop::Decision::Deny);
                                }
                                match codex_bridge::stop(pid) {
                                    Ok(()) => { self.desktop_codex_stopping = true;
                                        self.desktop_codex_status = "Stopping Codex…".into(); }
                                    Err(error) => self.desktop_codex_status = error,
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Bookland chat")
                        .fill(Color32::from_rgba_unmultiplied(42, 45, 53, 160))
                        .corner_radius(12)).clicked() { self.agent_chat_desktop = false; }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.add_enabled(self.desktop_codex_rx.is_none(),
                            egui::Checkbox::new(&mut self.desktop_codex_allow_edits,
                                RichText::new("Allow edits")
                                    .color(MUTED)))
                            .on_hover_text("This lets Codex edit its selected workspace. Other actions ask here first.");
                    });
                });
                }
            });
        if let Some(index) = gradient {
            ui.painter().set(index, agents_upper_shade(panel.response.rect));
        }
        self.glass_rects[0] = Some(panel.response.rect);
        self.glass_menu_below(ui, width);
    }
    fn glass_panel(&mut self, ui: &mut egui::Ui) {
        let (width, height) = self.open_dimensions();
        if self.workspace == Workspace::Agents {
            if self.agent_chat_open {
                if self.agent_chat_desktop {
                    self.compact_desktop_codex_chat_panel(ui, width, height);
                } else { self.compact_glass_codex_chat_panel(ui, width, height); }
            } else {
                self.compact_glass_agents_panel(ui, width, height);
            }
            return;
        }
        if self.workspace == Workspace::Nook && self.prefs.widgets == Widget::DEFAULT
            && self.prefs.nook_enabled && height <= 260.0 {
            self.compact_glass_panel(ui, width, height);
            return;
        }
        if self.workspace == Workspace::Music {
            self.compact_glass_music_panel(ui, width, height);
            return;
        }
        if self.workspace != Workspace::Nook {
            self.compact_glass_workspace_panel(ui, width, height);
            return;
        }
        let (notch, _) = ui.allocate_exact_size(
            Vec2::new(180.0, 38.0),
            egui::Sense::hover());
        ui.painter().rect_filled(notch, CornerRadius { nw: 0, ne: 0, sw: 18, se: 18 }, BLACK);
        self.open_notch_rect = Some(notch);
        let mut body_sheen = None;
        let body = Frame::new()
            .fill(if self.screenshot.is_some() {
                Color32::from_rgba_unmultiplied(16, 19, 25, 224)
            } else {
                Color32::from_rgba_unmultiplied(11, 14, 20, 122)
            })
            .corner_radius(29)
            .stroke(Stroke::new(0.9, Color32::from_rgba_unmultiplied(246, 249, 255, 112)))
            .shadow(egui::epaint::Shadow { offset: [0, 9], blur: 32, spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 155) })
            .inner_margin(Margin::same(18))
            .show(ui, |ui| {
                body_sheen = Some(ui.painter().add(egui::Shape::Noop));
                ui.set_width(width - 36.0);
                let compact_header = self.workspace == Workspace::Agents;
                let (header, _) = ui.allocate_exact_size(Vec2::new(width - 36.0,
                    if compact_header { 34.0 } else { 42.0 }), egui::Sense::hover());
                ui.painter().text(header.center(), Align2::CENTER_CENTER, "Bookland",
                    FontId::proportional(if compact_header { 17.0 } else { 21.0 }), TEXT);
                if ui.put(egui::Rect::from_min_size(
                    egui::pos2(header.right() - 60.0, header.top() + 4.0), Vec2::splat(32.0)),
                    egui::Button::new(RichText::new("⚙").size(17.0).color(TEXT)).frame(false))
                    .on_hover_text("Settings").clicked() { self.settings_open = true; }
                if ui.put(egui::Rect::from_min_size(
                    egui::pos2(header.right() - 30.0, header.top() + 4.0), Vec2::splat(32.0)),
                    egui::Button::new(RichText::new("×").size(20.0).color(TEXT)).frame(false))
                    .on_hover_text("Close island").clicked() {
                        self.expanded = false;
                        self.hover_blocked_until_exit = true;
                    }
                ui.add_space(if compact_header { 3.0 } else { 10.0 });
                if self.workspace == Workspace::Nook {
                    if !self.prefs.nook_enabled {
                        label(ui, "Nook is disabled in Settings.", 14.0, MUTED);
                    } else if self.prefs.widgets == Widget::DEFAULT {
                        self.glass_nook(ui, (height - 100.0).max(200.0));
                    } else {
                        egui::ScrollArea::vertical().max_height((height - 85.0).max(180.0))
                            .show(ui, |ui| self.nook(ui));
                    }
                } else {
                    egui::ScrollArea::vertical().max_height((height - 85.0).max(180.0))
                        .show(ui, |ui| match self.workspace {
                            Workspace::Nook => {},
                            Workspace::Music => self.music(ui),
                            Workspace::Calendar => self.calendar(ui),
                            Workspace::Tray => self.tray(ui),
                            Workspace::Clipboard => self.clipboard_browser(ui, false),
                            Workspace::Agents => self.agents(ui),
                            Workspace::Prompts => self.sheet_prompt_builder(ui),
                            Workspace::Prompter => self.prompter_workspace(ui),
                        });
                }
                if let Some((message, created)) = &self.toast {
                    if created.elapsed() < Duration::from_secs(3) {
                        ui.add_space(7.0); label(ui, message, 12.0, ACCENT);
                    }
                }
            });
        if let Some(idx) = body_sheen { ui.painter().set(idx, glass_sheen(body.response.rect, 38)); }
        self.glass_rects[0] = Some(body.response.rect);
        self.glass_menu_below(ui, width);
    }
    fn workspace_menu(&mut self, ui: &mut egui::Ui) {
        let workspaces = self.visible_workspaces();
        let menu = Frame::new()
            .fill(if self.prefs.style == "Glass" && self.screenshot.is_none() {
                glass_surface_fill()
            } else { BLACK })
            .corner_radius(24)
            .stroke(Stroke::new(0.8, Color32::from_rgba_unmultiplied(235, 238, 255, 58)))
            .inner_margin(Margin::symmetric(9, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for workspace in workspaces {
                        let active = self.menu_workspace_active(workspace);
                        let menu_name = if workspace == Workspace::Nook {
                            "Bookland"
                        } else { workspace.name() };
                        let labeled = active
                            && self.prefs.menu_labels
                            && self.prefs.menu_style != "Icons"
                            && self.prefs.menu_style != "Dots";
                        let text = if labeled {
                            format!("   {}", menu_name)
                        } else {
                            String::new()
                        };
                        let response = ui
                            .add(
                                egui::Button::new(
                                    RichText::new(text).size(15.0).color(if active {
                                        TEXT
                                    } else {
                                        MUTED
                                    }),
                                )
                                .fill(if active {
                                    Color32::from_rgb(58, 59, 65)
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .corner_radius(if self.prefs.rounded_buttons { 20 } else { 4 })
                                .min_size(Vec2::new(if labeled { 92.0 } else { 42.0 }, 34.0)),
                            )
                            .on_hover_text(if self.workspace == workspace {
                                "Collapse island"
                            } else { menu_name });
                        paint_workspace_icon(
                            ui,
                            response.rect,
                            workspace,
                            self.bookland_mark.as_ref(),
                            if active { TEXT } else { MUTED },
                            self.prefs.menu_style == "Dots",
                        );
                        if response.clicked() {
                            if self.workspace == workspace {
                                self.expanded = false;
                                self.opened_by_hover = false;
                                self.hover_blocked_until_exit = true;
                                self.clipboard_bottom_sheet = false;
                            } else {
                                self.workspace = workspace;
                                if workspace == Workspace::Clipboard {
                                    self.clipboard_bottom_sheet = self.prefs.clipboard_open_bottom;
                                    self.clipboard_focus_pending = self.prefs.clipboard_focus_search;
                                }
                            }
                        }
                    }
                });
            });
        self.glass_rects[1] = Some(menu.response.rect);
    }
    fn panel(&mut self, ui: &mut egui::Ui) {
        if self.workspace == Workspace::Clipboard && self.clipboard_bottom_sheet
            && self.screenshot.is_none() {
            let shelf = Frame::new()
                .fill(Color32::from_rgba_unmultiplied(7, 8, 12, 250))
                .corner_radius(24)
                .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 218, 236, 80)))
                .inner_margin(Margin::symmetric(18, 14))
                .show(ui, |ui| {
                    ui.set_width(self.clipboard_shelf_width - 36.0);
                    ui.set_min_height(195.0);
                    self.clipboard_browser(ui, false);
                });
            self.glass_rects[0] = Some(shelf.response.rect);
            return;
        }
        if self.prefs.style == "Glass" {
            self.glass_panel(ui);
            return;
        }
        let width = self.prefs.open_width.clamp(420.0, 700.0);
        let padding = self.prefs.content_padding.clamp(8.0, 30.0) as i8;
        let header = Frame::new()
            .fill(BLACK)
            .inner_margin(Margin::symmetric(20, 13))
            .show(ui, |ui| {
                ui.set_width(width - 40.0);
                ui.set_min_height(89.0);
                ui.horizontal(|ui| {
                    label(ui, "BOOKLAND", 10.0, MUTED);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("×").size(17.0).color(TEXT))
                                    .frame(false),
                            )
                            .on_hover_text("Close island")
                            .clicked()
                        {
                            self.expanded = false;
                            self.hover_blocked_until_exit = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("⚙").size(16.0).color(TEXT))
                                    .frame(false),
                            )
                            .on_hover_text("Settings")
                            .clicked()
                        {
                            self.settings_open = true;
                        }
                    });
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    label(ui, self.workspace.name(), 23.0, TEXT);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        label(
                            ui,
                            &run_output("date", &["+%a, %b %e"]).unwrap_or_default(),
                            12.0,
                            MUTED,
                        );
                    });
                });
                if self.prefs.menu_style == "In the notch" {
                    ui.add_space(8.0);
                    self.workspace_menu(ui);
                }
            });
        self.open_notch_rect = Some(header.response.rect);
        let lower = if self.prefs.style == "Classic" || self.screenshot.is_some() {
            Color32::from_rgb(31, 32, 38)
        } else {
            glass_surface_fill()
        };
        let lower_frame = Frame::new()
            .fill(lower)
            .stroke(Stroke::new(0.8, Color32::from_rgba_unmultiplied(235, 240, 255, 60)))
            .corner_radius(CornerRadius {
                nw: 0,
                ne: 0,
                sw: 26,
                se: 26,
            })
            .inner_margin(Margin::same(padding))
            .show(ui, |ui| {
                ui.set_width(width - 2.0 * padding as f32);
                ui.set_min_height((self.prefs.open_height - 118.0).max(350.0));
                egui::ScrollArea::vertical()
                    .max_height((self.prefs.open_height - 155.0).max(320.0))
                    .show(ui, |ui| match self.workspace {
                        Workspace::Nook => self.nook(ui),
                        Workspace::Music => self.music(ui),
                        Workspace::Calendar => self.calendar(ui),
                        Workspace::Tray => self.tray(ui),
                        Workspace::Clipboard => self.clipboard_browser(ui, false),
                        Workspace::Agents => self.agents(ui),
                        Workspace::Prompts => self.sheet_prompt_builder(ui),
                        Workspace::Prompter => self.prompter_workspace(ui),
                    });
                if let Some((message, created)) = &self.toast {
                    if created.elapsed() < Duration::from_secs(3) {
                        ui.add_space(7.0);
                        label(ui, message, 12.0, ACCENT);
                    }
                }
            });
        self.glass_rects[0] = Some(lower_frame.response.rect);
        if self.prefs.menu_style != "In the notch" {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - 310.0) / 2.0).max(0.0));
                self.workspace_menu(ui);
            });
        }
    }
    fn closed_notch(&mut self, ui: &mut egui::Ui) {
        let (width, height) = self.closed_dimensions();
        let left_shape = ui.painter().add(egui::Shape::Noop);
        let right_shape = ui.painter().add(egui::Shape::Noop);
        let bridge_shape = ui.painter().add(egui::Shape::Noop);
        let response = Frame::new()
            .fill(Color32::TRANSPARENT)
            .corner_radius(CornerRadius {
                nw: 10,
                ne: 10,
                sw: 17,
                se: 17,
            })
            .inner_margin(Margin::symmetric(15, 4))
            .show(ui, |ui| {
                ui.set_width(width - 30.0);
                ui.set_min_height(height - 8.0);
                if let Some((activity, _)) = &self.airpods_activity {
                    ui.horizontal(|ui| {
                        label(ui, "●", 11.0, Color32::from_rgb(125, 222, 164));
                        label(ui, activity, 12.0, TEXT);
                    });
                } else if self.pocket_job.is_some() {
                    let (strip, _) = ui.allocate_exact_size(
                        Vec2::new(width - 30.0, height - 8.0), egui::Sense::hover());
                    let right = strip.right_center() - Vec2::new(8.0, 0.0);
                    self.native_ring_center = Some(right);
                    ui.painter().text(strip.left_center() + Vec2::new(8.0, 0.0),
                        Align2::LEFT_CENTER, "Working…", FontId::proportional(11.0), TEXT);
                } else if let Some((activity, success, _)) = &self.pocket_activity {
                    ui.horizontal(|ui| {
                        label(ui, if *success { "✓" } else { "!" }, 12.0,
                            if *success { Color32::from_rgb(125, 222, 164) }
                            else { Color32::from_rgb(249, 164, 120) });
                        label(ui, activity, 11.0, TEXT);
                    });
                } else if let Some(done) = &self.agent_completion {
                    ui.horizontal(|ui| {
                        label(ui, "✦", 13.0, ACCENT);
                        label(ui, &format!("{} finished", done.source), 12.0, TEXT);
                    });
                } else if self.timer_end.is_some() && self.prefs.pocket_mode != "Media" {
                    ui.horizontal(|ui| {
                        label(ui, "◷", 12.0, ACCENT);
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            label(ui, &format_time(self.remaining()), 12.0, TEXT);
                        });
                    });
                } else if self.prefs.pocket_mode == "Agents" ||
                    (self.prefs.pocket_mode == "Auto" && self.agents.iter().any(|agent| agent.status == "Working")) {
                    let working = self.agents.iter().find(|agent| agent.status == "Working");
                    if let Some(agent) = working {
                        let (strip, response) = ui.allocate_exact_size(
                            Vec2::new(width - 30.0, height - 8.0), egui::Sense::hover());
                        let left = egui::pos2(strip.left() + 7.0, strip.center().y);
                        let right = egui::pos2(strip.right() - 7.0, strip.center().y);
                        self.native_ring_center = Some(right);
                        let blue = Color32::from_rgb(75, 139, 251);
                        paint_codex_logo(ui.painter(), left, 15.0, blue,
                            self.codex_logo.as_ref());
                        if !self.native_installed || self.preview_mode || self.screenshot.is_some() {
                            let time = ui.input(|input| input.time) as f32;
                            let arc: Vec<_> = (0..=28).map(|step| {
                                let angle = time * 2.1
                                    + step as f32 / 28.0 * std::f32::consts::FRAC_PI_2;
                                right + Vec2::angled(angle) * 7.0
                            }).collect();
                            ui.painter().circle_stroke(right, 7.0,
                                Stroke::new(1.5, Color32::from_rgba_unmultiplied(52, 93, 156, 110)));
                            ui.painter().add(egui::Shape::line(arc, Stroke::new(2.0, blue)));
                        }
                        if self.airpods.name.is_some() {
                            let badge = self.airpods.compact_battery()
                                .unwrap_or_else(|| "AirPods".into());
                            ui.painter().text(strip.center(), Align2::CENTER_CENTER, badge,
                                FontId::proportional(10.0),
                                Color32::from_rgb(190, 223, 210));
                        }
                        response.on_hover_text(format!("{} working", agent.source));
                    } else {
                        label(ui, "No agent working", 12.0, MUTED);
                    }
                } else if self.prefs.pocket_mode == "Auto"
                    && (self.video_rx.is_some() || self.downloads.iter().any(|item| item.active)) {
                    let (strip, _) = ui.allocate_exact_size(
                        Vec2::new(width - 30.0, height - 8.0), egui::Sense::hover());
                    let left = strip.left_center() + Vec2::new(9.0, 0.0);
                    let right = strip.right_center() - Vec2::new(8.0, 0.0);
                    self.native_ring_center = Some(right);
                    ui.painter().text(left, Align2::LEFT_CENTER, "↓",
                        FontId::proportional(16.0), ACCENT);
                    if !self.native_installed || self.preview_mode || self.screenshot.is_some() {
                        let angle = ui.input(|input| input.time) as f32 * 2.1;
                        let arc = (0..=28).map(|step| right + Vec2::angled(
                            angle + step as f32 / 28.0 * std::f32::consts::FRAC_PI_2) * 7.0)
                            .collect::<Vec<_>>();
                        ui.painter().circle_stroke(right, 7.0,
                            Stroke::new(1.5, Color32::from_rgba_unmultiplied(52, 93, 156, 110)));
                        ui.painter().add(egui::Shape::line(arc,
                            Stroke::new(2.0, Color32::from_rgb(75, 139, 251))));
                    }
                    let progress = if self.video_rx.is_some() {
                        self.video_progress.percent
                            .filter(|value| value.is_finite())
                            .map(|value| format!("Saving video · {:.0}%", value.clamp(0.0, 100.0)))
                            .unwrap_or_else(|| "Saving video…".into())
                    } else {
                        self.downloads.iter().find(|item| item.active)
                            .and_then(|item| item.bytes_per_second)
                            .map(|speed| format!("Downloading · {}/s",
                                downloads::format_bytes(speed)))
                            .unwrap_or_else(|| "Downloading…".into())
                    };
                    ui.painter().text(strip.center(), Align2::CENTER_CENTER, progress,
                        FontId::proportional(11.0), TEXT);
                } else if self.prefs.pocket_mode == "Media" ||
                    (self.prefs.pocket_mode == "Auto" && self.media_app.is_some()) {
                    let airpods_badge = self.airpods.name.as_ref().map(|_|
                        self.airpods.compact_battery().unwrap_or_else(|| "AirPods".into()));
                    ui.horizontal(|ui| {
                        label(ui, "♫", 13.0, ACCENT);
                        let max_title = if airpods_badge.is_some() { 14 } else { 22 };
                        let title: String = self.media_title.chars().take(max_title).collect();
                        music_text(ui, &title, 12.0, TEXT, false, false);
                        if let Some(badge) = &airpods_badge {
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                label(ui, badge, 10.0, Color32::from_rgb(190, 223, 210));
                            });
                        }
                    });
                } else if self.prefs.pocket_mode == "Auto" && self.airpods.name.is_some() {
                    ui.horizontal(|ui| {
                        label(ui, "●", 10.0, Color32::from_rgb(125, 222, 164));
                        if let Some(airpods) = self.airpods.compact_label() {
                            label(ui, &airpods, 11.0, TEXT);
                        }
                    });
                } else {
                    let (strip, _) = ui.allocate_exact_size(
                        Vec2::new(width - 30.0, height - 8.0), egui::Sense::hover());
                    if let Some(mark) = &self.bookland_mark {
                        ui.painter().image(mark.id(), egui::Rect::from_center_size(
                            egui::pos2(strip.left() + 10.0, strip.center().y),
                            Vec2::splat(22.0)),
                            egui::Rect::from_min_max(egui::Pos2::ZERO,
                                egui::pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    if self.prefs.pocket_battery {
                        ui.painter().text(strip.right_center() - Vec2::new(8.0, 0.0),
                            Align2::RIGHT_CENTER, &self.battery,
                            FontId::proportional(11.0), MUTED);
                    }
                }
            })
            .response
            .interact(egui::Sense::click());
        let mut physical_width = 0.0;
        let mut physical_height = 0.0;
        #[cfg(target_os = "macos")]
        let has_physical_notch = !self.preview_mode && self.screenshot.is_none()
            && unsafe { nooklet_physical_notch(&mut physical_width, &mut physical_height) != 0 };
        #[cfg(not(target_os = "macos"))]
        let has_physical_notch = false;
        if has_physical_notch {
            let shoulder_width = ((response.rect.width() - physical_width as f32) * 0.5).max(0.0);
            if let Some([left, right]) = &self.notch_shoulders {
                let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
                ui.painter().set(left_shape, egui::Shape::image(left.id(),
                    egui::Rect::from_min_size(response.rect.min,
                        Vec2::new(shoulder_width, response.rect.height())), uv, Color32::WHITE));
                ui.painter().set(right_shape, egui::Shape::image(right.id(),
                    egui::Rect::from_min_size(egui::pos2(
                        response.rect.right() - shoulder_width, response.rect.top()),
                        Vec2::new(shoulder_width, response.rect.height())), uv, Color32::WHITE));
            }
            ui.painter().set(bridge_shape, egui::Shape::rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(response.rect.left() + shoulder_width, response.rect.top()),
                    egui::pos2(response.rect.right() - shoulder_width,
                        response.rect.bottom())), 0.0, BLACK));
        } else {
            ui.painter().set(left_shape, egui::Shape::rect_filled(response.rect,
                CornerRadius { nw: 10, ne: 10, sw: 17, se: 17 }, BLACK));
        }
        response.context_menu(|ui| {
            if ui.button("Open Nook").clicked() {
                self.workspace = Workspace::Nook;
                self.workspace_chosen_by_user = true;
                self.expanded = true;
                self.opened_by_hover = false;
                self.hover_blocked_until_exit = false;
                self.last_hovered = Instant::now();
                self.hover_opened_at = self.last_hovered;
                ui.close();
            }
            if ui.button("Float Pocket").clicked() {
                self.open_pocket_near_pointer(ui.ctx());
                ui.close();
            }
            if ui.button("Edit Workspace").clicked() {
                self.settings_section = "Nook".into();
                self.settings_open = true;
                ui.close();
            }
            if ui.button("Settings").clicked() {
                self.settings_open = true;
                ui.close();
            }
        });
        #[cfg(target_os = "macos")]
        let physically_hovered = unsafe { bookland_pointer_over_island_rect(
            response.rect.min.x as f64, response.rect.min.y as f64,
            response.rect.width() as f64, response.rect.height() as f64,
            0.0, true,
        ) };
        #[cfg(not(target_os = "macos"))]
        let physically_hovered = response.hovered();
        self.open_notch_rect = Some(response.rect);
        let hover_allowed = hover_allowed_after_close(
            &mut self.hover_blocked_until_exit, physically_hovered);
        if response.clicked() {
            let completion_clicked = if self.pocket_job.is_some() || self.pocket_activity.is_some() {
                false
            } else if let Some(done) = self.agent_completion.take() {
                self.workspace = Workspace::Agents;
                self.agent_chat_open = false;
                self.agent_usage_view = false;
                self.selected_agent = self.agents.iter()
                    .position(|agent| agent.path == done.path && agent.source == done.source)
                    .unwrap_or(0);
                true
            } else { false };
            if !completion_clicked { self.choose_hover_workspace(); }
            self.expanded = true;
            self.opened_by_hover = true;
            self.hover_blocked_until_exit = false;
            self.last_hovered = Instant::now();
            self.hover_opened_at = self.last_hovered;
        } else if self.prefs.open_on_hover && hover_allowed
            && self.screenshot.is_none()
            && !response.context_menu_opened() && !response.secondary_clicked()
            && ui.ctx().input(|input| !input.pointer.any_down())
        {
            self.open_from_hover();
        }
    }
    fn morphing_notch(&mut self, ui: &mut egui::Ui) {
        let (open_width, open_height) = self.open_dimensions();
        let (closed_width, closed_height) = self.closed_dimensions();
        let width = egui::lerp(
            closed_width..=open_width,
            self.transition,
        );
        let height = egui::lerp(
            closed_height..=open_height,
            self.transition,
        );
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
        let radius = (17.0 + self.transition * 12.0).round() as u8;
        let top_radius = ((1.0 - self.transition) * 10.0).round() as u8;
        let corners = CornerRadius { nw: top_radius, ne: top_radius,
            sw: radius, se: radius };
        let reveal = if self.prefs.style == "Glass" {
            ((self.transition - 0.82) / 0.14).clamp(0.0, 1.0)
        } else { 0.0 };
        let shell = Color32::from_rgba_unmultiplied(0, 0, 0,
            ((1.0 - reveal) * 255.0).round() as u8);
        if self.native_installed && !self.preview_mode && self.screenshot.is_none() {
            self.native_morph_shell = Some((rect, top_radius as f32,
                radius as f32, 1.0 - reveal));
        } else {
            ui.painter().rect_filled(rect, corners, shell);
        }
        self.glass_rects[0] = Some(rect);
        // Bring the actual workspace in before the shell finishes expanding.
        // Clip it to the moving outline so text and controls do not pop in on
        // the final frame, while leaving the native glass bound to that outline.
        if self.prefs.style == "Glass" && self.transition >= 0.82 {
            let full_rect = egui::Rect::from_min_size(
                egui::pos2(rect.center().x - open_width * 0.5, rect.top()),
                Vec2::new(open_width, open_height + 75.0),
            );
            let mut content = ui.new_child(egui::UiBuilder::new()
                .max_rect(full_rect).layout(egui::Layout::top_down(Align::Min)));
            content.set_clip_rect(rect.intersect(ui.clip_rect()));
            content.set_opacity(reveal);
            if !self.expanded || self.transition < 0.98 { content.disable(); }
            self.panel(&mut content);
            self.glass_rects[0] = Some(rect);
            self.glass_rects[1] = None;
            self.open_notch_rect = Some(rect);
        }
    }
    fn settings_nav(&mut self, ui: &mut egui::Ui) {
        let brand = self.bookland_mark.clone();
        Frame::new()
            .fill(Color32::from_rgb(26, 35, 52))
            .stroke(Stroke::new(0.8, Color32::from_rgb(78, 101, 139)))
            .corner_radius(16.0)
            .shadow(egui::epaint::Shadow { offset: [0, 5], blur: 16, spread: 0,
                color: Color32::from_black_alpha(75) })
            .inner_margin(13.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(42.0), egui::Sense::hover());
                    if let Some(mark) = &brand {
                        ui.painter().image(mark.id(), rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO,
                                egui::Pos2::new(1.0, 1.0)), Color32::WHITE);
                    } else {
                        ui.painter().rect_filled(rect, 10.0,
                            Color32::from_rgb(37, 66, 106));
                        ui.painter().text(rect.center(), Align2::CENTER_CENTER,
                            "B", FontId::proportional(23.0), TEXT);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Bookland").size(17.0).strong().color(TEXT));
                        label(ui, "A living island for Mac", 10.0,
                            Color32::from_rgb(173, 194, 226));
                    });
                });
            });
        ui.add_space(12.0);
        for (group, items) in [
            (
                "SETTINGS",
                vec![
                    "General", "Appearance", "Displays", "Behavior", "Pocket", "Permissions",
                ],
            ),
            (
                "WORKSPACES",
                vec![
                    "Nook", "Music", "Tray", "Clipboard", "Prompter", "Calendar", "Agents",
                ],
            ),
            (
                "FEATURES",
                vec![
                    "Live Activities", "Lock Screen", "Downloads", "Messages (beta)",
                    "Terminal", "Drop Area", "Keyboard Shortcuts", "About",
                ],
            ),
        ] {
            ui.add_space(10.0);
            label(ui, group, 10.0, MUTED);
            ui.add_space(5.0);
            for name in items {
                let selected = self.settings_section == name;
                let response = ui.add(egui::Button::new(
                    // Keep the name on the button for accessibility; paint it at a fixed
                    // offset so the badge cannot overlap longer section names.
                    RichText::new(name).size(12.5).color(Color32::TRANSPARENT))
                    .fill(if selected { Color32::from_rgb(48, 62, 91) }
                        else { Color32::TRANSPARENT })
                    .stroke(Stroke::new(if selected { 0.8 } else { 0.0 },
                        Color32::from_rgb(83, 107, 151)))
                    .corner_radius(9.0)
                    .min_size(Vec2::new(194.0, 29.0)));
                let badge = egui::Rect::from_center_size(
                    egui::pos2(response.rect.left() + 17.0, response.rect.center().y),
                    Vec2::splat(18.0));
                ui.painter().rect_filled(badge, 5.0,
                    if selected { Color32::from_rgb(92, 133, 203) }
                    else { Color32::from_rgb(55, 68, 88) });
                paint_settings_icon(ui.painter(), badge.center(), name,
                    Color32::from_rgb(243, 248, 255));
                ui.painter().text(
                    egui::pos2(response.rect.left() + 32.0, response.rect.center().y),
                    Align2::LEFT_CENTER,
                    name,
                    FontId::proportional(12.5),
                    if selected { TEXT } else { Color32::from_rgb(176, 180, 192) },
                );
                if response.clicked() { self.settings_section = name.into(); }
            }
        }
    }
    fn settings_content(&mut self, ui: &mut egui::Ui) {
        label(ui, "BOOKLAND  /  SETTINGS", 10.0, Color32::from_rgb(132, 170, 230));
        ui.add_space(7.0);
        ui.label(RichText::new(&self.settings_section).size(27.0).strong().color(TEXT));
        let description = match self.settings_section.as_str() {
            "General" => "Startup, media source, and everyday behavior.",
            "Appearance" => "Shape, glass, and motion for your island.",
            "Displays" => "Choose where Bookland appears.",
            "Behavior" => "Tune how the island opens and closes.",
            "Permissions" => "Control access to Mac features.",
            "Music" => "Playback, lyrics, audio, and AirPods.",
            "Nook" => "Arrange the controls inside your island.",
            "Calendar" => "Your calendar connection and date display.",
            "Agents" => "Codex activity and island decisions.",
            _ => "Make Bookland yours.",
        };
        label(ui, description, 11.0, MUTED);
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
        match self.settings_section.as_str() {
            "General" => {
                section_title(ui, "STARTUP");
                settings_card(ui, |ui| {
                    if ui.add_enabled(!self.preview_mode,
                        egui::Checkbox::new(&mut self.prefs.launch_at_login, "Launch at login"))
                        .on_hover_text("Takes effect at the next sign-in.").changed() {
                        if let Err(error) = set_launch_at_login(self.prefs.launch_at_login) {
                            self.prefs.launch_at_login = !self.prefs.launch_at_login;
                            self.notify(format!("Could not change login item: {error}"));
                        }
                    }
                    ui.checkbox(&mut self.prefs.open_on_hover, "Open island on hover");
                    ui.checkbox(&mut self.prefs.start_expanded, "Start expanded");
                });
                ui.add_space(13.0);
                section_title(ui, "MEDIA SOURCE");
                settings_card(ui, |ui| {
                    egui::ComboBox::from_id_salt("media_source")
                        .selected_text(&self.prefs.media_source)
                        .show_ui(ui, |ui| {
                            for source in ["System", "Music", "Spotify"] {
                                ui.selectable_value(
                                    &mut self.prefs.media_source,
                                    source.into(),
                                    source,
                                );
                            }
                        });
                });
                ui.add_space(13.0);
                section_title(ui, "DEMO");
                settings_card(ui, |ui| {
                    if small_button(ui, "Open island").clicked() {
                        self.expanded = true;
                        self.opened_by_hover = false;
                    }
                    ui.checkbox(&mut self.prefs.demo_mode, "Demo mode")
                        .on_hover_text("Shows labeled sample activity in the Agents workspace.");
                    if small_button(ui, "Onboarding tour").clicked() {
                        self.tour_step = Some(0);
                    }
                });
            }
            "Appearance" => {
                section_title(ui, "STYLE");
                settings_card(ui, |ui| {
                    ui.horizontal(|ui| {
                        for style in ["Glass", "Classic"] {
                            ui.selectable_value(&mut self.prefs.style, style.into(), style);
                        }
                    });
                    ui.add(
                        egui::Slider::new(&mut self.prefs.glass_level, 0.0..=1.0)
                            .text("Glass level"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.prefs.animation_speed, 0.5..=2.0)
                            .text("Animation speed"),
                    );
                });
                ui.add_space(12.0);
                section_title(ui, "WORKSPACE MENU");
                settings_card(ui, |ui| {
                    egui::ComboBox::from_id_salt("menu_style")
                        .selected_text(if self.prefs.menu_style == "In the notch" {
                            "In the island"
                        } else { &self.prefs.menu_style })
                        .show_ui(ui, |ui| {
                            for style in ["Compact", "Icons", "In the notch", "Dots"] {
                                ui.selectable_value(
                                    &mut self.prefs.menu_style,
                                    style.into(),
                                    if style == "In the notch" { "In the island" } else { style },
                                );
                            }
                        });
                    ui.checkbox(&mut self.prefs.menu_labels, "Show menu labels");
                    ui.checkbox(&mut self.prefs.show_extra_workspaces,
                        "Show Clipboard and Prompts in island menu");
                });
                ui.add_space(12.0);
                section_title(ui, "ISLAND SIZE");
                settings_card(ui, |ui| {
                    let glass = self.prefs.style == "Glass";
                    ui.add(egui::Slider::new(&mut self.prefs.open_width,
                        if glass { 900.0..=1200.0 } else { 420.0..=700.0 }).text("Open width"));
                    ui.add(egui::Slider::new(&mut self.prefs.open_height,
                        if glass { 170.0..=550.0 } else { 530.0..=780.0 }).text("Open height"));
                    ui.add(
                        egui::Slider::new(&mut self.prefs.closed_width, 180.0..=400.0)
                            .text("Closed width"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.prefs.closed_height, 30.0..=70.0)
                            .text("Closed height"),
                    );
                    ui.add_enabled(
                        false,
                        egui::Checkbox::new(
                            &mut self.prefs.match_resolution,
                            "Match display resolution",
                        ),
                    )
                    .on_hover_text("Exact display mapping is not yet verified.");
                });
                ui.add_space(12.0);
                section_title(ui, "DETAILS");
                settings_card(ui, |ui| {
                    ui.checkbox(&mut self.prefs.rounded_buttons, "Rounded buttons");
                    ui.add(
                        egui::Slider::new(&mut self.prefs.content_padding, 8.0..=30.0)
                            .text("Content padding"),
                    );
                });
            }
            "Displays" => {
                section_title(ui, "ISLAND DISPLAY");
                settings_card(ui, |ui| {
                    let displays = available_island_displays();
                    let selected = displays.iter()
                        .find(|display| display.id == self.prefs.island_display_id);
                    let selected_name = selected.map(|display| display.name.as_str())
                        .unwrap_or("Automatic (current display)");
                    egui::ComboBox::from_id_salt("island_display")
                        .selected_text(selected_name)
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.prefs.island_display_id, 0,
                                "Automatic (current display)").clicked() {
                                self.pocket_wake_context.request_repaint();
                            }
                            for display in &displays {
                                let kind = if display.notch { "camera notch" }
                                    else { "rounded island" };
                                let main = if display.main { " · Main" } else { "" };
                                let title = format!("{} · {}{}", display.name, kind, main);
                                if ui.selectable_value(&mut self.prefs.island_display_id,
                                    display.id, title).clicked() {
                                    self.pocket_wake_context.request_repaint();
                                }
                            }
                        });
                    label(ui, "Choose where Bookland sits. A screen without a camera notch gets a rounded island at its top edge.",
                        12.0, MUTED);
                    if self.prefs.island_display_id != 0 && selected.is_none() {
                        label(ui, "That display is disconnected. Bookland stays on the current screen until it returns.",
                            12.0, MUTED);
                    }
                });
            }
            "Behavior" => {
                settings_card(ui, |ui| {
                    ui.checkbox(&mut self.prefs.open_on_hover, "Always open on hover");
                    ui.checkbox(&mut self.prefs.haptic_feedback, "Haptic feedback");
                    ui.checkbox(&mut self.prefs.auto_close, "Close after the pointer leaves");
                    ui.add(egui::Slider::new(&mut self.prefs.close_delay_ms, 100..=3000)
                        .text("Close delay (ms)"));
                    ui.checkbox(&mut self.prefs.prevent_close_while_typing,
                        "Prevent closing on mouse leave while typing");
                    ui.checkbox(&mut self.prefs.lock_while_typing,
                        "Lock while typing");
                    ui.separator();
                    ui.checkbox(&mut self.prefs.gestures_on_hover,
                        "Allow gestures when hovering the island");
                    ui.add_enabled_ui(self.prefs.gestures_on_hover, |ui| {
                        ui.checkbox(&mut self.prefs.gesture_vertical,
                            "Open/close with vertical gestures");
                        ui.checkbox(&mut self.prefs.gesture_horizontal_media,
                            "Control media with horizontal gestures");
                        ui.add_enabled_ui(self.prefs.gesture_horizontal_media, |ui| {
                            ui.checkbox(&mut self.prefs.gesture_invert_media,
                                "Invert media gesture actions");
                        });
                    });
                    label(ui, "Clicking the island keeps it open until you close it.", 12.0, MUTED);
                });
            }
            "Pocket" => {
                settings_card(ui, |ui| {
                    ui.checkbox(&mut self.prefs.pocket_shake,
                        "Shake a dragged file to open floating Pocket");
                    label(ui, "Content in the closed island", 12.0, MUTED);
                    egui::ComboBox::from_id_salt("pocket_mode")
                        .selected_text(&self.prefs.pocket_mode)
                        .show_ui(ui, |ui| {
                            for choice in ["Auto", "Timer", "Media", "Agents", "None"] {
                                ui.selectable_value(&mut self.prefs.pocket_mode, choice.into(), choice);
                            }
                        });
                    ui.checkbox(&mut self.prefs.pocket_battery, "Show battery when idle");
                });
                ui.add_space(12.0);
                self.pipeline_editor(ui);
            }
            "Live Activities" => {
                settings_card(ui, |ui| {
                    ui.checkbox(&mut self.prefs.notify_timer, "Notify when Pomodoro finishes");
                    ui.checkbox(&mut self.prefs.volume_hud,
                        "Show a Liquid Glass volume HUD");
                    ui.add_enabled_ui(self.prefs.volume_hud, |ui| {
                        ui.horizontal(|ui| {
                            label(ui, "Volume placement", 12.0, MUTED);
                            egui::ComboBox::from_id_salt("volume_hud_style")
                                .selected_text(if self.prefs.volume_hud_style == "Inline by the notch" {
                                    "Inline beside island"
                                } else { "Below island" })
                                .show_ui(ui, |ui| {
                                    for choice in ["Below the notch", "Inline by the notch"] {
                                        ui.selectable_value(&mut self.prefs.volume_hud_style,
                                            choice.into(), if choice == "Below the notch" {
                                                "Below island"
                                            } else { "Inline beside island" });
                                    }
                                });
                        });
                    });
                    label(ui, "The running timer and current track can appear in the closed island. Configure that in Pocket.", 12.0, MUTED);
                });
            }
            "Nook" => {
                ui.checkbox(&mut self.prefs.nook_enabled, "Enable Nook workspace");
                ui.checkbox(&mut self.prefs.dividers, "Show widget dividers");
                if small_button(ui, "Restore defaults").clicked() {
                    self.prefs.widgets = Widget::DEFAULT.to_vec();
                    self.selected_widget = Widget::MediaPlayer;
                }
                ui.add_space(12.0);
                section_title(ui, "YOUR NOOK");
                label(ui, "Drag a widget to reorder it, click to edit, or remove it.", 12.0, MUTED);
                ui.add_space(8.0);
                let widgets = self.prefs.widgets.clone();
                Frame::new()
                    .fill(Color32::from_rgb(82, 41, 150))
                    .corner_radius(13)
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        Frame::new()
                            .fill(Color32::from_rgb(8, 8, 12))
                            .corner_radius(10)
                            .inner_margin(Margin::same(6))
                            .show(ui, |ui| {
                                egui::ScrollArea::horizontal()
                                    .id_salt("nook_widget_preview")
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            for widget in &widgets {
                                                let detail = match widget {
                                                    Widget::MediaPlayer => self.media_title.as_str(),
                                                    Widget::Pulse => "CPU · MEM · SSD",
                                                    Widget::Pomodoro => "25:00",
                                                    Widget::Todos => "Tasks",
                                                    Widget::Notes => "Quick notes",
                                                    Widget::Clipboard => "History",
                                                    Widget::Downloads => "Transfers",
                                                    Widget::Terminal => "Commands",
                                                    Widget::Weather => "Forecast",
                                                    Widget::Mirror => "Camera preview",
                                                };
                                                let selected = self.selected_widget == *widget;
                                                let response = ui.add(
                                                    egui::Button::new(format!("{}\n{}", widget.name(), detail))
                                                        .min_size(Vec2::new(116.0, 68.0))
                                                        .fill(if selected {
                                                            Color32::from_rgb(37, 34, 49)
                                                        } else {
                                                            Color32::from_rgb(17, 17, 22)
                                                        })
                                                        .corner_radius(8));
                                                if selected {
                                                    ui.painter().rect_stroke(response.rect, 8.0,
                                                        Stroke::new(2.0, Color32::from_rgb(44, 137, 255)),
                                                        egui::StrokeKind::Outside);
                                                }
                                                if response.clicked() { self.selected_widget = *widget; }
                                            }
                                        });
                                    });
                            });
                    });
                ui.add_space(8.0);
                let mut remove: Option<usize> = None;
                let mut rows = Vec::with_capacity(widgets.len());
                let pointer = ui.ctx().input(|input| input.pointer.interact_pos());
                for (i, widget) in widgets.iter().enumerate() {
                    let row = settings_card(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (handle_rect, handle) = ui.allocate_exact_size(
                                Vec2::new(20.0, 22.0), egui::Sense::drag());
                            for offset in [-5.0, 0.0, 5.0] {
                                ui.painter().line_segment([
                                    handle_rect.center() + Vec2::new(-6.0, offset),
                                    handle_rect.center() + Vec2::new(6.0, offset),
                                ], Stroke::new(1.5, MUTED));
                            }
                            let handle = handle.on_hover_text("Drag to reorder");
                            if handle.drag_started() {
                                self.dragged_widget = Some(*widget);
                            }
                            if ui
                                .selectable_label(
                                    self.selected_widget == *widget,
                                    format!("{}  {}", widget.icon(), widget.name()),
                                )
                                .clicked()
                            {
                                self.selected_widget = *widget;
                            }
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .small_button("×")
                                    .on_hover_text("Remove widget")
                                    .clicked()
                                {
                                    remove = Some(i);
                                }
                            });
                        });
                    });
                    if self.dragged_widget.is_some_and(|active| active == *widget)
                        || (self.dragged_widget.is_some()
                            && pointer.is_some_and(|pos| row.response.rect.contains(pos))) {
                        ui.painter().rect_stroke(row.response.rect, 9.0,
                            Stroke::new(1.5, ACCENT), egui::StrokeKind::Inside);
                    }
                    rows.push(row.response.rect);
                    ui.add_space(5.0);
                }
                if let Some(i) = remove {
                    let removed = self.prefs.widgets.remove(i);
                    if self.dragged_widget == Some(removed) { self.dragged_widget = None; }
                }
                if ui.ctx().input(|input| input.pointer.any_released()) {
                    if let Some(active) = self.dragged_widget.take() {
                        if let (Some(from), Some(to)) = (
                            self.prefs.widgets.iter().position(|widget| *widget == active),
                            pointer.and_then(|pos| rows.iter().position(|rect| rect.contains(pos))),
                        ) {
                            if from != to {
                                let widget = self.prefs.widgets.remove(from);
                                self.prefs.widgets.insert(to, widget);
                            }
                        }
                    }
                }
                let missing: Vec<_> = Widget::ALL
                    .into_iter()
                    .filter(|w| !self.prefs.widgets.contains(w))
                    .collect();
                if !missing.is_empty() {
                    ui.menu_button("+ Add widget", |ui| {
                        for widget in missing {
                            if ui.button(widget.name()).clicked() {
                                self.prefs.widgets.push(widget);
                                ui.close();
                            }
                        }
                    });
                }
                ui.add_space(10.0);
                section_title(ui, "SELECTED WIDGET");
                self.widget_settings(ui);
            }
            "Music" => {
                self.media_settings(ui);
            }
            "Clipboard" => {
                section_title(ui, "HISTORY");
                settings_card(ui, |ui| {
                    ui.checkbox(&mut self.prefs.capture_clipboard, "Capture automatically");
                    if ui.checkbox(&mut self.prefs.persist_clipboard,
                        "Save history, including images, on this Mac").changed() {
                        self.save_data();
                    }
                    if ui.add(egui::Slider::new(&mut self.prefs.clipboard_history_size, 25..=1000)
                        .text("History size")).changed() {
                        trim_clipboard_history(&mut self.clipboard, &self.clipboard_favorites,
                            self.prefs.clipboard_history_size);
                        trim_clipboard_files(&mut self.clipboard_files,
                            self.prefs.clipboard_history_size);
                        self.save_data();
                    }
                    ui.checkbox(&mut self.prefs.clipboard_clear_on_quit, "Clear non-favorites when quitting");
                    if small_button(ui, "Clear history now").clicked() {
                        self.clipboard.clear();
                        self.clipboard_images.clear();
                        self.clipboard_files.clear();
                        self.clipboard_favorites.clear();
                        self.last_clip.clear();
                        self.last_image_fingerprint = None;
                        self.save_data();
                    }
                });
                ui.add_space(12.0);
                section_title(ui, "SHELF");
                settings_card(ui, |ui| {
                    ui.horizontal(|ui| {
                        label(ui, "Open history", 12.0, MUTED);
                        if ui.selectable_value(&mut self.prefs.clipboard_open_bottom, false, "In the island").clicked() {
                            self.clipboard_bottom_sheet = false;
                        }
                        if ui.selectable_value(&mut self.prefs.clipboard_open_bottom, true, "Bottom shelf").clicked() {
                            self.clipboard_bottom_sheet = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        label(ui, "Card size", 12.0, MUTED);
                        for size in ["Small", "Medium", "Large"] {
                            ui.selectable_value(&mut self.prefs.clipboard_card_size, size.into(), size);
                        }
                    });
                    ui.checkbox(&mut self.prefs.clipboard_focus_search, "Focus search when opened");
                    ui.checkbox(&mut self.prefs.clipboard_hide_after_copy, "Hide bottom shelf after copying");
                });
                ui.add_space(12.0);
                section_title(ui, "PRIVACY");
                settings_card(ui, |ui| {
                    ui.checkbox(&mut self.prefs.clipboard_ignore_passwords,
                        "Ignore copies marked confidential or temporary");
                    label(ui, "Ignored apps", 12.0, MUTED);
                    let mut remove = None;
                    for (index, app_id) in self.prefs.clipboard_ignore_apps.iter().enumerate() {
                        ui.horizontal(|ui| {
                            label(ui, app_id, 12.0, TEXT);
                            if ui.small_button("Remove").clicked() { remove = Some(index); }
                        });
                    }
                    if let Some(index) = remove { self.prefs.clipboard_ignore_apps.remove(index); }
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.clipboard_app_draft)
                            .hint_text("App bundle ID, e.g. com.apple.Passwords")
                            .desired_width(300.0));
                        if ui.small_button("Add app").clicked() {
                            let id = self.clipboard_app_draft.trim();
                            if id.contains('.') && !id.chars().any(char::is_whitespace)
                                && !self.prefs.clipboard_ignore_apps.iter().any(|item| item == id) {
                                self.prefs.clipboard_ignore_apps.push(id.into());
                                self.clipboard_app_draft.clear();
                            } else { self.notify("Enter a unique app bundle ID"); }
                        }
                    });
                    label(ui, "When an ignored app is in front, new copies are skipped.", 11.0, MUTED);
                });
            }
            "Tray" | "Drop Area" => {
                settings_card(ui, |ui| {
                    label(ui, "Dropped files are saved locally and can be revealed in Finder.", 12.0, MUTED);
                    if small_button(ui, "Clear tray").clicked() {
                        self.files.clear();
                        self.save_data();
                    }
                });
            }
            "Prompter" => {
                settings_card(ui, |ui| {
                    ui.checkbox(&mut self.prefs.prompter_enabled, "Show Prompter in the island menu");
                    label(ui, "Mode", 12.0, MUTED);
                    ui.horizontal(|ui| {
                        for mode in ["Scroll", "Voice"] {
                            ui.selectable_value(&mut self.prefs.prompter_mode, mode.into(), mode);
                        }
                    });
                    ui.add(egui::Slider::new(&mut self.prefs.prompter_speed, 10.0..=120.0).text("Scroll speed"));
                    ui.add(egui::Slider::new(&mut self.prefs.prompter_text_size, 16.0..=42.0).text("Text size"));
                    label(ui, "Speech language (BCP 47)", 12.0, MUTED);
                    ui.add(egui::TextEdit::singleline(&mut self.prefs.prompter_language)
                        .hint_text("en-US, he-IL, ar-SA…").desired_width(180.0));
                    #[cfg(target_os = "macos")]
                    if let Ok(locale) = std::ffi::CString::new(self.prefs.prompter_language.trim()) {
                        let flags = unsafe { bookland_speech_capabilities(locale.as_ptr()) };
                        label(ui, if flags & 1 == 0 { "This language is not supported here" }
                            else if flags & 2 != 0 { "On-device recognition is available on this Mac" }
                            else { "This language requires Apple's speech service on this Mac" },
                            11.0, MUTED);
                    }
                    ui.checkbox(&mut self.prefs.prompter_allow_network,
                        "Allow Apple's speech service when on-device recognition is unavailable");
                    label(ui, "Voice requests microphone and Speech Recognition access when started. Network mode may send audio to Apple.", 11.0, MUTED);
                    label(ui, "Script", 12.0, MUTED);
                    #[cfg(target_os = "macos")]
                    if small_button(ui, "Load script…").clicked() {
                        let mut buffer = vec![0_u8; 1_000_001];
                        let length = unsafe { bookland_choose_prompter_script(
                            buffer.as_mut_ptr().cast(), buffer.len()) };
                        if length > 0 && length < buffer.len() {
                            if let Ok(script) = String::from_utf8(buffer[..length].to_vec()) {
                                self.prompter = script;
                                self.prompter_scroll = 0.0;
                                self.prompter_voice_word = 0;
                                self.save_data();
                            }
                        }
                    }
                    if ui.add(egui::TextEdit::multiline(&mut self.prompter)
                        .desired_rows(14).desired_width(f32::INFINITY)
                        .hint_text("Paste or write a script…")).changed() {
                        self.save_data();
                    }
                });
            }
            "Downloads" => {
                settings_card(ui, |ui| {
                    label(ui, "Watch active and recent files in a folder.", 12.0, MUTED);
                    if ui.checkbox(&mut self.prefs.watch_downloads,
                        "Show a result card when a download finishes").changed() {
                        self.downloads.clear();
                        self.last_download_poll = Instant::now() - Duration::from_secs(20);
                    }
                    let folder = downloads_root(&self.prefs.downloads_folder)
                        .unwrap_or_else(|| PathBuf::from("Downloads"));
                    label(ui, &folder.to_string_lossy(), 11.0, MUTED);
                    ui.horizontal(|ui| {
                        #[cfg(target_os = "macos")]
                        if small_button(ui, "Choose folder…").clicked() {
                            let mut buffer = vec![0_u8; 4096];
                            let length = unsafe { bookland_choose_folder(buffer.as_mut_ptr().cast(), buffer.len()) };
                            if length > 0 {
                                self.prefs.downloads_folder = String::from_utf8_lossy(&buffer[..length]).into_owned();
                                self.prefs.watch_downloads = true;
                                self.downloads.clear();
                                self.downloads_error = None;
                                self.last_download_poll = Instant::now() - Duration::from_secs(20);
                            }
                        }
                        if small_button(ui, "Use Downloads").clicked() {
                            self.prefs.downloads_folder.clear();
                            self.prefs.watch_downloads = true;
                            self.downloads.clear();
                            self.downloads_error = None;
                            self.last_download_poll = Instant::now() - Duration::from_secs(20);
                        }
                        if small_button(ui, "Reveal folder").clicked() {
                            let _ = Command::new("open").arg(&folder).spawn();
                        }
                    });
                    if let Some(error) = &self.downloads_error {
                        label(ui, &format!("Folder unavailable: {error}"), 11.0, MUTED);
                    }
                });
            }
            "Terminal" => {
                settings_card(ui, |ui| {
                    label(ui, "Add the Terminal widget in Nook settings. It runs only commands you type and click Run.", 12.0, MUTED);
                });
            }
            "Permissions" => {
                settings_card(ui, |ui| {
                    label(ui, &format!("Calendar: {}", if self.calendar_status == 1 { "Connected" } else if self.calendar_status == 2 { "Denied" } else { "Not connected" }), 13.0, TEXT);
                    if small_button(ui, "Open Privacy Settings").clicked() {
                        let _ = Command::new("open").arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars").spawn();
                    }
                });
            }
            "Keyboard Shortcuts" => {
                settings_card(ui, |ui| {
                    ui.checkbox(&mut self.prefs.global_shortcut, "Use Command+Option+N to open or close the island");
                    ui.checkbox(&mut self.prefs.clipboard_shortcut, "Use Control+Command+6 to open Clipboard from the bottom");
                    label(ui, "The shortcut works while another app is in front.", 12.0, MUTED);
                });
            }
            "Agents" => {
                settings_card(ui, |ui| {
                    section_title(ui, "CODEX IN THE ISLAND");
                    label(ui, &format!("Codex works in {}", codex_workspace().display()), 12.0, MUTED);
                    if small_button(ui, "Show project folder").clicked() {
                        let _ = Command::new("open").arg(codex_workspace()).spawn();
                    }
                    if ui.add_enabled(self.codex_rx.is_none(),
                        egui::Checkbox::new(&mut self.prefs.codex_allow_edits,
                            "Allow Codex to edit files in the Bookland project")).changed() {
                        self.codex_session_id = None;
                        self.codex_messages.clear();
                        self.codex_status = "Started a new chat for this access mode".into();
                        self.save_data();
                    }
                    label(ui, "Off: read only. On: workspace edits with Codex automatic approval review. Changing this starts a new chat.", 11.0, MUTED);
                    ui.add_space(13.0);
                    section_title(ui, "LOCAL ACTIVITY");
                    if ui.checkbox(&mut self.prefs.agent_monitor,
                        "Show local AI agent activity").changed() {
                        self.agents_initialized = false;
                        self.agent_completion = None;
                    }
                    ui.checkbox(&mut self.prefs.monitor_craft, "Include Craft Anything projects from Documents");
                    label(ui, "Codex sessions update every five seconds. Craft access may need macOS folder permission. Data stays on this Mac.", 12.0, MUTED);
                    label(ui, &format!("{} local tasks found", self.agents.len()), 12.0, ACCENT);
                    if small_button(ui, "Refresh now").clicked() {
                        self.last_agent_poll = Instant::now() - Duration::from_secs(20);
                    }
                });
            }
            "About" => {
                settings_card(ui, |ui| {
                    label(ui, &format!("Bookland {}", env!("CARGO_PKG_VERSION")),
                        18.0, TEXT);
                    label(ui, "A living island for your Mac.", 13.0, MUTED);
                    ui.add_space(8.0);
                    label(ui, "Bookland is free and open source, with no Bookland account, trial, or subscription. Built with Rust and native AppKit Liquid Glass.",
                        12.0, TEXT);
                    ui.add_space(9.0);
                    ui.horizontal(|ui| {
                        ui.hyperlink_to("Website",
                            "https://bookland-mac-island.moshe99871.chatgpt.site");
                        ui.hyperlink_to("Source on GitHub",
                            "https://github.com/KyrocEU/bookland");
                    });
                });
                ui.add_space(12.0);
                section_title(ui, "INDEPENDENT OPEN SOURCE PROJECT");
                settings_card(ui, |ui| {
                    label(ui, "Bookland's original code is MIT licensed. Third-party components and marks keep their own terms; see the repository notices.",
                        12.0, TEXT);
                    label(ui, "The island and major workspaces are working. Motion, integrations, and feature coverage are still in progress.",
                        12.0, MUTED);
                });
            }
            _ => {
                settings_card(ui, |ui| {
                    label(
                        ui,
                        "This section was visible in the demo settings.",
                        13.0,
                        TEXT,
                    );
                    label(
                        ui,
                        "Its controls and behavior have not been verified.",
                        12.0,
                        MUTED,
                    );
                });
            }
        }
    }
    fn media_settings(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "MEDIA PLAYER");
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                for layout in ["Default", "Expanded"] {
                    ui.selectable_value(&mut self.prefs.media_layout, layout.into(), layout);
                }
            });
            label(
                ui,
                "Expanded shows the playing app and seekable song progress.",
                12.0,
                MUTED,
            );
            ui.add_space(8.0);
            label(ui, "Extra action", 12.0, MUTED);
            egui::ComboBox::from_id_salt("extra_action")
                .selected_text(&self.prefs.extra_action)
                .show_ui(ui, |ui| {
                    for choice in ["Shuffle", "Open app", "None"] {
                        ui.selectable_value(&mut self.prefs.extra_action, choice.into(), choice);
                    }
                });
            ui.checkbox(&mut self.prefs.show_synced_lyrics, "Show synced lyrics when available");
            label(ui, "Looks up the song title and artist on LRCLIB when the track changes.", 11.0, MUTED);
            ui.checkbox(&mut self.prefs.audio_spectrum, "Live audio spectrum");
            label(ui, "Uses System Audio Recording only while the Music panel is open. No audio is saved.",
                11.0, MUTED);
        });
        ui.add_space(12.0);
        section_title(ui, "AIRPODS");
        card().show(ui, |ui| {
            #[cfg(target_os = "macos")]
            unsafe {
                if self.airpods_events_enabled {
                    label(ui, "Live connection alerts enabled", 12.0, TEXT);
                } else if bookland_airpods_authorization() == 2 {
                    label(ui, "Bluetooth access is off in macOS Privacy settings. Bookland checks every 30 seconds.", 12.0, MUTED);
                } else {
                    label(ui, "Enable Bluetooth for immediate connection alerts. Without it, Bookland checks every 30 seconds.", 12.0, MUTED);
                    if small_button(ui, "Enable live AirPods alerts").clicked()
                        && bookland_airpods_request_access() == 0 {
                        self.notify("Bluetooth access is unavailable");
                    }
                }
            }
        });
    }
    fn widget_settings(&mut self, ui: &mut egui::Ui) {
        match self.selected_widget {
            Widget::MediaPlayer => self.media_settings(ui),
            Widget::Pulse => {
                card().show(ui, |ui| {
                    label(ui, "Pulse style", 12.0, MUTED);
                    ui.horizontal(|ui| {
                        for style in ["Cards", "Rings", "List"] {
                            ui.selectable_value(&mut self.prefs.pulse_style, style.into(), style);
                        }
                    });
                    ui.add_space(7.0);
                    for (side, target) in [
                        ("Left card", &mut self.prefs.pulse_left),
                        ("Right card", &mut self.prefs.pulse_right),
                    ] {
                        egui::ComboBox::from_label(side)
                            .selected_text(target.as_str())
                            .show_ui(ui, |ui| {
                                for metric in ["CPU", "RAM", "GPU", "Disk", "Battery", "Network"] {
                                    ui.selectable_value(target, metric.into(), metric);
                                }
                            });
                    }
                    ui.checkbox(&mut self.prefs.pulse_explain, "Explain on hover");
                    ui.add(
                        egui::Slider::new(&mut self.prefs.pulse_interval, 1..=30)
                            .text("Refresh interval (seconds)"),
                    );
                    label(ui, "GPU usage is available on supported Macs.", 11.0, MUTED);
                });
            }
            Widget::Pomodoro => {
                card().show(ui, |ui| {
                    label(
                        ui,
                        "Choose 5, 15, or 25 minutes in the Nook widget.",
                        12.0,
                        MUTED,
                    );
                });
            }
            Widget::Todos => {
                card().show(ui, |ui| {
                    label(
                        ui,
                        "Add, check, and remove to-dos in the Nook widget.",
                        12.0,
                        MUTED,
                    );
                });
            }
            Widget::Notes => { label(ui, "Notes are saved locally as you type. Use the widget button to save a copy in Apple Notes.", 12.0, MUTED); }
            Widget::Clipboard => { label(ui, "Search and filter copied text in Clipboard. Items marked private by macOS or password managers are skipped.", 12.0, MUTED); }
            Widget::Downloads => { label(ui, "Shows active transfers and recent files from the watched folder. Choose the folder in Settings → Downloads.", 12.0, MUTED); }
            Widget::Terminal => { label(ui, "Runs a command only when you press Run in the island.", 12.0, MUTED); }
            Widget::Weather => {
                card().show(ui, |ui| {
                    label(ui, "City", 12.0, MUTED);
                    if ui.add(egui::TextEdit::singleline(&mut self.prefs.weather_city)
                        .hint_text("For example: Hebron")).changed() {
                        self.last_weather_poll = Instant::now();
                    }
                    label(ui, "Weather uses Open-Meteo. The city name is sent when this widget is enabled.", 11.0, MUTED);
                    if small_button(ui, "Refresh weather").clicked() {
                        self.last_weather_poll = Instant::now() - Duration::from_secs(1000);
                    }
                });
            }
            Widget::Mirror => {
                label(ui, "Add Mirror to your Nook, then press Start camera there. The preview stops when you close the island or leave Nook.", 12.0, MUTED);
            }
        }
    }
    fn settings_window_ui(&mut self, ctx: &egui::Context) {
        apply_visuals(ctx, self.prefs.rounded_buttons);
        let before = self.prefs.clone();
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Color32::from_rgb(14, 17, 23))
                .inner_margin(Margin::same(11)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(9.0, 8.0);
                ui.horizontal(|ui| {
                    Frame::new()
                        .fill(Color32::from_rgb(24, 27, 35))
                        .stroke(Stroke::new(0.8, Color32::from_rgb(48, 54, 68)))
                        .corner_radius(17.0)
                        .inner_margin(13)
                        .show(ui, |ui| {
                            ui.set_width(202.0);
                            ui.set_min_height(580.0);
                            ui.vertical(|ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(580.0)
                                    .show(ui, |ui| self.settings_nav(ui));
                            });
                        });
                    ui.add_space(2.0);
                    Frame::new()
                        .fill(Color32::from_rgb(28, 30, 38))
                        .stroke(Stroke::new(0.8, Color32::from_rgb(53, 58, 72)))
                        .corner_radius(17.0)
                        .inner_margin(18)
                        .show(ui, |ui| {
                            ui.set_width(535.0);
                            ui.set_min_height(570.0);
                            ui.vertical(|ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(570.0)
                                    .show(ui, |ui| self.settings_content(ui));
                            });
                        });
                });
            });
        if self.tour_step.is_some() {
            self.onboarding_tour(ctx);
        }
        if self.prefs != before {
            self.save_prefs();
        }
    }
    fn onboarding_tour(&mut self, ctx: &egui::Context) {
        enum Action { Stay, Close, Back, Next, Jump(usize), TryIt }
        let step = self.tour_step.unwrap_or(0).min(TOUR_PAGES.len() - 1);
        let page = TOUR_PAGES[step];
        let mark = self.bookland_mark.clone();
        let accent = match step {
            0 | 6 => Color32::from_rgb(129, 169, 255),
            1 => Color32::from_rgb(123, 218, 181),
            2 => Color32::from_rgb(185, 150, 255),
            3 => Color32::from_rgb(244, 182, 119),
            4 => Color32::from_rgb(122, 205, 231),
            _ => Color32::from_rgb(167, 144, 247),
        };
        let mut action = Action::Stay;
        egui::Modal::new(egui::Id::new("bookland_onboarding_tour"))
            .backdrop_color(Color32::from_black_alpha(190))
            .frame(Frame::new().fill(Color32::from_rgb(21, 23, 29))
                .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(244, 247, 255, 36)))
                .corner_radius(24).inner_margin(Margin::same(24))
                .shadow(egui::epaint::Shadow { offset: [0, 16], blur: 36, spread: 0,
                    color: Color32::from_black_alpha(120) }))
            .show(ctx, |ui| {
                ui.set_width(510.0);
                ui.horizontal(|ui| {
                    if let Some(mark) = &mark {
                        ui.add(egui::Image::from_texture(mark)
                            .fit_to_exact_size(Vec2::splat(30.0)));
                    }
                    ui.label(RichText::new("BOOKLAND").size(14.0).strong().color(TEXT));
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{}/{}", step + 1, TOUR_PAGES.len()))
                            .size(12.0).color(MUTED));
                    });
                });
                ui.add_space(17.0);
                let (hero, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 132.0), egui::Sense::hover());
                ui.painter().rect_filled(hero, 19.0, Color32::from_rgb(30, 34, 43));
                ui.painter().circle_filled(hero.center(), 69.0,
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 24));
                let island = egui::Rect::from_center_size(hero.center(), Vec2::new(260.0, 59.0));
                ui.painter().rect_filled(island, 25.0, Color32::from_rgb(3, 5, 8));
                if let Some(mark) = &mark {
                    ui.painter().image(mark.id(), egui::Rect::from_center_size(
                        island.left_center() + Vec2::new(28.0, 0.0), Vec2::splat(30.0)),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE);
                }
                ui.painter().text(island.center(), Align2::CENTER_CENTER, page.eyebrow,
                    FontId::proportional(13.0), TEXT);
                for index in 0..TOUR_PAGES.len() {
                    ui.painter().circle_filled(
                        island.right_center() - Vec2::new(40.0 - index as f32 * 5.0, 0.0),
                        if index == step { 2.3 } else { 1.3 },
                        if index == step { accent } else { Color32::from_gray(85) },
                    );
                }
                ui.add_space(18.0);
                ui.label(RichText::new(page.title).size(25.0).strong().color(TEXT));
                ui.add_space(8.0);
                ui.label(RichText::new(page.summary).size(14.0).color(TEXT));
                ui.add_space(10.0);
                ui.label(RichText::new(page.detail).size(12.0).color(MUTED));
                ui.add_space(23.0);
                ui.horizontal(|ui| {
                    for index in 0..TOUR_PAGES.len() {
                        let (dot, response) = ui.allocate_exact_size(
                            Vec2::splat(13.0), egui::Sense::click());
                        ui.painter().circle_filled(dot.center(), if index == step { 4.5 } else { 3.0 },
                            if index == step { accent } else { Color32::from_gray(82) });
                        if response.clicked() { action = Action::Jump(index); }
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let next = if step + 1 == TOUR_PAGES.len() { "Done" } else { "Next" };
                        if ui.add(egui::Button::new(
                            RichText::new(next).strong().color(Color32::from_rgb(13, 18, 27)))
                            .fill(accent)
                            .min_size(Vec2::new(73.0, 31.0))).clicked() {
                            action = if step + 1 == TOUR_PAGES.len() {
                                Action::Close
                            } else { Action::Next };
                        }
                        if step > 0 && ui.button("Back").clicked() { action = Action::Back; }
                        if ui.button("Close").clicked() { action = Action::Close; }
                    });
                });
                ui.add_space(11.0);
                let try_label = if page.workspace.is_some() {
                    "Try this in the island ↗"
                } else { "Open Appearance settings ↗" };
                if ui.link(try_label).clicked() { action = Action::TryIt; }
            });
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            action = Action::Close;
        }
        match action {
            Action::Stay => {}
            Action::Close => self.tour_step = None,
            Action::Back => self.tour_step = Some(step.saturating_sub(1)),
            Action::Next => self.tour_step = Some((step + 1).min(TOUR_PAGES.len() - 1)),
            Action::Jump(index) => self.tour_step = Some(index),
            Action::TryIt => {
                self.tour_step = None;
                self.settings_section = page.settings.into();
                if let Some(workspace) = page.workspace {
                    self.workspace = workspace;
                    self.workspace_chosen_by_user = true;
                    self.expanded = true;
                    self.opened_by_hover = false;
                    self.clipboard_bottom_sheet = false;
                    self.settings_open = false;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                    self.pocket_wake_context.request_repaint();
                }
            }
        }
    }
    fn screenshot_events(&mut self, ctx: &egui::Context) {
        if let Some(path) = &self.screenshot {
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
                    for pixel in &image.pixels {
                        rgba.extend_from_slice(&pixel.to_array());
                    }
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(err) = image::save_buffer(
                        path,
                        &rgba,
                        image.size[0] as u32,
                        image.size[1] as u32,
                        image::ColorType::Rgba8,
                    ) {
                        eprintln!("Could not save screenshot: {err}");
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                    return;
                }
            }
            self.frames += 1;
            if self.frames >= 120 && !self.shot_requested {
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::UserData::default()));
                self.shot_requested = true;
            }
        }
    }
}

impl Drop for DeskApp {
    fn drop(&mut self) {
        if let Some(pid) = self.desktop_codex_pid { let _ = codex_bridge::stop(pid); }
        #[cfg(target_os = "macos")]
        unsafe {
            if self.prompter_voice_started { bookland_speech_stop(); }
            if self.mirror_open { bookland_mirror_stop(); }
            if self.pocket_monitor_enabled {
                bookland_pocket_drag_monitor_set(false, std::ptr::null_mut(), None, None);
            }
            if self.hover_monitor_enabled {
                nooklet_hover_monitor_set(0, std::ptr::null_mut(), None);
            }
            if self.animation_clock_enabled {
                nooklet_animation_clock_set(0, std::ptr::null_mut(), None);
            }
            if self.spectrum_capture_active { bookland_audio_spectrum_set(0); }
            if self.airpods_events_attempted { bookland_airpods_watch_set(0); }
        }
        if self.prefs.clipboard_clear_on_quit && !self.preview_mode {
            let favorites = self.clipboard_favorites.clone();
            self.clipboard.retain(|item| favorites.contains(item));
            self.clipboard_images.retain(|item| item.favorite);
            self.clipboard_files.retain(|item| item.favorite);
            self.save_data();
        }
    }
}

impl eframe::App for DeskApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // eframe's default clear has 180/255 alpha. That paints the entire
        // transparent host as a faint rectangle around the camera cutout.
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.agent_completion.as_ref().is_some_and(|done|
            done.created_at.elapsed() >= Duration::from_secs(4)) {
            self.agent_completion = None;
        }
        if self.notch_shoulders.is_none() {
            let load = |name, bytes: &[u8]| -> Option<egui::TextureHandle> {
                let image = image::load_from_memory(bytes).ok()?.to_rgba8();
                let pixels = egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize], image.as_raw());
                Some(ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR))
            };
            if let (Some(left), Some(right)) = (
                load("notch-shoulder-left", include_bytes!("../assets/notch-shoulder-left.png")),
                load("notch-shoulder-right", include_bytes!("../assets/notch-shoulder-right.png")),
            ) {
                self.notch_shoulders = Some([left, right]);
            }
        }
        if self.codex_logo.is_none() {
            if let Ok(image) = image::load_from_memory(
                include_bytes!("../assets/openai-blossom-white.png")) {
                let rgba = image.to_rgba8();
                let pixels = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                self.codex_logo = Some(ctx.load_texture("openai-blossom", pixels,
                    egui::TextureOptions::LINEAR));
            }
        }
        if self.bookland_mark.is_none() {
            if let Ok(image) = image::load_from_memory(
                include_bytes!("../assets/brand/bookland-icon-premium.png")) {
                // The distributed icon is large; the island needs only a
                // small texture, so do not keep a 1254-pixel GPU copy.
                let rgba = image.thumbnail(96, 96).to_rgba8();
                let pixels = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                self.bookland_mark = Some(ctx.load_texture("bookland-premium-island", pixels,
                    egui::TextureOptions::LINEAR));
            }
        }
        #[cfg(target_os = "macos")]
        unsafe {
            let desired = self.prefs.pocket_shake && !self.preview_mode && self.screenshot.is_none();
            if desired != self.pocket_monitor_enabled {
                let pointer = self.pocket_wake_context.as_ref() as *const egui::Context
                    as *mut std::ffi::c_void;
                bookland_pocket_drag_monitor_set(desired, pointer,
                    desired.then_some(bookland_pocket_wake),
                    desired.then_some(bookland_pocket_shake_sample));
                self.pocket_monitor_enabled = desired;
            }
            let mut x = 0.0;
            let mut y = 0.0;
            if desired && bookland_pocket_drag_monitor_take(&mut x, &mut y) {
                self.pocket_anchor = egui::pos2(x as f32, y as f32);
                self.pocket_floating = true;
                self.pocket_opened_from_drag = true;
                self.notify("Pocket open · drop the file in the floating panel");
            }
        }
        if self.prompter_voice_started && (!self.expanded || self.workspace != Workspace::Prompter
            || self.prefs.prompter_mode != "Voice") {
            #[cfg(target_os = "macos")]
            unsafe { bookland_speech_stop(); }
            self.prompter_voice_started = false;
        }
        let typing_in_island = ctx.wants_keyboard_input() && ctx.input(|input| input.focused);
        let escape_locked = self.prefs.lock_while_typing && typing_in_island;
        if self.expanded && self.workspace == Workspace::Clipboard
            && self.clipboard_bottom_sheet
            && !escape_locked
            && ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.expanded = false;
            self.opened_by_hover = false;
            self.hover_blocked_until_exit = true;
        }
        if self.expanded && self.workspace != Workspace::Clipboard
            && !escape_locked
            && ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.expanded = false;
            self.opened_by_hover = false;
            self.hover_blocked_until_exit = true;
        }
        #[cfg(target_os = "macos")]
        unsafe {
            if self.prefs.global_shortcut != self.shortcut_registered && !self.preview_mode {
                if nooklet_shortcut_set(self.prefs.global_shortcut as i32) != 0 {
                    self.shortcut_registered = self.prefs.global_shortcut;
                } else {
                    self.prefs.global_shortcut = false;
                    self.notify("Could not register ⌘⌥N; another app may use it");
                    self.save_prefs();
                }
            }
            if nooklet_shortcut_take() != 0 {
                self.expanded = !self.expanded;
                self.opened_by_hover = false;
                self.hover_blocked_until_exit = !self.expanded;
                self.clipboard_bottom_sheet = false;
            }
            if self.prefs.clipboard_shortcut != self.clipboard_shortcut_registered && !self.preview_mode {
                if bookland_clipboard_shortcut_set(self.prefs.clipboard_shortcut as i32) != 0 {
                    self.clipboard_shortcut_registered = self.prefs.clipboard_shortcut;
                } else {
                    self.prefs.clipboard_shortcut = false;
                    self.notify("Could not register Control+Command+6; another app may use it");
                    self.save_prefs();
                }
            }
            if bookland_clipboard_shortcut_take() != 0 {
                self.workspace = Workspace::Clipboard;
                self.expanded = true;
                self.opened_by_hover = false;
                self.clipboard_bottom_sheet = true;
                self.clipboard_focus_pending = self.prefs.clipboard_focus_search;
            }
        }
        // Use the last settled notch rect before clearing frame geometry. A
        // tracking-area wake can now choose the open target before the spring
        // step, saving the extra UI frame that the in-panel hover path needed.
        if !self.expanded && self.transition == 0.0 && self.prefs.open_on_hover
            && self.screenshot.is_none() && !self.preview_mode
            && !ctx.is_popup_open()
            && ctx.input(|input| !input.pointer.any_down() && !input.pointer.any_click())
        {
            if let Some(rect) = self.open_notch_rect {
                #[cfg(target_os = "macos")]
                let physically_hovered = unsafe { bookland_pointer_over_island_rect(
                    rect.min.x as f64, rect.min.y as f64,
                    rect.width() as f64, rect.height() as f64, 0.0, true,
                ) };
                #[cfg(not(target_os = "macos"))]
                let physically_hovered = ctx.input(|input| input.pointer.hover_pos())
                    .is_some_and(|pointer| rect.contains(pointer));
                if hover_allowed_after_close(&mut self.hover_blocked_until_exit,
                    physically_hovered) {
                    self.open_from_hover();
                }
            }
        }
        self.glass_rects = [None; 8];
        self.open_notch_rect = None;
        self.native_ring_center = None;
        self.native_usage_ring_center = None;
        self.native_morph_shell = None;
        let now = Instant::now();
        let (dragging_file, pointer_x) = ctx.input(|input|
            (!input.raw.hovered_files.is_empty(), input.pointer.hover_pos().map(|pos| pos.x)));
        if self.drag_shake.update(pointer_x,
            dragging_file && self.prefs.pocket_shake && self.screenshot.is_none(), now) {
            self.open_pocket_near_pointer(ctx);
            self.pocket_opened_from_drag = true;
            self.notify("Pocket open · drop the file in the floating panel");
        }
        let target_changed = self.expanded != self.animation_target_expanded;
        self.animation_target_expanded = self.expanded;
        if target_changed {
            self.native_morph_pending = true;
            self.native_morph_animating = false;
        }
        // Start moving on the first frame after a hover event. Zeroing this
        // frame added a full display interval to the apparent hover latency.
        let dt = spring_frame_dt(now.duration_since(self.last_frame), target_changed);
        self.last_frame = now;
        let expanded_before_panel = self.expanded;
        let target = if self.expanded { 1.0 } else { 0.0 };
        #[cfg(target_os = "macos")]
        let reduce_motion = unsafe { nooklet_reduce_motion() != 0 };
        #[cfg(not(target_os = "macos"))]
        let reduce_motion = false;
        // The original notch shape uses a 0.5 s / 0.82 SwiftUI spring.
        // Keep the slight overshoot instead of snapping to the open size at
        // the first crossing. Reduce Motion remains fully damped.
        let (response, damping) = if reduce_motion {
            (0.25, 1.0)
        } else if self.prefs.style == "Glass" {
            (0.5, 0.82)
        } else {
            (if self.expanded { 0.47 } else { 0.4 }, 1.0)
        };
        let omega = std::f32::consts::TAU * self.prefs.animation_speed.clamp(0.5, 2.0) / response;
        (self.transition, self.transition_velocity) = spring_step(
            self.transition, self.transition_velocity, target, omega, damping, dt);
        self.transition = self.transition.clamp(0.0, 1.02);
        if (self.transition - target).abs() < 0.005 && self.transition_velocity.abs() < 0.02 {
            self.transition = target;
            self.transition_velocity = 0.0;
        }
        if self.screenshot.is_some() || target_changed {
            ctx.request_repaint();
        } else if self.transition != target {
            // The native display link requests frames on vblank while the
            // island moves. Use a timer only when that link is unavailable.
            #[cfg(target_os = "macos")]
            if !self.animation_clock_enabled {
                ctx.request_repaint_after(Duration::from_millis(8));
            }
            #[cfg(not(target_os = "macos"))]
            ctx.request_repaint_after(Duration::from_millis(8));
        } else {
            // The native tracking area wakes the island immediately on pointer
            // entry/exit. Avoid redrawing the closed Metal surface several
            // times per second just to discover a hover that AppKit already
            // reports. The next idle wake also processes background results.
            ctx.request_repaint_after(Duration::from_millis(if self.expanded { 100 }
                else if self.prefs.open_on_hover { 750 } else { 1000 }));
        }
        self.screenshot_events(ctx);
        self.poll(ctx);
        let position_bottom = self.expanded && self.clipboard_bottom_sheet
            && self.workspace == Workspace::Clipboard && self.screenshot.is_none();
        if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
            self.clipboard_shelf_width = (monitor.x - 48.0).clamp(720.0, 2400.0);
        }
        let (open_width, open_height) = if position_bottom {
            (self.clipboard_shelf_width, 195.0)
        } else { self.open_dimensions() };
        let compact_nook = self.prefs.style == "Glass" && self.workspace == Workspace::Nook
            && self.prefs.widgets == Widget::DEFAULT && self.prefs.nook_enabled
            && open_height <= 260.0;
        let (closed_width, closed_height) = self.closed_dimensions();
        // Let the spring animate the painted island inside a stable host.
        // Resizing the Metal swapchain on every spring frame creates visible
        // pacing stalls and more work for WindowServer. Keep the expanded host
        // through the morph, then shrink it once the closed shape settles.
        let full_height = open_height + if position_bottom { 34.0 }
            else if compact_nook { 75.0 } else { 160.0 };
        let size = if self.transition == 0.0 && !self.expanded {
            Vec2::new(closed_width + 30.0, closed_height + 12.0)
        } else {
            Vec2::new(open_width + 44.0, full_height)
        };
        if !self.positioned || size != self.applied_size || position_bottom != self.last_bottom_position {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
            if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(
                    (monitor.x - size.x) / 2.0,
                    if position_bottom { (monitor.y - size.y - 20.0).max(0.0) } else { 0.0 },
                )));
                self.positioned = true;
                self.last_bottom_position = position_bottom;
            }
            self.applied_size = size;
        }
        ctx.set_visuals(egui::Visuals::dark());
        let before = self.prefs.clone();
        egui::CentralPanel::default()
            .frame(Frame::new().fill(if self.screenshot.is_some() {
                Color32::from_rgb(7, 9, 13)
            } else {
                Color32::TRANSPARENT
            }))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if self.transition >= 0.96
                        && (self.prefs.style != "Glass" || self.transition == target
                            || self.screenshot.is_some() || self.preview_mode) {
                        self.panel(ui);
                    } else if self.transition > 0.0 {
                        self.morphing_notch(ui);
                    } else {
                        self.closed_notch(ui);
                    }
                });
            });
        let gesture_rect = if self.transition < 0.96 {
            self.open_notch_rect
        } else {
            self.open_notch_rect.or(self.glass_rects[0]).map(|panel| {
                egui::Rect::from_center_size(
                    egui::pos2(panel.center().x, panel.top() + 20.0),
                    Vec2::new(panel.width().min(255.0), 40.0),
                )
            })
        };
        let gesture_pointer = ctx.input(|input| input.pointer.hover_pos());
        #[cfg(target_os = "macos")]
        let gesture_delta = unsafe {
            let mut x = 0.0;
            let mut y = 0.0;
            if bookland_scroll_take(&mut x, &mut y) != 0 {
                Vec2::new(x as f32, y as f32)
            } else { Vec2::ZERO }
        };
        #[cfg(not(target_os = "macos"))]
        let gesture_delta = ctx.input(|input| input.raw_scroll_delta);
        if self.prefs.gestures_on_hover && self.screenshot.is_none() && !self.preview_mode
            && gesture_pointer.is_some_and(|pointer|
                gesture_rect.is_some_and(|rect| rect.contains(pointer))) {
            if let Some(action) = self.island_gesture.update(
                (gesture_delta.x, gesture_delta.y), now, self.expanded,
                self.prefs.gesture_vertical, self.prefs.gesture_horizontal_media,
                self.prefs.gesture_invert_media,
            ) {
                match action {
                    IslandGestureAction::Open => {
                        self.expanded = true;
                        self.opened_by_hover = true;
                        self.hover_blocked_until_exit = false;
                        self.last_hovered = now;
                        self.hover_opened_at = now;
                    }
                    IslandGestureAction::Close if !escape_locked => {
                        self.expanded = false;
                        self.opened_by_hover = false;
                        self.hover_blocked_until_exit = true;
                    }
                    IslandGestureAction::Previous => self.skip_media(true),
                    IslandGestureAction::Next => self.skip_media(false),
                    _ => {}
                }
            }
        } else {
            self.island_gesture.reset();
        }
        if self.expanded && self.opened_by_hover && self.prefs.auto_close && self.screenshot.is_none() {
            let regions = [self.glass_rects[0], self.glass_rects[1], self.open_notch_rect];
            #[cfg(target_os = "macos")]
            let hovered = regions.iter().flatten().any(|rect| unsafe {
                bookland_pointer_over_island_rect(rect.min.x as f64, rect.min.y as f64,
                    rect.width() as f64, rect.height() as f64, 8.0, false)
            });
            #[cfg(not(target_os = "macos"))]
            let hovered = ctx.input(|i| i.pointer.hover_pos())
                .is_some_and(|p| regions.iter().flatten().any(|rect| rect.expand(8.0).contains(p)));
            let editing_text = ctx.wants_keyboard_input() && ctx.input(|input| input.focused);
            if hovered || (self.prefs.prevent_close_while_typing && editing_text) {
                self.last_hovered = Instant::now();
            } else if self.last_hovered.elapsed() >= Duration::from_millis(self.prefs.close_delay_ms.clamp(100, 3000))
                && self.hover_opened_at.elapsed() >= hover_close_guard(self.prefs.animation_speed, {
                    #[cfg(target_os = "macos")]
                    { unsafe { nooklet_reduce_motion() != 0 } }
                    #[cfg(not(target_os = "macos"))]
                    { false }
                }) {
                self.expanded = false;
                self.opened_by_hover = false;
            }
        }
        if self.expanded != self.last_haptic_expanded {
            if self.prefs.haptic_feedback && self.screenshot.is_none() && !self.preview_mode {
                #[cfg(target_os = "macos")]
                unsafe { bookland_haptic_feedback(); }
            }
            self.last_haptic_expanded = self.expanded;
        }
        self.apply_native_glass();
        if self.expanded != expanded_before_panel {
            ctx.request_repaint();
        }
        if self.prefs != before {
            self.save_prefs();
        }
        if self.settings_open {
            let builder = egui::ViewportBuilder::default()
                .with_title("Bookland Settings")
                .with_inner_size([860.0, 630.0])
                // Both grouped columns and their control cards have fixed
                // minimum widths. Keep the viewport wide enough that the
                // right-hand controls are never clipped by a resize.
                .with_min_inner_size([850.0, 600.0])
                .with_window_level(egui::WindowLevel::AlwaysOnTop);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("bookland_settings"),
                builder,
                |settings_ctx, class| {
                    if settings_ctx.input(|i| i.viewport().close_requested()) {
                        self.settings_open = false;
                        self.tour_step = None;
                        return;
                    }
                    if class == egui::ViewportClass::Embedded {
                        egui::Window::new("Bookland Settings").show(settings_ctx, |ui| {
                            self.settings_content(ui);
                        });
                        if self.tour_step.is_some() { self.onboarding_tour(settings_ctx); }
                    } else {
                        self.settings_window_ui(settings_ctx);
                    }
                },
            );
        }
        if self.pocket_floating && self.screenshot.is_none() {
            let pocket_height = if self.files.is_empty() { 200.0 }
                else if self.pocket_status.is_empty() { 295.0 } else { 320.0 };
            let builder = egui::ViewportBuilder::default()
                .with_title("Bookland Pocket")
                .with_position(self.pocket_anchor)
                .with_inner_size([430.0, pocket_height])
                .with_decorations(false)
                .with_transparent(true)
                .with_active(!self.pocket_opened_from_drag)
                .with_resizable(false)
                .with_taskbar(false)
                .with_window_level(egui::WindowLevel::AlwaysOnTop);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("bookland_floating_pocket"),
                builder,
                |pocket_ctx, class| {
                    if pocket_ctx.input(|input| input.viewport().close_requested()) {
                        self.pocket_floating = false;
                        return;
                    }
                    if class == egui::ViewportClass::Embedded {
                        egui::Window::new("Bookland Pocket").show(pocket_ctx, |ui| {
                            label(ui, "Pocket", 19.0, TEXT);
                            label(ui, "Drop a file here", 13.0, MUTED);
                        });
                    } else {
                        self.floating_pocket_ui(pocket_ctx);
                    }
                },
            );
        }
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--mirror-health") {
        #[cfg(target_os = "macos")]
        println!("camera_available={} permission={} capture_state={}",
            unsafe { bookland_mirror_camera_available() },
            unsafe { bookland_mirror_permission() },
            unsafe { bookland_mirror_state() });
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--speech-health") {
        #[cfg(target_os = "macos")]
        for locale in ["en-US", "he-IL", "ar-IL", "ar-SA"] {
            let code = std::ffi::CString::new(locale).unwrap();
            let flags = unsafe { bookland_speech_capabilities(code.as_ptr()) };
            println!("{locale}: supported={}, on-device={}, available={}",
                flags & 1 != 0, flags & 2 != 0, flags & 4 != 0);
        }
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--health") {
        let agents = std::env::var_os("HOME")
            .map(|home| agents::scan(&PathBuf::from(home)))
            .unwrap_or_default();
        println!("local agents: {}, working: {}", agents.len(),
            agents.iter().filter(|agent| agent.status == "Working").count());
        return Ok(());
    }
    // A separately signed staging bundle can opt into preview behavior without
    // replacing the running app or changing its Calendar authorization.
    let bundled_preview = std::env::current_exe().ok()
        .and_then(|executable| executable.parent()
            .map(|folder| folder.join("../Resources/bookland-preview-mode")))
        .is_some_and(|flag| flag.is_file());
    let preview_mode = bundled_preview || args.iter().any(|arg| arg == "--preview");
    let screenshot = args
        .windows(2)
        .find(|pair| pair[0] == "--screenshot")
        .map(|pair| PathBuf::from(&pair[1]));
    let shot_mode = screenshot.is_some();
    let screenshot_workspace = args.windows(2)
        .find(|pair| pair[0] == "--screenshot-workspace")
        .and_then(|pair| Workspace::DEFAULT.into_iter()
            .chain(std::iter::once(Workspace::Prompter))
            .find(|workspace| workspace.name().eq_ignore_ascii_case(&pair[1])));
    let viewport = egui::ViewportBuilder::default()
        .with_title("Bookland")
        .with_inner_size(if shot_mode || preview_mode {
            [564.0, 740.0]
        } else {
            [270.0, 50.0]
        })
        .with_decorations(false)
        .with_transparent(true)
        .with_active(!shot_mode)
        .with_window_level(if shot_mode || preview_mode {
            #[cfg(target_os = "windows")]
            {
                egui::WindowLevel::AlwaysOnBottom
            }
            #[cfg(not(target_os = "windows"))]
            {
                egui::WindowLevel::Normal
            }
        } else {
            egui::WindowLevel::AlwaysOnTop
        });
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        event_loop_builder: Some(Box::new(move |builder| {
            #[cfg(target_os = "macos")]
            {
                use winit::platform::macos::EventLoopBuilderExtMacOS;
                builder.with_activate_ignoring_other_apps(!shot_mode);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = builder;
        })),
        ..Default::default()
    };
    eframe::run_native(
        "Bookland",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            if shot_mode {
                let ctx = cc.egui_ctx.clone();
                std::thread::spawn(move || {
                    for _ in 0..1200 {
                        ctx.request_repaint();
                        std::thread::sleep(Duration::from_millis(16));
                    }
                });
            }
            #[cfg(target_os = "macos")]
            let native_view = cc.window_handle().ok().and_then(|h| {
                if let RawWindowHandle::AppKit(handle) = h.as_raw() {
                    Some(handle.ns_view.as_ptr() as usize)
                } else {
                    None
                }
            }).unwrap_or(0);
            #[cfg(not(target_os = "macos"))]
            let native_view = 0;
            let mut app = DeskApp::new(screenshot, native_view, preview_mode,
                cc.egui_ctx.clone());
            if shot_mode {
                if let Some(workspace) = screenshot_workspace { app.workspace = workspace; }
                if app.workspace == Workspace::Agents {
                    if let Some(home) = std::env::var_os("HOME") {
                        app.agents = agents::scan_local(&PathBuf::from(home));
                        app.agents_initialized = true;
                    }
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod note_tests {
    use super::note_html;

    #[test]
    fn note_text_is_html_escaped_before_export() {
        assert_eq!(note_html("A & <B>\n\"C\""), "<div>A &amp; &lt;B&gt;<br>&quot;C&quot;</div>");
    }
}
