//! Production sibling of v2_sdk; vendored core is compiled into the host binary.
use super::v2_sdk as sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tezbar_markdown_styler::{
    self as core, document,
    temp::{self, Artifact},
    Converted,
};

pub fn commands() -> &'static [&'static str] {
    &["mp-style-markdown"]
}
fn capability() -> Result<String, String> {
    core::capability()
}
#[derive(Serialize, Deserialize)]
struct Session {
    ids: BTreeMap<String, String>,
    display: String,
    message: String,
    converted: Option<Converted>,
    normal: Option<Artifact>,
    sample: Option<Artifact>,
    owned: Vec<Artifact>,
    disposed: bool,
}
impl Session {
    fn new() -> Self {
        Self {
            ids: BTreeMap::new(),
            display: "Reading selected text…".into(),
            message: String::new(),
            converted: None,
            normal: None,
            sample: None,
            owned: Vec::new(),
            disposed: false,
        }
    }
    fn rotate(&mut self) -> Result<(), String> {
        let stamp = capability()?;
        self.ids = ["open", "refresh", "copy", "sample"]
            .map(|verb| (verb.into(), format!("markdown-styler:{stamp}:{verb}")))
            .into_iter()
            .collect();
        Ok(())
    }
    fn budget(&self, bytes: usize) -> Result<(), String> {
        if self.owned.len() >= core::MAX_FILES
            || self
                .owned
                .iter()
                .map(|a| a.bytes)
                .sum::<usize>()
                .saturating_add(bytes)
                > core::MAX_TOTAL_BYTES
        {
            Err("Temporary document resource limit reached; close this session before converting again".into())
        } else {
            Ok(())
        }
    }
    fn action(&self, verb: &str, title: &str, key: &str, modifiers: &[&str]) -> Value {
        json!({"id":self.ids[verb],"title":title,"kind":"action","shortcut":{"modifiers":modifiers,"key":key}})
    }
    fn view(&self) -> Value {
        let mut actions = Vec::new();
        if self.normal.is_some() {
            actions.push(self.action("open", "Open in Browser and Copy", "o", &["cmd"]));
            actions.push(self.action("refresh", "Refresh Selection", "r", &["cmd"]));
            actions.push(self.action("copy", "Copy Original Markdown", "c", &["cmd", "shift"]));
            actions.push(self.action("sample", "Try Sample Conversion", "t", &["cmd"]));
        } else {
            actions.push(self.action("sample", "Try Sample Conversion", "t", &["cmd"]));
            actions.push(self.action("refresh", "Refresh Selection", "r", &["cmd"]));
        }
        let mut v = sdk::view(
            "Style Markdown",
            json!({"type":"Detail","props":{"markdown":core::safe_detail(&self.display)},"children":[]}),
            actions,
        );
        if !self.message.is_empty() {
            v["message"] = json!(self.message)
        }
        v
    }
}
fn save(state: &mut Value, s: &Session) -> Result<(), String> {
    *state = serde_json::to_value(s).map_err(|_| "Unable to checkpoint Markdown Styler session")?;
    Ok(())
}
fn ack(result: CoreResult) -> bool {
    match result {
        Ok(v) => {
            v.is_object()
                && v["ok"] == true
                && v.get("error").is_none_or(Value::is_null)
                && v.get("success").is_none_or(|x| x == true)
        }
        Err(_) => false,
    }
}
fn selection_error() -> String {
    "❌ Unable to get selected text\n\nError: Selection or conversion unavailable\n\nPlease select some text in another application and try again.".into()
}
async fn selection(ctx: &CoreContext, state: &mut Value, s: &mut Session) -> Result<(), String> {
    s.converted = None;
    s.normal = None;
    s.message.clear();
    s.display = "Reading selected text…".into();
    save(state, s)?;
    let selected = ctx.request("core:platform:selected-text", vec![]).await;
    let selected = match selected {
        Ok(Value::String(s)) => s,
        _ => {
            s.display = selection_error();
            save(state, s)?;
            return Ok(());
        }
    };
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // Synchronous bounded conversion and file transaction have no cancellation
    // point: RAII owns output until committed to canonical host session state.
    match core::convert(&selected) {
        Err(e) => {
            s.display = format!("❌ Unable to convert selected text\n\n{e}");
        }
        Ok(c) => {
            if !c.styled_html.is_empty() {
                let result = (|| {
                    let doc = document::preview(&c.styled_html, false).map_err(str::to_string)?;
                    s.budget(doc.len())?;
                    let pending =
                        temp::create(&ctx.paths.temp_dir, &doc, false).map_err(str::to_string)?;
                    Ok::<_, String>(pending)
                })();
                match result {
                    Ok(pending) => {
                        let artifact = pending.commit();
                        s.owned.push(artifact.clone());
                        s.normal = Some(artifact);
                        save(state, s)?;
                    }
                    Err(e) => {
                        s.display = format!("❌ Unable to convert selected text\n\n{e}");
                        save(state, s)?;
                        return Ok(());
                    }
                }
            }
            let time = chrono::Local::now().format("%X").to_string();
            let execution = capability()?;
            s.display = c.display(&time, &execution);
            s.converted = Some(c);
        }
    }
    save(state, s)
}
pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    if !commands().contains(&command) {
        return Err("Unknown Markdown Styler command".into());
    }
    // Heartbeats/search/load-more never initialize, mint IDs or cross any effect
    // boundary. NativeSessions may discard search state clones.
    if matches!(
        operation,
        "extension:refresh-session" | "extension:search-text-changed" | "extension:load-more"
    ) {
        return match serde_json::from_value::<Session>(state.clone()) {
            Ok(s) if !s.disposed => Ok(s.view()),
            _ => Ok(sdk::unchanged()),
        };
    }
    if operation == "extension:dispose-session" {
        if let Ok(mut s) = serde_json::from_value::<Session>(state.clone()) {
            s.disposed = true;
            s.ids.clear();
            let mut failed = false;
            s.owned.retain(|a| {
                let keep = temp::cleanup(a).is_err();
                failed |= keep;
                keep
            });
            s.converted = None;
            s.normal = None;
            s.sample = None;
            s.display.clear();
            s.message.clear();
            save(state, &s)?;
            if failed {
                return Err("Some temporary documents could not be removed safely".into());
            }
        }
        *state = json!({"disposed":true});
        return Ok(sdk::unchanged());
    }
    if operation == "extension:run-command" {
        if let Ok(s) = serde_json::from_value::<Session>(state.clone()) {
            if s.disposed {
                return Err("Markdown Styler session is closed".into());
            }
            return Ok(s.view());
        }
        if state["disposed"] == true {
            return Err("Markdown Styler session is closed".into());
        }
        let mut s = Session::new();
        s.rotate()?;
        save(state, &s)?;
        selection(ctx, state, &mut s).await?;
        return Ok(s.view());
    }
    if operation != "extension:invoke-action" {
        return Err("Unsupported Markdown Styler operation".into());
    }
    let mut s: Session = serde_json::from_value(state.clone())
        .map_err(|_| "Markdown Styler session is unavailable")?;
    if s.disposed {
        return Err("Markdown Styler session is closed".into());
    }
    let id = payload["actionId"]
        .as_str()
        .ok_or("Invalid Markdown Styler action")?;
    let verb = s
        .ids
        .iter()
        .find_map(|(verb, token)| (token == id).then(|| verb.clone()))
        .ok_or("Stale or invalid Markdown Styler action; use the current actions")?;
    if matches!(verb.as_str(), "open" | "copy") && s.normal.is_none() {
        return Err("Action is not available in this state".into());
    }
    // Rotate and checkpoint intent before awaits. Cancellation cannot cause old
    // action replay or manufacture success after an unconfirmed OS operation.
    s.rotate()?;
    s.message = "Action result unconfirmed; no automatic retry was performed".into();
    save(state, &s)?;
    match verb.as_str() {
        "refresh" => {
            selection(ctx, state, &mut s).await?;
        }
        "open" => {
            let url = temp::file_url(s.normal.as_ref().ok_or("No converted document")?);
            let confirmed = match url {
                Ok(url) => ack(sdk::open(ctx, &url).await),
                Err(_) => false,
            };
            s.message = if confirmed {
                "Opened in Browser — Please follow the instructions to copy content manually"
            } else {
                "Failed to Open — Unable to open preview in browser; result unconfirmed"
            }
            .into();
        }
        "copy" => {
            let markdown = &s
                .converted
                .as_ref()
                .ok_or("No converted Markdown")?
                .markdown;
            s.message = if ack(sdk::copy(ctx, markdown).await) {
                "Markdown Copied — Original Markdown text copied to clipboard"
            } else {
                "Copy Failed — Please try again; result unconfirmed"
            }
            .into();
        }
        "sample" => {
            // Reuse acknowledged owned sample file after a failed/uncertain open.
            // Explicit Refresh affects normal selection only, never this file.
            let artifact: Result<Artifact, String> = if let Some(a) = s.sample.clone() {
                Ok(a)
            } else {
                (|| {
                    let styled = core::render::styled(document::SAMPLE).map_err(str::to_string)?;
                    let doc = document::preview(&styled, true).map_err(str::to_string)?;
                    s.budget(doc.len())?;
                    let pending =
                        temp::create(&ctx.paths.temp_dir, &doc, true).map_err(str::to_string)?;
                    Ok(pending.commit())
                })()
            };
            match artifact {
                Err(_) => s.message = "Sample Failed — Unable to generate sample".into(),
                Ok(a) => {
                    if s.sample.is_none() {
                        s.owned.push(a.clone());
                        s.sample = Some(a.clone());
                    }
                    save(state, &s)?; // file is canonical even if browser fails/cancels
                    let confirmed = match temp::file_url(&a) {
                        Ok(url) => ack(sdk::open(ctx, &url).await),
                        Err(_) => false,
                    };
                    s.message = if confirmed {
                        "Sample Ready — Demo conversion completed - check browser"
                    } else {
                        "Sample Failed — Unable to open sample in browser; result unconfirmed"
                    }
                    .into();
                }
            }
        }
        _ => return Err("Unsupported Markdown Styler action".into()),
    }
    save(state, &s)?;
    Ok(s.view())
}
