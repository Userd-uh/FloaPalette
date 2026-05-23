#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, Window};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::ShortcutState;

static LAST_TARGET_WINDOW: LazyLock<Mutex<isize>> = LazyLock::new(|| Mutex::new(0));

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppData {
    app_name: String,
    version: u32,
    categories: Vec<Category>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Category {
    id: String,
    label: String,
    color: String,
    items: Vec<CommandItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CommandItem {
    id: String,
    title: String,
    command: String,
    description: String,
    favorite: bool,
    auto_enter: bool,
    input_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    data_file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SourcePayload {
    commands: Vec<SourceCommand>,
}

#[derive(Debug, Deserialize)]
struct SourceCommand {
    id: Option<String>,
    category: Option<String>,
    name: Option<String>,
    title: Option<String>,
    desc: Option<String>,
    description: Option<String>,
    cmd: Option<String>,
    command: Option<String>,
    text: Option<String>,
    content: Option<String>,
    value: Option<String>,
    tag: Option<String>,
    favorite: Option<bool>,
    #[serde(default, alias = "autoEnter")]
    auto_enter: Option<bool>,
    action: Option<String>,
}

#[cfg(windows)]
mod win_input {
    use std::ptr;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput,
        VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT, VK_LEFT,
        VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
        VK_V,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, SetForegroundWindow,
    };

    pub fn foreground_window() -> isize {
        unsafe { GetForegroundWindow() as isize }
    }

    pub fn focus_window(hwnd: isize) -> Result<(), String> {
        if hwnd == 0 {
            return Err("貼り付け先ウィンドウが未記録です".to_string());
        }
        unsafe {
            if SetForegroundWindow(hwnd as _) == 0 {
                return Err(format!("SetForegroundWindow failed: {}", std::io::Error::last_os_error()));
            }
        }
        Ok(())
    }

    pub fn send_text(text: &str, auto_enter: bool) -> Result<(), String> {
        set_clipboard_text(text)?;
        send_paste()?;
        if auto_enter {
            send_vk(VK_RETURN, false)?;
            send_vk(VK_RETURN, true)?;
        }
        Ok(())
    }

    pub fn send_shortcut(shortcut: &str) -> Result<(), String> {
        let sequence = windows_shortcut_variant(shortcut)
            .replace(" + ", "+")
            .replace(" +", "+")
            .replace("+ ", "+");
        if sequence.trim().is_empty() {
            return Err("ショートカットが空です".to_string());
        }

        for chord in sequence.split_whitespace() {
            send_chord(chord)?;
            std::thread::sleep(std::time::Duration::from_millis(70));
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn send_unicode_text(text: &str, auto_enter: bool) -> Result<(), String> {
        for ch in text.encode_utf16() {
            send_unicode_unit(ch)?;
        }
        if auto_enter {
            send_vk(VK_RETURN, false)?;
            send_vk(VK_RETURN, true)?;
        }
        Ok(())
    }

    fn set_clipboard_text(text: &str) -> Result<(), String> {
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);
        let bytes = wide.len() * std::mem::size_of::<u16>();

        unsafe {
            let handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if handle.is_null() {
                return Err(format!("GlobalAlloc failed: {}", std::io::Error::last_os_error()));
            }

            let locked = GlobalLock(handle) as *mut u16;
            if locked.is_null() {
                return Err(format!("GlobalLock failed: {}", std::io::Error::last_os_error()));
            }
            ptr::copy_nonoverlapping(wide.as_ptr(), locked, wide.len());
            GlobalUnlock(handle);

            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Err(format!("OpenClipboard failed: {}", std::io::Error::last_os_error()));
            }
            if EmptyClipboard() == 0 {
                CloseClipboard();
                return Err(format!("EmptyClipboard failed: {}", std::io::Error::last_os_error()));
            }
            if SetClipboardData(CF_UNICODETEXT as u32, handle).is_null() {
                CloseClipboard();
                return Err(format!("SetClipboardData failed: {}", std::io::Error::last_os_error()));
            }
            CloseClipboard();
        }

        Ok(())
    }

    fn send_paste() -> Result<(), String> {
        send_vk(VK_CONTROL, false)?;
        send_vk(VK_V, false)?;
        send_vk(VK_V, true)?;
        send_vk(VK_CONTROL, true)
    }

    fn send_unicode_unit(unit: u16) -> Result<(), String> {
        let mut input_down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0,
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let mut input_up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0,
                    wScan: unit,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let size = std::mem::size_of::<INPUT>() as i32;

        unsafe {
            let sent1 = SendInput(1, &mut input_down, size);
            if sent1 == 0 {
                return Err(format!("SendInput down failed: {}", std::io::Error::last_os_error()));
            }
            let sent2 = SendInput(1, &mut input_up, size);
            if sent2 == 0 {
                return Err(format!("SendInput up failed: {}", std::io::Error::last_os_error()));
            }
        }

        Ok(())
    }

    fn send_chord(chord: &str) -> Result<(), String> {
        let mut modifiers: Vec<u16> = Vec::new();
        let mut key: Option<u16> = None;

        for raw in chord.split('+') {
            let token = raw.trim();
            if token.is_empty() {
                continue;
            }
            if let Some(modifier) = modifier_vk(token) {
                modifiers.push(modifier);
            } else if key.is_none() {
                key = key_vk(token);
            } else {
                return Err(format!("ショートカットのキー指定が複数あります: {chord}"));
            }
        }

        let key = key.ok_or_else(|| format!("ショートカットのキーを認識できません: {chord}"))?;

        for modifier in &modifiers {
            send_vk(*modifier, false)?;
        }
        send_vk(key, false)?;
        send_vk(key, true)?;
        for modifier in modifiers.iter().rev() {
            send_vk(*modifier, true)?;
        }

        Ok(())
    }

    fn windows_shortcut_variant(shortcut: &str) -> String {
        let parts: Vec<&str> = shortcut
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        let selected = parts
            .iter()
            .find(|part| {
                let lower = part.to_ascii_lowercase();
                lower.contains("ctrl") || lower.contains("control")
            })
            .copied()
            .or_else(|| parts.first().copied())
            .unwrap_or(shortcut);

        selected
            .replace("Command", "Ctrl")
            .replace("command", "Ctrl")
            .replace("Cmd", "Ctrl")
            .replace("cmd", "Ctrl")
    }

    fn modifier_vk(token: &str) -> Option<u16> {
        match normalize_token(token).as_str() {
            "CTRL" | "CONTROL" => Some(VK_CONTROL),
            "SHIFT" => Some(VK_SHIFT),
            "ALT" | "OPTION" => Some(VK_MENU),
            "WIN" | "WINDOWS" | "META" => Some(VK_LWIN),
            "CMD" | "COMMAND" => Some(VK_CONTROL),
            _ => None,
        }
    }

    fn key_vk(token: &str) -> Option<u16> {
        let normalized = normalize_token(token);
        if normalized.len() == 1 {
            let ch = normalized.as_bytes()[0];
            if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
                return Some(ch as u16);
            }
        }
        if let Some(number) = normalized.strip_prefix('F').and_then(|n| n.parse::<u16>().ok()) {
            if (1..=24).contains(&number) {
                return Some(0x70 + number - 1);
            }
        }
        match normalized.as_str() {
            "ENTER" | "RETURN" => Some(VK_RETURN),
            "TAB" => Some(VK_TAB),
            "ESC" | "ESCAPE" => Some(VK_ESCAPE),
            "SPACE" => Some(VK_SPACE),
            "BACKSPACE" => Some(VK_BACK),
            "DELETE" | "DEL" => Some(VK_DELETE),
            "INSERT" | "INS" => Some(VK_INSERT),
            "LEFT" => Some(VK_LEFT),
            "RIGHT" => Some(VK_RIGHT),
            "UP" => Some(VK_UP),
            "DOWN" => Some(VK_DOWN),
            "HOME" => Some(VK_HOME),
            "END" => Some(VK_END),
            "PAGEUP" | "PGUP" => Some(VK_PRIOR),
            "PAGEDOWN" | "PGDN" => Some(VK_NEXT),
            _ => None,
        }
    }

    fn normalize_token(token: &str) -> String {
        token
            .trim()
            .chars()
            .filter(|ch| !ch.is_whitespace() && *ch != '-')
            .flat_map(char::to_uppercase)
            .collect()
    }

    fn send_vk(key: u16, key_up: bool) -> Result<(), String> {
        let mut down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    wScan: 0,
                    dwFlags: if key_up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let size = std::mem::size_of::<INPUT>() as i32;

        unsafe {
            let sent = SendInput(1, &mut down, size);
            if sent == 0 {
                return Err(format!("SendInput vk failed: {}", std::io::Error::last_os_error()));
            }
        }

        Ok(())
    }
}

#[cfg(not(windows))]
mod win_input {
    pub fn foreground_window() -> isize {
        0
    }

    pub fn focus_window(_hwnd: isize) -> Result<(), String> {
        Err("Windows only".to_string())
    }

    pub fn send_text(_text: &str, _auto_enter: bool) -> Result<(), String> {
        Err("Windows only".to_string())
    }

    pub fn send_shortcut(_shortcut: &str) -> Result<(), String> {
        Err("Windows only".to_string())
    }
}

fn data_path() -> Result<PathBuf, String> {
    let candidates = [
        std::env::current_dir().ok().map(|p| p.join("data").join("commands.json")),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join("data").join("commands.json"))),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|dir| dir.parent())
                .and_then(|dir| dir.parent())
                .and_then(|dir| dir.parent())
                .map(|dir| dir.join("data").join("commands.json"))
        }),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("data").join("commands.json")),
    ];

    candidates
        .into_iter()
        .flatten()
        .find(|path| path.exists())
        .ok_or_else(|| "data/commands.json was not found".to_string())
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

