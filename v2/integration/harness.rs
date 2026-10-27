// The real beta common.rs uses only this Tauri emitter API in headless tests.
extern crate self as tauri;
#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit(&self, channel: &str, payload: serde_json::Value) -> Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit(&self, _: &str, _: serde_json::Value) -> Result<(), String> {
        Ok(())
    }
}
#[path = "harness/native_core/mod.rs"]
pub mod native_core;
