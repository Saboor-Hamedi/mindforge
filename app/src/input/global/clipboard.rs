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
        fn GlobalSize(hMem: *mut std::ffi::c_void) -> usize;
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
                    let units = GlobalSize(handle) / std::mem::size_of::<u16>();
                    let slice = std::slice::from_raw_parts(ptr, units);
                    if let Some(len) = slice.iter().position(|&unit| unit == 0) {
                        result = String::from_utf16(&slice[..len]).ok();
                    }
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
    let ctrl_a =
        ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::A));
    if ctrl_a {
        if app.command_bar.in_command {
            app.editor.cmd_ed.select_all();
            app.misc.sound.play();
            app.set_status("Selected all text (Ctrl+A)", now);
            return Some(false);
        }
        let editor_visible = matches!(app.misc.mode, Mode::Normal | Mode::Doc)
            && (!app.misc.show_welcome || app.misc.mode == Mode::Doc);
        if !editor_visible {
            return None;
        }
        // Leave Insert/Visual mode, then visually select the whole buffer in Neovim.
        if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
            if let Err(error) = backend.send_input("<C-\\><C-n>ggVG") {
                app.set_status(&format!("Neovim input failed: {error}"), now);
                return Some(false);
            }
            app.set_status("Selected all text (Ctrl+A)", now);
            return Some(true);
        }
        return None;
    }

    // egui-winit turns Ctrl+C / Ctrl+X into `Event::Copy` / `Event::Cut` and
    // does not emit the matching key press, so accept either form.
    let ctrl_only = |m: egui::Modifiers| m.ctrl && !m.shift;
    let (ctrl_c, ctrl_x) = ctx.input(|i| {
        let copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy))
            || (ctrl_only(i.modifiers) && i.key_pressed(egui::Key::C));
        let cut = i.events.iter().any(|e| matches!(e, egui::Event::Cut))
            || (ctrl_only(i.modifiers) && i.key_pressed(egui::Key::X));
        (copy, cut)
    });

    // Clipboard Copy (Ctrl+C)
    if ctrl_c {
        if !app.command_bar.in_command {
            return None;
        }
        let text = app.editor.cmd_ed.selected_text();
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
    if ctrl_x {
        if !app.command_bar.in_command {
            return None;
        }
        let t = app.editor.cmd_ed.selected_text();
        app.editor.cmd_ed.delete_selection();
        if let Some(text) = t {
            app.misc.clipboard_text = Some(text.clone());
            set_win32_clipboard(&text);
            ctx.copy_text(text);
            app.misc.sound.play();
            app.set_status("Cut selection", now);
            return Some(true);
        }
        return Some(false);
    }

    // Clipboard Paste (Ctrl+V)
    let ctrl_v =
        ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::V));
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
            } else if app.misc.mode == Mode::Normal && !app.command_bar.in_command {
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    let _ = backend.paste(&text);
                } else {
                    app.services
                        .vim_runtime
                        .queue_input(crate::vim::PendingVimInput::Paste(text));
                }
                return Some(true);
            }
        }
        return Some(false);
    }

    None
}
