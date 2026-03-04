//! LSP (Language Server Protocol) client for code intelligence.
//!
//! Supports rust-analyzer, pyright, tsserver and other LSP-compatible servers.
//! Communicates via JSON-RPC over stdin/stdout.

use lsp_types::Uri;
use lsp_types::*;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use url::Url;

/// Messages from the LSP server to the UI thread.
#[derive(Debug, Clone)]
pub enum LspEvent {
    /// Diagnostics for a file.
    Diagnostics {
        uri: String,
        diagnostics: Vec<LspDiagnostic>,
    },
    /// Completion results.
    Completions {
        request_id: i64,
        items: Vec<LspCompletionItem>,
    },
    /// Hover result.
    Hover { request_id: i64, contents: String },
    /// Go to definition result.
    Definition {
        request_id: i64,
        location: Option<LspLocation>,
    },
    /// Find references result.
    References {
        request_id: i64,
        locations: Vec<LspLocation>,
    },
    /// Rename result (workspace edit).
    Rename {
        request_id: i64,
        edit: Option<LspWorkspaceEdit>,
    },
    /// Server initialized successfully.
    Initialized,
    /// Server exited or errored.
    ServerError(String),
}

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub message: String,
    pub severity: DiagnosticSeverityLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticSeverityLevel {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct LspCompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LspLocation {
    pub uri: String,
    pub line: usize,
    pub col: usize,
}

/// A text edit within a single file (for rename results).
#[derive(Debug, Clone)]
pub struct LspTextEdit {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub new_text: String,
}

/// A workspace edit returned from rename — edits grouped by file URI.
#[derive(Debug, Clone)]
pub struct LspWorkspaceEdit {
    /// Map of file URI to list of text edits for that file.
    pub changes: HashMap<String, Vec<LspTextEdit>>,
}

/// State for the references results panel.
pub struct ReferencesState {
    pub visible: bool,
    pub locations: Vec<LspLocation>,
    /// Scroll offset for the results list.
    pub scroll: f32,
}

impl Default for ReferencesState {
    fn default() -> Self {
        Self {
            visible: false,
            locations: Vec::new(),
            scroll: 0.0,
        }
    }
}

/// State for the inline rename dialog.
#[derive(Default)]
pub struct RenameDialogState {
    pub visible: bool,
    pub input: String,
    /// Whether the text input should request focus on the next frame.
    pub needs_focus: bool,
}

/// State for a single LSP server connection.
struct LspServer {
    process: Child,
    stdin: std::process::ChildStdin,
    next_id: i64,
    #[allow(dead_code)]
    root_uri: String,
    _reader_thread: std::thread::JoinHandle<()>,
}

/// Manages LSP server connections for different languages.
pub struct LspManager {
    servers: HashMap<String, LspServer>,
    event_tx: mpsc::Sender<LspEvent>,
    event_rx: mpsc::Receiver<LspEvent>,
    /// File version counters for didChange notifications.
    file_versions: HashMap<String, i32>,
    /// Cached diagnostics per file URI.
    pub diagnostics: HashMap<String, Vec<LspDiagnostic>>,
    /// Pending completion results.
    pub pending_completions: Option<Vec<LspCompletionItem>>,
    /// Pending hover result.
    pub pending_hover: Option<String>,
    /// Pending definition result.
    pub pending_definition: Option<LspLocation>,
    /// Pending references results.
    pub pending_references: Option<Vec<LspLocation>>,
    /// Pending rename workspace edit.
    pub pending_rename: Option<LspWorkspaceEdit>,
    /// Last completion request ID (to discard stale responses).
    last_completion_id: i64,
    last_hover_id: i64,
    last_definition_id: i64,
    last_references_id: i64,
    last_rename_id: i64,
    /// Tracks which request IDs are references vs definition requests.
    /// Shared with reader threads so they can disambiguate Location[] responses.
    pending_request_types: Arc<Mutex<HashMap<i64, &'static str>>>,
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LspManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            servers: HashMap::new(),
            event_tx: tx,
            event_rx: rx,
            file_versions: HashMap::new(),
            diagnostics: HashMap::new(),
            pending_completions: None,
            pending_hover: None,
            pending_definition: None,
            pending_references: None,
            pending_rename: None,
            last_completion_id: 0,
            last_hover_id: 0,
            last_definition_id: 0,
            last_references_id: 0,
            last_rename_id: 0,
            pending_request_types: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Poll for events from LSP servers. Call this every frame.
    pub fn poll_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                LspEvent::Diagnostics { uri, diagnostics } => {
                    self.diagnostics.insert(uri, diagnostics);
                }
                LspEvent::Completions { request_id, items } => {
                    if request_id == self.last_completion_id {
                        self.pending_completions = Some(items);
                    }
                }
                LspEvent::Hover {
                    request_id,
                    contents,
                } => {
                    if request_id == self.last_hover_id {
                        self.pending_hover = Some(contents);
                    }
                }
                LspEvent::Definition {
                    request_id,
                    location,
                } => {
                    if request_id == self.last_definition_id {
                        self.pending_definition = location;
                    }
                }
                LspEvent::References {
                    request_id,
                    locations,
                } => {
                    if request_id == self.last_references_id {
                        self.pending_references = Some(locations);
                    }
                }
                LspEvent::Rename { request_id, edit } => {
                    if request_id == self.last_rename_id {
                        self.pending_rename = edit;
                    }
                }
                LspEvent::Initialized => {
                    log::info!("LSP server initialized");
                }
                LspEvent::ServerError(msg) => {
                    log::error!("LSP server error: {}", msg);
                }
            }
        }
    }

    /// Get the LSP server command for a language.
    fn server_command(language: &str) -> Option<(&'static str, Vec<&'static str>)> {
        match language {
            "Rust" => Some(("rust-analyzer", vec![])),
            "Python" => Some(("pyright-langserver", vec!["--stdio"])),
            "JavaScript" | "TypeScript" | "JSX" | "TSX" => {
                Some(("typescript-language-server", vec!["--stdio"]))
            }
            "Go" => Some(("gopls", vec![])),
            "C" | "C++" => Some(("clangd", vec![])),
            "Lua" => Some(("lua-language-server", vec![])),
            _ => None,
        }
    }

    /// Ensure an LSP server is running for the given language and workspace root.
    pub fn ensure_server(&mut self, language: &str, workspace_root: &Path) {
        if self.servers.contains_key(language) {
            return;
        }

        let Some((cmd, args)) = Self::server_command(language) else {
            return;
        };

        // Check if the server binary exists
        if Command::new(cmd)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_err()
        {
            // Try which
            if Command::new("which")
                .arg(cmd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_or(true, |s| !s.success())
            {
                log::warn!("LSP server '{}' not found for language '{}'", cmd, language);
                return;
            }
        }

        log::info!("Starting LSP server '{}' for '{}'", cmd, language);

        let mut child = match Command::new(cmd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .current_dir(workspace_root)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to start LSP server '{}': {}", cmd, e);
                return;
            }
        };

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let root_uri = Url::from_file_path(workspace_root)
            .unwrap_or_else(|_| Url::parse("file:///").unwrap())
            .to_string();

        let tx = self.event_tx.clone();
        let req_types = self.pending_request_types.clone();
        let reader_thread = std::thread::spawn(move || {
            read_lsp_messages(stdout, tx, req_types);
        });

        let mut server = LspServer {
            process: child,
            stdin,
            next_id: 1,
            root_uri: root_uri.clone(),
            _reader_thread: reader_thread,
        };

        // Send initialize request
        let id = server.next_id;
        server.next_id += 1;

        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(root_uri.parse::<Uri>().unwrap()),
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    completion: Some(CompletionClientCapabilities {
                        completion_item: Some(CompletionItemCapability {
                            snippet_support: Some(false),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    hover: Some(HoverClientCapabilities {
                        ..Default::default()
                    }),
                    publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                        ..Default::default()
                    }),
                    definition: Some(GotoCapability {
                        ..Default::default()
                    }),
                    references: Some(DynamicRegistrationClientCapabilities {
                        ..Default::default()
                    }),
                    rename: Some(RenameClientCapabilities {
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": params,
        });
        send_message(&mut server.stdin, &msg);

        self.servers.insert(language.to_string(), server);
    }

    /// Notify the server that a file was opened.
    pub fn did_open(&mut self, language: &str, uri: &str, text: &str) {
        let lang_id = match language {
            "Rust" => "rust",
            "Python" => "python",
            "JavaScript" => "javascript",
            "TypeScript" => "typescript",
            "Go" => "go",
            "C" => "c",
            "C++" => "cpp",
            "Lua" => "lua",
            _ => language,
        };

        self.file_versions.insert(uri.to_string(), 1);

        if let Some(server) = self.servers.get_mut(language) {
            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": lang_id,
                        "version": 1,
                        "text": text,
                    }
                }
            });
            send_message(&mut server.stdin, &msg);
        }
    }

    /// Notify the server that a file's content changed.
    pub fn did_change(&mut self, language: &str, uri: &str, text: &str) {
        let version = self.file_versions.entry(uri.to_string()).or_insert(1);
        *version += 1;
        let v = *version;

        if let Some(server) = self.servers.get_mut(language) {
            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": v,
                    },
                    "contentChanges": [{
                        "text": text,
                    }]
                }
            });
            send_message(&mut server.stdin, &msg);
        }
    }

    /// Request completions at a position.
    pub fn request_completion(&mut self, language: &str, uri: &str, line: u32, col: u32) {
        if let Some(server) = self.servers.get_mut(language) {
            let id = server.next_id;
            server.next_id += 1;
            self.last_completion_id = id;

            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": col },
                }
            });
            send_message(&mut server.stdin, &msg);
        }
    }

    /// Request hover info at a position.
    pub fn request_hover(&mut self, language: &str, uri: &str, line: u32, col: u32) {
        if let Some(server) = self.servers.get_mut(language) {
            let id = server.next_id;
            server.next_id += 1;
            self.last_hover_id = id;

            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": col },
                }
            });
            send_message(&mut server.stdin, &msg);
        }
    }

    /// Request go-to-definition at a position.
    pub fn request_definition(&mut self, language: &str, uri: &str, line: u32, col: u32) {
        if let Some(server) = self.servers.get_mut(language) {
            let id = server.next_id;
            server.next_id += 1;
            self.last_definition_id = id;
            if let Ok(mut map) = self.pending_request_types.lock() {
                map.insert(id, "definition");
            }

            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/definition",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": col },
                }
            });
            send_message(&mut server.stdin, &msg);
        }
    }

    /// Request find references at a position (Shift+F12).
    pub fn request_references(&mut self, language: &str, uri: &str, line: u32, col: u32) {
        if let Some(server) = self.servers.get_mut(language) {
            let id = server.next_id;
            server.next_id += 1;
            self.last_references_id = id;
            if let Ok(mut map) = self.pending_request_types.lock() {
                map.insert(id, "references");
            }

            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/references",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": col },
                    "context": { "includeDeclaration": true },
                }
            });
            send_message(&mut server.stdin, &msg);
        }
    }

    /// Request rename symbol at a position (F2).
    pub fn request_rename(
        &mut self,
        language: &str,
        uri: &str,
        line: u32,
        col: u32,
        new_name: &str,
    ) {
        if let Some(server) = self.servers.get_mut(language) {
            let id = server.next_id;
            server.next_id += 1;
            self.last_rename_id = id;
            if let Ok(mut map) = self.pending_request_types.lock() {
                map.insert(id, "rename");
            }

            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/rename",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": col },
                    "newName": new_name,
                }
            });
            send_message(&mut server.stdin, &msg);
        }
    }

    /// Get diagnostics for a file path.
    pub fn get_diagnostics(&self, path: &Path) -> &[LspDiagnostic] {
        let uri = Url::from_file_path(path)
            .map(|u| u.to_string())
            .unwrap_or_default();
        self.diagnostics
            .get(&uri)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Take pending completions (clears them).
    pub fn take_completions(&mut self) -> Option<Vec<LspCompletionItem>> {
        self.pending_completions.take()
    }

    /// Take pending hover (clears it).
    pub fn take_hover(&mut self) -> Option<String> {
        self.pending_hover.take()
    }

    /// Take pending definition (clears it).
    pub fn take_definition(&mut self) -> Option<LspLocation> {
        self.pending_definition.take()
    }

    /// Take pending references (clears them).
    pub fn take_references(&mut self) -> Option<Vec<LspLocation>> {
        self.pending_references.take()
    }

    /// Take pending rename result (clears it).
    pub fn take_rename(&mut self) -> Option<LspWorkspaceEdit> {
        self.pending_rename.take()
    }

    /// Shut down all servers.
    pub fn shutdown_all(&mut self) {
        for (lang, mut server) in self.servers.drain() {
            log::info!("Shutting down LSP server for '{}'", lang);
            let id = server.next_id;
            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "shutdown",
                "params": null,
            });
            send_message(&mut server.stdin, &msg);
            // Send exit notification
            let exit_msg = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null,
            });
            send_message(&mut server.stdin, &exit_msg);
            let _ = server.process.wait();
        }
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}

