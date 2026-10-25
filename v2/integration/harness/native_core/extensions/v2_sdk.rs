//! Shared compiled-in Native V2 SDK. Storage never touches legacy extension storage.
use crate::native_core::common::{write_json_atomic, CoreContext, CoreResult};
use serde_json::{json, Value};
use std::sync::Mutex;

// One blocking transaction lane prevents lost updates across SDK callers/contexts.
static STORE_LOCK: Mutex<()> = Mutex::new(());

pub fn view(title: &str, root: Value, actions: Vec<Value>) -> Value {
    json!({"ok":true,"mode":"view","title":title,"root":root,"actions":actions})
}
pub fn no_view(message: &str) -> Value {
    json!({"ok":true,"mode":"no-view","message":message})
}
pub fn unchanged() -> Value {
    json!({"ok":true,"mode":"unchanged"})
}
pub async fn open(ctx: &CoreContext, target: &str) -> CoreResult {
    ctx.request("shell:open", vec![json!(target)]).await
}
pub async fn copy(ctx: &CoreContext, text: &str) -> CoreResult {
    ctx.request("clipboard:write", vec![json!(text)]).await
}
fn store_path(ctx: &CoreContext, slug: &str) -> Result<std::path::PathBuf, String> {
    if slug.is_empty()
        || !slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid native extension storage slug".into());
    }
    Ok(ctx
        .paths
        .data_dir
        .join("native-extensions-v2")
        .join(slug)
        .join("storage.json"))
}
fn read(path: &std::path::Path) -> CoreResult {
    match std::fs::read(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
        Ok(bytes) => {
            let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if !value.is_object() {
                return Err("Native extension storage must be an object".into());
            }
            Ok(value)
        }
    }
}
pub async fn read_store(ctx: &CoreContext, slug: &str) -> CoreResult {
    let path = store_path(ctx, slug)?;
    tokio::task::spawn_blocking(move || {
        let _guard = STORE_LOCK.lock().map_err(|e| e.to_string())?;
        read(&path)
    })
    .await
    .map_err(|e| e.to_string())?
}
pub async fn update_store<F>(ctx: &CoreContext, slug: &str, change: F) -> CoreResult
where
    F: FnOnce(&mut Value) -> Result<(), String> + Send + 'static,
{
    let path = store_path(ctx, slug)?;
    tokio::task::spawn_blocking(move || {
        let _guard = STORE_LOCK.lock().map_err(|e| e.to_string())?;
        let mut value = read(&path)?;
        change(&mut value)?;
        if !value.is_object() {
            return Err("Native extension storage must remain an object".into());
        }
        write_json_atomic(&path, &value)?;
        Ok(value)
    })
    .await
    .map_err(|e| e.to_string())?
}
fn encode(text: &str, form: bool) -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"*-._".contains(&b) || (!form && b"!~'()".contains(&b)) {
            out.push(b as char);
        } else if form && b == b' ' {
            out.push('+');
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 15) as usize] as char);
        }
    }
    out
}
pub fn encode_form_query(text: &str) -> String {
    encode(text, true)
}
pub fn encode_component(text: &str) -> String {
    encode(text, false)
}