fn read_settings(app: &AppHandle) -> AppSettings {
    let Ok(path) = settings_path(app) else {
        return AppSettings::default();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    serde_json::from_str::<AppSettings>(&text).unwrap_or_default()
}

fn write_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

fn configured_data_path(app: &AppHandle) -> Option<PathBuf> {
    read_settings(app)
        .data_file_path
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
}

fn active_data_path(app: &AppHandle) -> Result<PathBuf, String> {
    configured_data_path(app).map_or_else(data_path, Ok)
}

fn read_data() -> Result<AppData, String> {
    match data_path() {
        Ok(path) => {
            let loaded = fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str::<AppData>(&text).map_err(|e| e.to_string()));

            if let Ok(data) = loaded {
                if is_valid_app_data(&data) {
                    return Ok(data);
                }
            }

            let imported = import_floinpalette_data()?;
            let text = serde_json::to_string_pretty(&imported).map_err(|e| e.to_string())?;
            fs::write(path, text).map_err(|e| e.to_string())?;
            Ok(imported)
        }
        Err(_) => import_floinpalette_data(),
    }
}

fn read_data_from_path(path: &PathBuf) -> Result<AppData, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str::<AppData>(&text).map_err(|e| e.to_string())
}

fn write_data_to_path(data: &AppData, path: &PathBuf) -> Result<(), String> {
    let text = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

fn is_valid_app_data(data: &AppData) -> bool {
    let item_count: usize = data.categories.iter().map(|category| category.items.len()).sum();
    if data.categories.is_empty() || item_count == 0 {
        return false;
    }
    data.categories
        .iter()
        .flat_map(|category| category.items.iter())
        .any(|item| !item.command.trim().is_empty())
}

fn import_floinpalette_data() -> Result<AppData, String> {
    let source_root = PathBuf::from(r"D:\project\FloInPalette\commands");
    let sources = [
        (source_root.join("global-commands.json"), None),
        (source_root.join("markdown-commands.json"), Some("Markdown".to_string())),
    ];
    let colors = [
        ("Text", "#6bc7ff"),
        ("Prompt", "#b46cff"),
        ("Terminal", "#35c2a0"),
        ("Obsidian", "#9f7aea"),
        ("HTML", "#f97316"),
        ("Symbols", "#f472b6"),
        ("Git", "#f6c453"),
        ("Markdown", "#b46cff"),
    ];
    let mut categories: Vec<Category> = Vec::new();

    for (path, default_category) in sources {
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
        let payload: SourcePayload =
            serde_json::from_str(&text).map_err(|e| format!("{}: {}", path.display(), e))?;

        for raw in payload.commands {
            if raw.action.as_deref() == Some("vscodeCommand") {
                continue;
            }
            let command = first_non_empty([
                raw.cmd.as_deref(),
                raw.command.as_deref(),
                raw.text.as_deref(),
                raw.content.as_deref(),
                raw.value.as_deref(),
            ]);
            if command.trim().is_empty() {
                continue;
            }
            let label = raw
                .category
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .or(default_category.clone())
                .unwrap_or_else(|| "General".to_string());
            let color = colors
                .iter()
                .find(|(name, _)| *name == label)
                .map(|(_, color)| color.to_string())
                .unwrap_or_else(|| "#6bc7ff".to_string());
            let category_index = match categories.iter().position(|category| category.label == label) {
                Some(index) => index,
                None => {
                    categories.push(Category {
                        id: slugify(&label),
                        label: label.clone(),
                        color,
                        items: Vec::new(),
                    });
                    categories.len() - 1
                }
            };
            let item_index = categories[category_index].items.len() + 1;

            categories[category_index].items.push(CommandItem {
                id: raw.id.unwrap_or_else(|| format!("{}-{}", slugify(&label), item_index)),
                title: first_non_empty([raw.name.as_deref(), raw.title.as_deref()]),
                command,
                description: first_non_empty([raw.desc.as_deref(), raw.description.as_deref()]),
                favorite: raw.favorite.unwrap_or(false) || raw.tag.as_deref() == Some("essential"),
                auto_enter: raw.auto_enter.unwrap_or(false),
                input_mode: None,
            });
        }
    }

    if categories.is_empty() {
        return Err("FloInPalette のコマンド定義を読み込めませんでした".to_string());
    }

    Ok(AppData {
        app_name: "FloaPalette".to_string(),
        version: 1,
        categories,
    })
}

fn first_non_empty<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> String {
    values
        .into_iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("")
        .to_string()
}

fn slugify(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "category".to_string()
    } else {
        trimmed
    }
}