/// Send a JSON-RPC message to the LSP server via stdin.
fn send_message(stdin: &mut std::process::ChildStdin, msg: &serde_json::Value) {
    let body = serde_json::to_string(msg).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let _ = stdin.write_all(header.as_bytes());
    let _ = stdin.write_all(body.as_bytes());
    let _ = stdin.flush();
}

/// Read LSP messages from stdout and send events.
fn read_lsp_messages(
    stdout: std::process::ChildStdout,
    tx: mpsc::Sender<LspEvent>,
    request_types: Arc<Mutex<HashMap<i64, &'static str>>>,
) {
    let mut reader = BufReader::new(stdout);
    let mut header_buf = String::new();

    loop {
        // Read headers
        let mut content_length: usize = 0;
        loop {
            header_buf.clear();
            match reader.read_line(&mut header_buf) {
                Ok(0) => return, // EOF
                Ok(_) => {}
                Err(_) => return,
            }
            let trimmed = header_buf.trim();
            if trimmed.is_empty() {
                break; // End of headers
            }
            if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
                if let Ok(len) = len_str.parse::<usize>() {
                    content_length = len;
                }
            }
        }

        if content_length == 0 {
            continue;
        }

        // Read body
        let mut body = vec![0u8; content_length];
        if std::io::Read::read_exact(&mut reader, &mut body).is_err() {
            return;
        }

        let Ok(msg) = serde_json::from_slice::<serde_json::Value>(&body) else {
            continue;
        };

        // Handle the message
        if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
            if method == "textDocument/publishDiagnostics" {
                if let Some(params) = msg.get("params") {
                    let uri = params
                        .get("uri")
                        .and_then(|u| u.as_str())
                        .unwrap_or("")
                        .to_string();
                    let diags = params
                        .get("diagnostics")
                        .and_then(|d| d.as_array())
                        .map(|arr| arr.iter().filter_map(parse_diagnostic).collect())
                        .unwrap_or_default();
                    let _ = tx.send(LspEvent::Diagnostics {
                        uri,
                        diagnostics: diags,
                    });
                }
            }
        } else if let Some(id) = msg.get("id").and_then(|i| i.as_i64()) {
            // Look up what kind of request this response is for
            let req_type = request_types
                .lock()
                .ok()
                .and_then(|mut map| map.remove(&id));

            // Response to a request
            if let Some(result) = msg.get("result") {
                // Check for rename response (WorkspaceEdit) first — unique shape
                if req_type == Some("rename") {
                    if let Some(edit) = parse_rename_response(result) {
                        let _ = tx.send(LspEvent::Rename {
                            request_id: id,
                            edit: Some(edit),
                        });
                    } else {
                        let _ = tx.send(LspEvent::Rename {
                            request_id: id,
                            edit: None,
                        });
                    }
                } else if req_type == Some("references") {
                    // References returns Location[] or null
                    let locations = parse_locations_response(result);
                    let _ = tx.send(LspEvent::References {
                        request_id: id,
                        locations,
                    });
                } else if let Some(items) = parse_completion_response(result) {
                    let _ = tx.send(LspEvent::Completions {
                        request_id: id,
                        items,
                    });
                } else if let Some(hover_text) = parse_hover_response(result) {
                    let _ = tx.send(LspEvent::Hover {
                        request_id: id,
                        contents: hover_text,
                    });
                } else if let Some(location) = parse_definition_response(result) {
                    let _ = tx.send(LspEvent::Definition {
                        request_id: id,
                        location: Some(location),
                    });
                } else if result.is_null() {
                    // Could be a null hover/definition response
                    // Check if it might be an initialize response
                    if result.get("capabilities").is_some()
                        || msg
                            .get("result")
                            .and_then(|r| r.get("capabilities"))
                            .is_some()
                    {
                        // Send initialized notification
                        let _ = tx.send(LspEvent::Initialized);
                    }
                }

                // Check for initialize response
                if result.get("capabilities").is_some() {
                    let _ = tx.send(LspEvent::Initialized);
                }
            }
        }
    }
}

