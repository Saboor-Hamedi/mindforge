//! Clipboard integration and clipboard shortcuts (Cut, Copy, Paste, Select All).

use crate::app::App;
use crate::mode::Mode;
use eframe::egui;

#[cfg(target_os = "windows")]
pub fn set_win32_clipboard(text: &str) {
    #[link(name = "user32")]
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenClipboard(hWndNewOwner: *mut std::ffi::c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(uFormat: u32, hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> *mut std::ffi::c_void;
        fn GlobalLock(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalUnlock(hMem: *mut std::ffi::c_void) -> i32;
        fn GlobalFree(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }
    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;

    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = utf16.len() * std::mem::size_of::<u16>();

    unsafe {
        let mut opened = false;
        for _ in 0..10 {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        if opened {
            EmptyClipboard();
            let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if !hmem.is_null() {
                let ptr = GlobalLock(hmem) as *mut u16;
                if !ptr.is_null() {
                    std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
                    GlobalUnlock(hmem);
                    if SetClipboardData(CF_UNICODETEXT, hmem).is_null() {
                        GlobalFree(hmem);
                    }
                } else {
                    GlobalFree(hmem);
                }
            }
            CloseClipboard();
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_win32_clipboard(_text: &str) {}

#[cfg(target_os = "windows")]
pub fn get_win32_clipboard() -> Option<String> {
    #[link(name = "user32")]
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenClipboard(hWndNewOwner: *mut std::ffi::c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn GetClipboardData(uFormat: u32) -> *mut std::ffi::c_void;
        fn GlobalLock(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalUnlock(hMem: *mut std::ffi::c_void) -> i32;
    }
    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        let mut opened = false;
        for _ in 0..10 {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        if opened {
            let handle = GetClipboardData(CF_UNICODETEXT);
            let mut result = None;
            if !handle.is_null() {
                let ptr = GlobalLock(handle) as *const u16;
                if !ptr.is_null() {
                    let mut len = 0;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(ptr, len);
                    result = String::from_utf16(slice).ok();
                    GlobalUnlock(handle);
                }
            }
            CloseClipboard();
            return result;
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn get_win32_clipboard() -> Option<String> {
    None
}

pub fn get_clipboard_text(app: &App) -> Option<String> {
    if let Some(text) = get_win32_clipboard() {
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(ref text) = app.misc.clipboard_text {
        if !text.is_empty() {
            return Some(text.clone());
        }
    }
    None
}

pub fn handle_clipboard_shortcuts(app: &mut App, ctx: &egui::Context, now: f64) -> Option<bool> {
    // Select All (Ctrl+A)
    let ctrl_a = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::A));
    if ctrl_a {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
            return None;
        }
        if app.command_bar.in_command {
            app.editor.cmd_ed.select_all();
        } else if app.misc.mode == Mode::Doc {
            app.editor.doc_ed.select_all();
        } else {
            app.editor.ed.select_all();
            return Some(true);
        }
        app.misc.sound.play();
        app.set_status("Selected all text (Ctrl+A)", now);
        return Some(false);
    }

    // Clipboard Copy (Ctrl+C)
    let ctrl_c = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::C));
    if ctrl_c {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
            return None;
        }
        let text = if app.command_bar.in_command {
            app.editor.cmd_ed.selected_text()
        } else if app.misc.mode == Mode::Doc {
            app.editor.doc_ed.selected_text()
        } else {
            app.editor.ed.selected_text()
        };
        if let Some(t) = text {
            app.misc.clipboard_text = Some(t.clone());
            set_win32_clipboard(&t);
            ctx.copy_text(t);
            app.misc.sound.play();
            app.set_status("Copied selection to clipboard", now);
            return Some(false);
        }
        return Some(false);
    }

    // Clipboard Cut (Ctrl+X)
    let ctrl_x = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::X));
    if ctrl_x {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
            return None;
        }
        let text = if app.command_bar.in_command {
            let t = app.editor.cmd_ed.selected_text();
            app.editor.cmd_ed.delete_selection();
            t
        } else if app.misc.mode == Mode::Doc {
            let t = app.editor.doc_ed.selected_text();
            if app.editor.doc_ed.delete_selection() {
                app.misc.sound.play();
                app.set_status("Cut selection", now);
            }
            t
        } else {
            let t = app.editor.ed.selected_text();
            if app.editor.ed.delete_selection() {
                app.editor.is_dirty = true;
                app.misc.sound.play();
                app.set_status("Cut selection", now);
            }
            t
        };
        if let Some(t) = text {
            app.misc.clipboard_text = Some(t.clone());
            set_win32_clipboard(&t);
            ctx.copy_text(t);
            return Some(true);
        }
        return Some(false);
    }

    // Clipboard Paste (Ctrl+V)
    let ctrl_v = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::V));
    if ctrl_v {
        if let Some(text) = get_clipboard_text(app) {
            if app.command_bar.in_command {
                crate::command::input::handle_command_paste(app, &text, now);
                return Some(true);
            } else if app.modal.search_open {
                app.modal.search_query.push_str(&text);
                app.modal.search_selected = 0;
                app.update_search_results();
                return Some(true);
            } else if app.modal.rename_open {
                app.modal.rename_input.push_str(&text);
                return Some(true);
            } else if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
                && app.misc.mode == Mode::Normal && !app.command_bar.in_command
            {
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    let _ = backend.paste(&text);
                } else {
                    app.services.vim_runtime.queue_input(crate::vim::PendingVimInput::Paste(text));
                }
                return Some(true);
            } else if app.misc.mode == Mode::Normal || app.misc.mode == Mode::Doc {
                if crate::input::editor::handle_editor_paste(app, &text, now) {
                    return Some(true);
                }
            }
        }
        return Some(false);
    }

    None
}