#[tauri::command]
fn load_app_data(app: AppHandle) -> Result<AppData, String> {
    match configured_data_path(&app) {
        Some(path) => read_data_from_path(&path),
        None => read_data(),
    }
}

#[tauri::command]
fn load_app_data_from_file(path: String) -> Result<AppData, String> {
    read_data_from_path(&PathBuf::from(path))
}

#[tauri::command]
fn save_app_data(app: AppHandle, data: AppData) -> Result<(), String> {
    let path = active_data_path(&app)?;
    write_data_to_path(&data, &path)
}

#[tauri::command]
fn get_data_file_path(app: AppHandle) -> Result<String, String> {
    Ok(active_data_path(&app)?.display().to_string())
}

#[tauri::command]
fn set_data_file_path(app: AppHandle, path: String) -> Result<(), String> {
    let selected = PathBuf::from(path);
    read_data_from_path(&selected)?;
    write_settings(
        &app,
        &AppSettings {
            data_file_path: Some(selected.display().to_string()),
        },
    )
}

#[tauri::command]
fn send_command(command: String, auto_enter: bool) -> Result<(), String> {
    thread::sleep(Duration::from_millis(120));
    win_input::send_text(&command, auto_enter)
}

#[tauri::command]
fn send_command_to_last_window(command: String, auto_enter: bool) -> Result<(), String> {
    let target = *LAST_TARGET_WINDOW.lock().map_err(|e| e.to_string())?;
    win_input::focus_window(target)?;
    thread::sleep(Duration::from_millis(120));
    win_input::send_text(&command, auto_enter)
}