fn parse_diagnostic(value: &serde_json::Value) -> Option<LspDiagnostic> {
    let range = value.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;

    let severity = match value.get("severity").and_then(|s| s.as_u64()) {
        Some(1) => DiagnosticSeverityLevel::Error,
        Some(2) => DiagnosticSeverityLevel::Warning,
        Some(3) => DiagnosticSeverityLevel::Info,
        _ => DiagnosticSeverityLevel::Hint,
    };

    Some(LspDiagnostic {
        line: start.get("line")?.as_u64()? as usize,
        col: start.get("character")?.as_u64()? as usize,
        end_line: end.get("line")?.as_u64()? as usize,
        end_col: end.get("character")?.as_u64()? as usize,
        message: value.get("message")?.as_str()?.to_string(),
        severity,
    })
}

fn parse_completion_response(result: &serde_json::Value) -> Option<Vec<LspCompletionItem>> {
    // CompletionList or array of CompletionItem
    let items_val = if let Some(items) = result.get("items") {
        items.as_array()?
    } else if result.is_array() {
        result.as_array()?
    } else {
        return None;
    };

    let items: Vec<LspCompletionItem> = items_val
        .iter()
        .map(|item| {
            let label = item
                .get("label")
                .and_then(|l| l.as_str())
                .unwrap_or("")
                .to_string();
            let detail = item
                .get("detail")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            let kind = item
                .get("kind")
                .and_then(|k| k.as_u64())
                .map(completion_kind_name);
            let insert_text = item
                .get("insertText")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());
            LspCompletionItem {
                label,
                detail,
                kind,
                insert_text,
            }
        })
        .collect();

    Some(items)
}

