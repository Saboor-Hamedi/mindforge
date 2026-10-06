//! Window shortcuts and borderless edge/corner resize management.

use eframe::egui::{self, CursorIcon, Key, ResizeDirection, ViewportCommand};

pub fn window_shortcuts(ctx: &egui::Context) {
    let (drag, f11, quit, new_window, is_fs) = ctx.input(|i| {
        (
            i.modifiers.alt && i.pointer.primary_pressed(),
            i.key_pressed(Key::F11),
            (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::W))
                || (i.modifiers.ctrl && i.key_pressed(Key::Q)),
            (i.modifiers.ctrl || i.modifiers.command)
                && i.modifiers.shift
                && !i.modifiers.alt
                && i.key_pressed(Key::N),
            i.viewport().fullscreen.unwrap_or(false),
        )
    });

    if new_window {
        if let Ok(exe) = std::env::current_exe() {
            let (pos_x, pos_y) = ctx.input(|i| {
                if let Some(pos) = i.viewport().outer_rect.map(|r| r.min) {
                    (pos.x + 36.0, pos.y + 36.0)
                } else {
                    (100.0, 100.0)
                }
            });
            let _ = std::process::Command::new(exe)
                .arg("--new-window")
                .arg("--pos-x")
                .arg(pos_x.to_string())
                .arg("--pos-y")
                .arg(pos_y.to_string())
                .spawn();
        }
    }
    if drag {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if f11 {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!is_fs));
    }
    if quit {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    // Borderless window edge & corner resize grips (6px borders)
    if !is_fs {
        let screen = ctx.screen_rect();
        if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
            let margin = 6.0;
            let on_right = pos.x >= screen.max.x - margin && pos.x <= screen.max.x + 2.0;
            let on_bottom = pos.y >= screen.max.y - margin && pos.y <= screen.max.y + 2.0;
            let on_left = pos.x <= screen.min.x + margin && pos.x >= screen.min.x - 2.0;

            let resize_dir = if on_right && on_bottom {
                Some((ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast))
            } else if on_left && on_bottom {
                Some((ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest))
            } else if on_right {
                Some((ResizeDirection::East, CursorIcon::ResizeEast))
            } else if on_bottom {
                Some((ResizeDirection::South, CursorIcon::ResizeSouth))
            } else if on_left {
                Some((ResizeDirection::West, CursorIcon::ResizeWest))
            } else {
                None
            };

            if let Some((dir, cursor)) = resize_dir {
                ctx.set_cursor_icon(cursor);
                if ctx.input(|i| i.pointer.primary_pressed()) {
                    ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                }
            }
        }
    }
}