#[tauri::command]
fn send_shortcut_to_last_window(shortcut: String) -> Result<(), String> {
    let target = *LAST_TARGET_WINDOW.lock().map_err(|e| e.to_string())?;
    win_input::focus_window(target)?;
    thread::sleep(Duration::from_millis(120));
    win_input::send_shortcut(&shortcut)
}

#[tauri::command]
fn show_window(window: Window) -> Result<(), String> {
    record_foreground_target();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn record_foreground_target_delayed() {
    thread::spawn(|| {
        thread::sleep(Duration::from_millis(80));
        record_foreground_target();
    });
}

fn record_foreground_target() {
    let foreground = win_input::foreground_window();
    if foreground != 0 {
        if let Ok(mut target) = LAST_TARGET_WINDOW.lock() {
            *target = foreground;
        }
    }
}

#[tauri::command]
fn hide_window(window: Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    app.exit(0);
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        record_foreground_target();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.set_always_on_top(true);
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();

            let _ = handle.global_shortcut().register("Ctrl+Shift+Space");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_app_data,
            load_app_data_from_file,
            save_app_data,
            get_data_file_path,
            set_data_file_path,
            send_command,
            send_command_to_last_window,
            send_shortcut_to_last_window,
            show_window,
            record_foreground_target_delayed,
            hide_window,
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
