//! Asynchronous MessagePack-RPC transport for an embedded Neovim process.
//! Pipe I/O and RPC decoding run on dedicated threads with unbounded channels
//! and direct egui repaint wakeup so the egui UI thread never blocks or starves.

use rmpv::Value;
use std::{
    collections::{HashMap, VecDeque},
    io::Write,
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Instant,
};

type RpcResult = Result<Value, String>;

struct Request {
    id: u64,
    method: String,
    args: Vec<Value>,
    reply: Option<mpsc::Sender<RpcResult>>,
}

pub struct NeovimClient {
    requests: mpsc::Sender<Request>,
    events: mpsc::Receiver<Value>,
    pending_events: VecDeque<Value>,
    next_id: u64,
    child: Option<Child>,
    repaint_ctx: Arc<Mutex<Option<eframe::egui::Context>>>,
    config_path: Option<std::path::PathBuf>,
    config_modified: Option<std::time::SystemTime>,
    last_config_check: Instant,
}

impl NeovimClient {
    pub fn start(repaint_ctx: Option<eframe::egui::Context>) -> Result<Self, String> {
        let mut cmd = Command::new("nvim");
        cmd.arg("--embed");
        // Load MindForge's dedicated lightweight Neovim config when present.
        // This is distinct from a user's full Neovim setup, which can launch
        // dashboards or other UI that does not belong inside the editor pane.
        let app_nvim_config = std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .map(|path| path.join("mindforge").join("mindforge").join("nvim").join("init.lua"))
            .filter(|path| path.is_file());
        if let Some(config) = &app_nvim_config {
            cmd.arg("-u").arg(config);
        } else {
            cmd.arg("--clean");
        }
        let config_modified = app_nvim_config
            .as_ref()
            .and_then(|path| std::fs::metadata(path).ok())
            .and_then(|metadata| metadata.modified().ok());
        cmd.args(["--cmd", "set mouse=a shortmess+=I nomore noswapfile"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().map_err(|e| format!("Could not start Neovim: {e}"))?;
        let stdin = child.stdin.take().ok_or("Neovim stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("Neovim stdout unavailable")?;

        let pending: Arc<Mutex<HashMap<u64, mpsc::Sender<RpcResult>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_writer = Arc::clone(&pending);
        let pending_reader = Arc::clone(&pending);

        let (requests, request_rx) = mpsc::channel::<Request>();
        let (events_tx, events) = mpsc::channel::<Value>();

        let repaint_ctx_shared = Arc::new(Mutex::new(repaint_ctx));
        let repaint_ctx_reader = Arc::clone(&repaint_ctx_shared);
        let repaint_gate = Arc::new(Mutex::new(None::<Instant>));
        let repaint_gate_reader = Arc::clone(&repaint_gate);

        // Dedicated RPC writer thread
        thread::Builder::new()
            .name("nvim-rpc-writer".into())
            .spawn(move || {
                let mut input = stdin;
                while let Ok(request) = request_rx.recv() {
                    let packet = if let Some(reply) = request.reply {
                        if let Ok(mut map) = pending_writer.lock() {
                            map.insert(request.id, reply);
                        }
                        Value::Array(vec![
                            Value::from(0), // Request
                            Value::from(request.id),
                            Value::from(request.method),
                            Value::Array(request.args),
                        ])
                    } else {
                        Value::Array(vec![
                            Value::from(2), // Notification
                            Value::from(request.method),
                            Value::Array(request.args),
                        ])
                    };

                    let result = rmpv::encode::write_value(&mut input, &packet)
                        .map_err(|e| e.to_string())
                        .and_then(|_| input.flush().map_err(|e| e.to_string()));

                    if let Err(error) = result {
                        if let Some(reply) = pending_writer
                            .lock()
                            .ok()
                            .and_then(|mut map| map.remove(&request.id))
                        {
                            let _ = reply.send(Err(error));
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        // Dedicated RPC reader thread
        thread::Builder::new()
            .name("nvim-rpc-reader".into())
            .spawn(move || {
                let mut output = stdout;
                loop {
                    let packet = match rmpv::decode::read_value(&mut output) {
                        Ok(v) => v,
                        Err(error) => {
                            // Fail outstanding RPCs as soon as Neovim closes
                            // its pipe. Otherwise startup can sit in
                            // recv_timeout while the UI has no event to wake it.
                            let message = format!("Neovim RPC connection closed: {error}");
                            if let Ok(mut pending) = pending_reader.lock() {
                                for (_, reply) in pending.drain() {
                                    let _ = reply.send(Err(message.clone()));
                                }
                            }
                            if let Ok(guard) = repaint_ctx_reader.lock() {
                                if let Some(ctx) = guard.as_ref() {
                                    ctx.request_repaint();
                                }
                            }
                            break;
                        }
                    };
                    let Some(items) = packet.as_array() else {
                        continue;
                    };
                    match items.first().and_then(Value::as_u64) {
                        Some(1) if items.len() >= 4 => {
                            // Response to request
                            let id = items[1].as_u64().unwrap_or(0);
                            let error = &items[2];
                            let response = if error.is_nil() {
                                Ok(items[3].clone())
                            } else {
                                Err(error.to_string())
                            };
                            if let Ok(mut map) = pending_reader.lock() {
                                if let Some(tx) = map.remove(&id) {
                                    let _ = tx.send(response);
                                }
                            }
                        }
                        Some(2) => {
                            // Notification (redraw, line events, etc.)
                            let _ = events_tx.send(packet);
                        }
                        _ => {}
                    }

                    // Neovim can emit a large startup redraw burst. Gate wakeups
                    // to 60fps so the UI is not continuously rescheduled for
                    // every packet in that burst.
                    let should_repaint = repaint_gate_reader
                        .lock()
                        .map(|mut last| {
                            let now = Instant::now();
                            if last.is_none_or(|previous| {
                                now.duration_since(previous)
                                    >= std::time::Duration::from_millis(16)
                            }) {
                                *last = Some(now);
                                true
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false);
                    if should_repaint {
                        if let Ok(guard) = repaint_ctx_reader.lock() {
                            if let Some(ctx) = guard.as_ref() {
                                ctx.request_repaint();
                            }
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        let client = Self {
            requests,
            events,
            pending_events: VecDeque::new(),
            next_id: 1,
            child: Some(child),
            repaint_ctx: repaint_ctx_shared,
            config_path: app_nvim_config,
            config_modified,
            last_config_check: Instant::now(),
        };
        Ok(client)
    }

    pub fn set_repaint_context(&self, ctx: eframe::egui::Context) {
        if let Ok(mut guard) = self.repaint_ctx.lock() {
            *guard = Some(ctx);
        }
    }

    pub fn attach_ui(&mut self, width: usize, height: usize) -> Result<(), String> {
        self.request(
            "nvim_ui_attach",
            vec![
                Value::from(width.max(1) as u64),
                Value::from(height.max(1) as u64),
                Value::Map(vec![
                    (Value::from("ext_linegrid"), Value::from(true)),
                    (Value::from("ext_popupmenu"), Value::from(true)),
                ]),
            ],
        )?;
        Ok(())
    }

    pub fn request(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let (result_tx, result_rx) = mpsc::channel();
        let req = Request {
            id,
            method: method.into(),
            args,
            reply: Some(result_tx),
        };
        self.requests
            .send(req)
            .map_err(|_| "Neovim RPC writer queue stopped".to_string())?;
        result_rx
            .recv_timeout(std::time::Duration::from_secs(15))
            .map_err(|_| "Timed out waiting for Neovim RPC response (15s)".to_string())?
    }

    pub fn notify(&self, method: &str, args: Vec<Value>) -> Result<(), String> {
        let id = self.next_id;
        self.requests
            .send(Request {
                id,
                method: method.into(),
                args,
                reply: None,
            })
            .map_err(|_| "Neovim RPC writer queue stopped".to_string())
    }

    pub fn drain_events(&mut self) -> Vec<Value> {
        // Cap how many events we apply per egui frame. Keep the remainder in
        // `pending_events`; draining the whole backlog here would make the
        // apparent cap ineffective during the initial redraw burst.
        const MAX_PER_TICK: usize = 200;
        while self.pending_events.len() < MAX_PER_TICK {
            match self.events.try_recv() {
                Ok(event) => self.pending_events.push_back(event),
                Err(_) => break,
            }
        }
        let count = MAX_PER_TICK.min(self.pending_events.len());
        self.pending_events.drain(..count).collect()
    }

    pub fn input(&mut self, input: &str) -> Result<(), String> {
        self.notify("nvim_input", vec![Value::from(input)])
    }

    /// `nvim_input` parses angle-bracket key notation; escape a literal less-than
    /// sign so user text such as `<Esc>` is inserted verbatim.
    pub fn input_text(&mut self, text: &str) -> Result<(), String> {
        let escaped = text.replace('<', "<LT>");
        self.input(&escaped)
    }

    pub fn paste(&mut self, text: &str) -> Result<(), String> {
        self.notify(
            "nvim_paste",
            vec![Value::from(text), Value::from(false), Value::from(-1)],
        )
    }

    pub fn input_mouse(
        &mut self,
        button: &str,
        action: &str,
        modifiers: &str,
        grid: usize,
        row: usize,
        column: usize,
    ) -> Result<(), String> {
        self.notify(
            "nvim_input_mouse",
            vec![
                Value::from(button),
                Value::from(action),
                Value::from(modifiers),
                Value::from(grid as u64),
                Value::from(row as u64),
                Value::from(column as u64),
            ],
        )
    }

    pub fn resize(&mut self, width: usize, height: usize) -> Result<(), String> {
        self.notify(
            "nvim_ui_try_resize",
            vec![Value::from(width as u64), Value::from(height as u64)],
        )
    }

    /// Reload MindForge's lightweight init.lua after it is saved on disk.
    pub fn reload_config_if_changed(&mut self) -> Result<bool, String> {
        if self.last_config_check.elapsed() < std::time::Duration::from_millis(400) {
            return Ok(false);
        }
        self.last_config_check = Instant::now();
        let Some(path) = self.config_path.as_ref() else {
            return Ok(false);
        };
        let Ok(metadata) = std::fs::metadata(path) else {
            return Ok(false);
        };
        let modified = metadata.modified().ok();
        if modified.is_none() || modified == self.config_modified {
            return Ok(false);
        }
        let path_string = path.to_string_lossy().into_owned();
        self.notify(
            "nvim_exec_lua",
            vec![
                Value::from("dofile(...)") ,
                Value::Array(vec![Value::from(path_string)]),
            ],
        )?;
        self.config_modified = modified;
        Ok(true)
    }

    pub fn poll_exit(&mut self) -> Result<(), String> {
        let Some(child) = self.child.as_mut() else {
            return Err("Neovim process has already been shut down".into());
        };
        match child.try_wait() {
            Ok(Some(status)) => Err(format!("Neovim exited with status {status}")),
            Ok(None) => Ok(()),
            Err(error) => Err(format!("Could not check Neovim process: {error}")),
        }
    }

    pub fn set_buffer_text(&mut self, text: &str) -> Result<(), String> {
        let lines: Vec<Value> = text.split('\n').map(Value::from).collect();
        self.request(
            "nvim_buf_set_lines",
            vec![
                Value::from(0),
                Value::from(0),
                Value::from(-1),
                Value::from(true),
                Value::Array(lines),
            ],
        )
        .map(|_| ())
    }

    pub fn set_buffer_text_async(&mut self, text: &str) -> Result<(), String> {
        let lines: Vec<Value> = text.split('\n').map(Value::from).collect();
        self.notify(
            "nvim_buf_set_lines",
            vec![
                Value::from(0),
                Value::from(0),
                Value::from(-1),
                Value::from(true),
                Value::Array(lines),
            ],
        )
    }

    pub fn set_cursor_async(&mut self, row: usize, column: usize) -> Result<(), String> {
        self.notify(
            "nvim_win_set_cursor",
            vec![
                Value::from(0),
                Value::Array(vec![
                    Value::from(row as u64 + 1),
                    Value::from(column as u64),
                ]),
            ],
        )
    }

    pub fn shutdown(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        // Reap Neovim away from egui. Killing the child is non-waiting; its
        // handle is moved to a short-lived worker for the blocking wait.
        let _ = child.kill();
        let _ = thread::Builder::new()
            .name("nvim-reaper".into())
            .spawn(move || {
                let _ = child.wait();
            });
    }
}

impl Drop for NeovimClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}