fn parse_hover_response(result: &serde_json::Value) -> Option<String> {
    let contents = result.get("contents")?;

    // MarkedString, MarkupContent, or array
    if let Some(value) = contents.get("value") {
        return value.as_str().map(|s| s.to_string());
    }
    if let Some(s) = contents.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = contents.as_array() {
        let parts: Vec<String> = arr
            .iter()
            .filter_map(|v| {
                v.get("value")
                    .and_then(|s| s.as_str())
                    .or_else(|| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();
        if !parts.is_empty() {
            return Some(parts.join("\n\n"));
        }
    }
    None
}

fn parse_definition_response(result: &serde_json::Value) -> Option<LspLocation> {
    // Can be Location, Location[], or LocationLink[]
    let loc = if result.is_array() {
        result.as_array()?.first()?
    } else if result.get("uri").is_some() {
        result
    } else if result.get("targetUri").is_some() {
        // LocationLink
        let uri = result.get("targetUri")?.as_str()?.to_string();
        let range = result.get("targetRange")?;
        let start = range.get("start")?;
        return Some(LspLocation {
            uri,
            line: start.get("line")?.as_u64()? as usize,
            col: start.get("character")?.as_u64()? as usize,
        });
    } else {
        return None;
    };

    let uri = loc.get("uri")?.as_str()?.to_string();
    let range = loc.get("range")?;
    let start = range.get("start")?;
    Some(LspLocation {
        uri,
        line: start.get("line")?.as_u64()? as usize,
        col: start.get("character")?.as_u64()? as usize,
    })
}

/// Parse a Location[] response into a list of LspLocation (used for references).
fn parse_locations_response(result: &serde_json::Value) -> Vec<LspLocation> {
    if result.is_null() {
        return Vec::new();
    }

    let arr = if result.is_array() {
        result.as_array()
    } else {
        None
    };

    let Some(items) = arr else {
        // Single location
        if let Some(loc) = parse_single_location(result) {
            return vec![loc];
        }
        return Vec::new();
    };

    items.iter().filter_map(parse_single_location).collect()
}

/// Parse a single Location or LocationLink JSON value.
fn parse_single_location(value: &serde_json::Value) -> Option<LspLocation> {
    if let Some(uri) = value.get("uri").and_then(|u| u.as_str()) {
        let range = value.get("range")?;
        let start = range.get("start")?;
        return Some(LspLocation {
            uri: uri.to_string(),
            line: start.get("line")?.as_u64()? as usize,
            col: start.get("character")?.as_u64()? as usize,
        });
    }
    if let Some(uri) = value.get("targetUri").and_then(|u| u.as_str()) {
        let range = value.get("targetRange")?;
        let start = range.get("start")?;
        return Some(LspLocation {
            uri: uri.to_string(),
            line: start.get("line")?.as_u64()? as usize,
            col: start.get("character")?.as_u64()? as usize,
        });
    }
    None
}

/// Parse a WorkspaceEdit from a rename response.
fn parse_rename_response(result: &serde_json::Value) -> Option<LspWorkspaceEdit> {
    if result.is_null() {
        return None;
    }

    let mut changes: HashMap<String, Vec<LspTextEdit>> = HashMap::new();

    // Standard WorkspaceEdit with "changes" field: { uri: TextEdit[] }
    if let Some(changes_obj) = result.get("changes").and_then(|c| c.as_object()) {
        for (uri, edits_val) in changes_obj {
            if let Some(edits_arr) = edits_val.as_array() {
                let edits: Vec<LspTextEdit> =
                    edits_arr.iter().filter_map(parse_text_edit).collect();
                if !edits.is_empty() {
                    changes.insert(uri.clone(), edits);
                }
            }
        }
    }

    // WorkspaceEdit with "documentChanges" field (TextDocumentEdit[])
    if let Some(doc_changes) = result.get("documentChanges").and_then(|d| d.as_array()) {
        for doc_change in doc_changes {
            let uri = doc_change
                .get("textDocument")
                .and_then(|td| td.get("uri"))
                .and_then(|u| u.as_str());
            let edits = doc_change.get("edits").and_then(|e| e.as_array());
            if let (Some(uri), Some(edits_arr)) = (uri, edits) {
                let edits: Vec<LspTextEdit> =
                    edits_arr.iter().filter_map(parse_text_edit).collect();
                if !edits.is_empty() {
                    changes.entry(uri.to_string()).or_default().extend(edits);
                }
            }
        }
    }

    if changes.is_empty() {
        return None;
    }

    Some(LspWorkspaceEdit { changes })
}

/// Parse a single LSP TextEdit JSON object.
fn parse_text_edit(value: &serde_json::Value) -> Option<LspTextEdit> {
    let range = value.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;
    let new_text = value.get("newText")?.as_str()?.to_string();

    Some(LspTextEdit {
        start_line: start.get("line")?.as_u64()? as usize,
        start_col: start.get("character")?.as_u64()? as usize,
        end_line: end.get("line")?.as_u64()? as usize,
        end_col: end.get("character")?.as_u64()? as usize,
        new_text,
    })
}

fn completion_kind_name(kind: u64) -> String {
    match kind {
        1 => "Text",
        2 => "Method",
        3 => "Function",
        4 => "Constructor",
        5 => "Field",
        6 => "Variable",
        7 => "Class",
        8 => "Interface",
        9 => "Module",
        10 => "Property",
        11 => "Unit",
        12 => "Value",
        13 => "Enum",
        14 => "Keyword",
        15 => "Snippet",
        16 => "Color",
        17 => "File",
        18 => "Reference",
        19 => "Folder",
        20 => "EnumMember",
        21 => "Constant",
        22 => "Struct",
        23 => "Event",
        24 => "Operator",
        25 => "TypeParameter",
        _ => "Unknown",
    }
    .to_string()
}

/// Render the LSP autocomplete popup (replaces the basic word-based one when LSP items exist).
pub fn render_lsp_autocomplete(
    ui: &mut egui::Ui,
    items: &[LspCompletionItem],
    selected: usize,
    cursor_screen_pos: egui::Pos2,
    line_height: f32,
) {
    if items.is_empty() {
        return;
    }

    let item_height = 22.0;
    let max_visible = 10.min(items.len());
    let popup_width = 320.0;
    let popup_height = max_visible as f32 * item_height;

    let popup_pos = egui::Pos2::new(cursor_screen_pos.x, cursor_screen_pos.y + line_height);
    let popup_rect =
        egui::Rect::from_min_size(popup_pos, egui::Vec2::new(popup_width, popup_height));

    // Shadow
    ui.painter().rect_filled(
        popup_rect.translate(egui::Vec2::new(2.0, 2.0)),
        4.0,
        egui::Color32::from_rgba_premultiplied(0, 0, 0, 60),
    );

    // Background
    ui.painter()
        .rect_filled(popup_rect, 4.0, egui::Color32::from_rgb(37, 37, 38));
    ui.painter().rect_stroke(
        popup_rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(69, 69, 69)),
    );

    let font_id = egui::FontId::monospace(12.0);
    let small_font = egui::FontId::monospace(10.0);

    let scroll_offset = if selected >= max_visible {
        selected - max_visible + 1
    } else {
        0
    };

    for (vi, i) in (scroll_offset..items.len().min(scroll_offset + max_visible)).enumerate() {
        let item = &items[i];
        let y = popup_pos.y + vi as f32 * item_height;
        let item_rect = egui::Rect::from_min_size(
            egui::Pos2::new(popup_pos.x, y),
            egui::Vec2::new(popup_width, item_height),
        );

        if i == selected {
            ui.painter()
                .rect_filled(item_rect, 0.0, egui::Color32::from_rgb(4, 57, 94));
        }

        // Kind icon
        let kind_char = match item.kind.as_deref() {
            Some("Function") | Some("Method") => "fn",
            Some("Variable") | Some("Field") => "var",
            Some("Class") | Some("Struct") => "cls",
            Some("Interface") => "ifc",
            Some("Module") => "mod",
            Some("Keyword") => "key",
            Some("Snippet") => "<>",
            Some("Constant") | Some("EnumMember") => "cst",
            Some("Enum") => "enm",
            Some("Property") => "prp",
            _ => " - ",
        };
        let kind_color = match item.kind.as_deref() {
            Some("Function") | Some("Method") => egui::Color32::from_rgb(220, 220, 170),
            Some("Variable") | Some("Field") => egui::Color32::from_rgb(156, 220, 254),
            Some("Class") | Some("Struct") => egui::Color32::from_rgb(78, 201, 176),
            Some("Keyword") => egui::Color32::from_rgb(197, 134, 192),
            _ => egui::Color32::from_rgb(188, 188, 188),
        };

        ui.painter().text(
            egui::Pos2::new(popup_pos.x + 4.0, y + 2.0),
            egui::Align2::LEFT_TOP,
            kind_char,
            font_id.clone(),
            kind_color,
        );

        // Label
        ui.painter().text(
            egui::Pos2::new(popup_pos.x + 22.0, y + 2.0),
            egui::Align2::LEFT_TOP,
            &item.label,
            font_id.clone(),
            if i == selected {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(188, 188, 188)
            },
        );

        // Detail (right-aligned, dimmer)
        if let Some(ref detail) = item.detail {
            let truncated: String = detail.chars().take(30).collect();
            ui.painter().text(
                egui::Pos2::new(popup_pos.x + popup_width - 8.0, y + 4.0),
                egui::Align2::RIGHT_TOP,
                &truncated,
                small_font.clone(),
                egui::Color32::from_rgb(120, 120, 120),
            );
        }
    }
}

/// Render diagnostic squiggles for a line.
pub fn render_diagnostic_squiggles(
    ui: &mut egui::Ui,
    diagnostics: &[LspDiagnostic],
    line_idx: usize,
    text_left: f32,
    y: f32,
    line_height: f32,
    char_width: f32,
    col_offset: usize,
) {
    for diag in diagnostics {
        if diag.line != line_idx {
            continue;
        }

        let start_col = diag.col.saturating_sub(col_offset);
        let end_col = if diag.end_line == line_idx {
            diag.end_col.saturating_sub(col_offset).max(start_col + 1)
        } else {
            start_col + 10 // extend to some reasonable width
        };

        let color = match diag.severity {
            DiagnosticSeverityLevel::Error => egui::Color32::from_rgb(255, 80, 80),
            DiagnosticSeverityLevel::Warning => egui::Color32::from_rgb(255, 200, 50),
            DiagnosticSeverityLevel::Info => egui::Color32::from_rgb(80, 180, 255),
            DiagnosticSeverityLevel::Hint => egui::Color32::from_rgb(150, 150, 150),
        };

        let x_start = text_left + 4.0 + start_col as f32 * char_width;
        let x_end = text_left + 4.0 + end_col as f32 * char_width;
        let squiggle_y = y + line_height - 3.0;

        // Draw squiggly line
        let mut points = Vec::new();
        let mut x = x_start;
        let mut up = true;
        while x < x_end {
            let sy = if up {
                squiggle_y - 1.5
            } else {
                squiggle_y + 1.5
            };
            points.push(egui::Pos2::new(x, sy));
            x += 3.0;
            up = !up;
        }

        if points.len() >= 2 {
            for pair in points.windows(2) {
                ui.painter()
                    .line_segment([pair[0], pair[1]], egui::Stroke::new(1.2, color));
            }
        }
    }
}

/// Render hover tooltip.
pub fn render_hover_tooltip(ui: &mut egui::Ui, text: &str, pos: egui::Pos2) {
    if text.is_empty() {
        return;
    }

    let font_id = egui::FontId::monospace(12.0);
    let max_width: f32 = 400.0;

    // Truncate for display
    let display_text: String = text.chars().take(500).collect();
    let lines: Vec<&str> = display_text.lines().take(15).collect();
    let display = lines.join("\n");

    let line_count = display.lines().count().max(1);
    let popup_height = line_count as f32 * 16.0 + 12.0;
    let popup_width =
        max_width.min(display.lines().map(|l| l.len()).max().unwrap_or(10) as f32 * 7.5 + 16.0);

    let popup_rect = egui::Rect::from_min_size(
        egui::Pos2::new(pos.x, pos.y - popup_height - 4.0),
        egui::Vec2::new(popup_width, popup_height),
    );

    // Background
    ui.painter()
        .rect_filled(popup_rect, 4.0, egui::Color32::from_rgb(45, 45, 48));
    ui.painter().rect_stroke(
        popup_rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
    );

    ui.painter().text(
        egui::Pos2::new(popup_rect.left() + 8.0, popup_rect.top() + 6.0),
        egui::Align2::LEFT_TOP,
        &display,
        font_id,
        egui::Color32::from_rgb(212, 212, 212),
    );
}

/// Render the references results panel (similar to Find in Files).
///
/// Returns `Some((file_path, line))` when the user clicks on a reference.
pub fn render_references_panel(
    ui: &mut egui::Ui,
    state: &mut ReferencesState,
) -> Option<(std::path::PathBuf, usize)> {
    let mut navigate_to: Option<(std::path::PathBuf, usize)> = None;

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(37, 37, 38))
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            // Header row
            ui.horizontal(|ui| {
                ui.strong(format!("References ({})", state.locations.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("\u{00D7}").on_hover_text("Close (Esc)").clicked() {
                        state.visible = false;
                    }
                });
            });

            ui.separator();

            if state.locations.is_empty() {
                ui.label("No references found.");
            }

            // Group locations by file
            let mut by_file: Vec<(String, Vec<&LspLocation>)> = Vec::new();
            for loc in &state.locations {
                if let Some(entry) = by_file.iter_mut().find(|(uri, _)| uri == &loc.uri) {
                    entry.1.push(loc);
                } else {
                    by_file.push((loc.uri.clone(), vec![loc]));
                }
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (uri, locs) in &by_file {
                        // Extract a nice display path from the URI
                        let display_path = Url::parse(uri)
                            .ok()
                            .and_then(|u| u.to_file_path().ok())
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| uri.clone());

                        // File header
                        ui.label(
                            egui::RichText::new(format!("{} ({})", display_path, locs.len()))
                                .strong()
                                .color(egui::Color32::from_rgb(200, 200, 200)),
                        );

                        // Individual references
                        for loc in locs {
                            let resp = ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                let label = format!("Line {}, Col {}", loc.line + 1, loc.col + 1);
                                let resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&label)
                                            .monospace()
                                            .color(egui::Color32::from_rgb(180, 180, 180)),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                resp
                            });

                            if resp.inner.clicked() {
                                if let Ok(url) = Url::parse(uri) {
                                    if let Ok(path) = url.to_file_path() {
                                        navigate_to = Some((path, loc.line));
                                    }
                                }
                            }
                        }

                        ui.add_space(2.0);
                    }
                });
        });

    navigate_to
}

/// Render the inline rename dialog as an egui Window.
/// Returns `Some(new_name)` when the user confirms the rename (presses Enter).
pub fn render_rename_dialog(ctx: &egui::Context, state: &mut RenameDialogState) -> Option<String> {
    if !state.visible {
        return None;
    }

    let mut result: Option<String> = None;
    let mut open = state.visible;

    egui::Window::new("Rename Symbol")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .title_bar(true)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 120.0])
        .fixed_size([300.0, 50.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("New name:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.input)
                        .desired_width(200.0)
                        .hint_text("Enter new name..."),
                );

                if state.needs_focus {
                    response.request_focus();
                    // Select all text in the input
                    state.needs_focus = false;
                }

                // Enter to confirm
                if response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !state.input.is_empty()
                {
                    result = Some(state.input.clone());
                    state.visible = false;
                }
            });
        });

    state.visible = open;

    // Escape pressed while dialog is open
    if state.visible && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.visible = false;
    }

    result
}
