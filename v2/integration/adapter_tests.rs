use crate::native_core::{
    common::{CoreContext, CorePaths},
    extensions::{native::NativeSessions, v2_catalog, v2_markdown_styler as adapter},
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
type Calls = Arc<Mutex<Vec<(String, Vec<Value>)>>>;
const SELECTED_TEXT_CHANNEL: &str = "core:platform:selected-text";
struct Test {
    dir: tempfile::TempDir,
    ctx: CoreContext,
    calls: Calls,
    reply: Arc<Mutex<Value>>,
}
impl Test {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let temp = p.join("tmp #文 %");
        std::fs::create_dir(&temp).unwrap();
        let ctx = CoreContext::headless(CorePaths {
            data_dir: p.join("data"),
            config_dir: p.join("config"),
            resource_dir: p.join("resources"),
            temp_dir: temp,
            home_dir: p.join("home"),
        });
        let calls: Calls = Arc::default();
        let seen = calls.clone();
        let reply = Arc::new(Mutex::new(json!({"ok":true})));
        let replies = reply.clone();
        ctx.set_router(Arc::new(move |ch, args| {
            seen.lock().unwrap().push((ch.clone(), args.clone()));
            let r = if ch == SELECTED_TEXT_CHANNEL {
                json!("  # Selected **Markdown** text  ")
            } else {
                replies.lock().unwrap().clone()
            };
            Box::pin(async move { Ok(r) })
        }))
        .unwrap();
        Self {
            dir,
            ctx,
            calls,
            reply,
        }
    }
    async fn handle(&self, op: &str, payload: Value, state: &mut Value) -> Value {
        adapter::handle(&self.ctx, "mp-style-markdown", op, &payload, state)
            .await
            .unwrap()
    }
    fn count(&self, ch: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(c, _)| c == ch)
            .count()
    }
}
fn action(v: &Value, title: &str) -> Value {
    json!({"actionId":v["actions"].as_array().unwrap().iter().find(|a|a["title"]==title).unwrap()["id"]})
}
#[tokio::test]
async fn direct_lifecycle_ack_heartbeat_retry_and_cleanup() {
    let t = Test::new();
    let mut state = json!({});
    let v = t
        .handle(
            "extension:run-command",
            json!({"argumentValues":{},"preferences":{}}),
            &mut state,
        )
        .await;
    assert_eq!(t.count(SELECTED_TEXT_CHANNEL), 1);
    assert_eq!(t.count("shell:open"), 0);
    assert_eq!(t.count("clipboard:write"), 0);
    let canonical = state.clone();
    for op in [
        "extension:refresh-session",
        "extension:search-text-changed",
        "extension:load-more",
    ] {
        let h = t
            .handle(op, json!({"searchText":"malicious"}), &mut state)
            .await;
        assert_eq!(h, v);
        assert_eq!(state, canonical)
    }
    let stale = action(&v, "Open in Browser and Copy");
    let opened = t
        .handle("extension:invoke-action", stale.clone(), &mut state)
        .await;
    assert!(opened["message"]
        .as_str()
        .unwrap()
        .starts_with("Opened in Browser"));
    assert!(adapter::handle(
        &t.ctx,
        "mp-style-markdown",
        "extension:invoke-action",
        &stale,
        &mut state
    )
    .await
    .is_err());
    let calls = t.calls.lock().unwrap().clone();
    let target = calls.iter().find(|(ch, _)| ch == "shell:open").unwrap();
    assert_eq!(target.1.len(), 1);
    let u = target.1[0].as_str().unwrap();
    assert!(u.starts_with("file:///"));
    assert!(u.contains("%20%23%E6%96%87%20%25"));
    assert_eq!(state["owned"].as_array().unwrap().len(), 1);
    let copied = t
        .handle(
            "extension:invoke-action",
            action(&opened, "Copy Original Markdown"),
            &mut state,
        )
        .await;
    assert!(copied["message"]
        .as_str()
        .unwrap()
        .starts_with("Markdown Copied"));
    assert_eq!(
        t.calls
            .lock()
            .unwrap()
            .iter()
            .find(|(c, _)| c == "clipboard:write")
            .unwrap()
            .1,
        vec![json!("# Selected **Markdown** text")]
    );
    let before = state["normal"].clone();
    let sample = t
        .handle(
            "extension:invoke-action",
            action(&copied, "Try Sample Conversion"),
            &mut state,
        )
        .await;
    assert_eq!(state["normal"], before);
    assert_eq!(t.count(SELECTED_TEXT_CHANNEL), 1);
    assert_eq!(state["owned"].as_array().unwrap().len(), 2);
    *t.reply.lock().unwrap() = Value::Null;
    let failed = t
        .handle(
            "extension:invoke-action",
            action(&sample, "Try Sample Conversion"),
            &mut state,
        )
        .await;
    assert!(failed["message"]
        .as_str()
        .unwrap()
        .starts_with("Sample Failed"));
    assert_eq!(state["owned"].as_array().unwrap().len(), 2);
    let docs = state["owned"].as_array().unwrap().clone();
    t.handle("extension:dispose-session", json!({}), &mut state)
        .await;
    for a in docs {
        assert!(!std::path::Path::new(a["directory"].as_str().unwrap()).exists())
    }
    assert!(adapter::handle(
        &t.ctx,
        "mp-style-markdown",
        "extension:invoke-action",
        &action(&failed, "Refresh Selection"),
        &mut state
    )
    .await
    .is_err());
    assert!(t.dir.path().exists());
}
#[tokio::test]
async fn all_nonpositive_ack_shapes_are_unconfirmed() {
    for reply in [
        Value::Null,
        json!(true),
        json!(false),
        json!({}),
        json!({"ok":false}),
        json!({"ok":true,"success":false}),
        json!({"ok":true,"error":"bad"}),
        json!({"ok":true,"success":null}),
        json!({"ok":"true"}),
        json!({"ok":true,"error":false}),
    ] {
        let t = Test::new();
        *t.reply.lock().unwrap() = reply;
        let mut s = json!({});
        let v = t.handle("extension:run-command", json!({}), &mut s).await;
        let v = t
            .handle(
                "extension:invoke-action",
                action(&v, "Copy Original Markdown"),
                &mut s,
            )
            .await;
        assert!(v["message"].as_str().unwrap().starts_with("Copy Failed"));
        t.handle("extension:dispose-session", json!({}), &mut s)
            .await;
    }
}
#[tokio::test]
async fn real_sessions_registry_catalog_alias_v1_coexistence_and_disposal() {
    let t = Test::new();
    let sessions = NativeSessions::default();
    let files = Arc::new(Mutex::new(()));
    let native = "com.tezbar.native.markdown-styler";
    let request =
        json!({"extensionId":"raycast.markdown-styler","commandName":"mp-style-markdown"});
    assert!(sessions
        .handle(
            &t.ctx,
            &files,
            "extension:run-command",
            std::slice::from_ref(&request)
        )
        .await
        .is_none());
    let legacy = t
        .ctx
        .paths
        .data_dir
        .join("extensions/packages/markdown-styler");
    std::fs::create_dir_all(&legacy).unwrap();
    let bytes=b"{\"name\":\"markdown-styler\",\"commands\":[{\"name\":\"mp-style-markdown\",\"platforms\":[\"macOS\",\"Windows\"]}]}";
    std::fs::write(legacy.join("package.json"), bytes).unwrap();
    v2_catalog::cli_install(&t.ctx, native).unwrap();
    assert!(v2_catalog::cli_list(&t.ctx).unwrap().is_array());
    let v = sessions
        .handle(
            &t.ctx,
            &files,
            "extension:run-command",
            std::slice::from_ref(&request),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(v["mode"], "view");
    let id = v["sessionId"].clone();
    let h = sessions
        .handle(
            &t.ctx,
            &files,
            "extension:refresh-session",
            &[json!({"sessionId":id})],
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(h["actions"], v["actions"]);
    assert_eq!(t.count(SELECTED_TEXT_CHANNEL), 1);
    let other = sessions
        .handle(
            &t.ctx,
            &files,
            "extension:run-command",
            &[json!({"extensionId":native,"commandName":"mp-style-markdown"})],
        )
        .await
        .unwrap()
        .unwrap();
    let mut forged = action(&v, "Open in Browser and Copy");
    forged["sessionId"] = other["sessionId"].clone();
    assert_eq!(
        sessions
            .handle(&t.ctx, &files, "extension:invoke-action", &[forged])
            .await
            .unwrap()
            .unwrap()["ok"],
        false
    );
    assert_eq!(t.count("shell:open"), 0);
    assert_eq!(std::fs::read(legacy.join("package.json")).unwrap(), bytes);
    sessions.clear().await;
    assert_eq!(std::fs::read_dir(&t.ctx.paths.temp_dir).unwrap().count(), 0);
    let stale = sessions
        .handle(
            &t.ctx,
            &files,
            "extension:refresh-session",
            &[json!({"sessionId":id})],
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stale["ok"], false);
}

#[tokio::test]
async fn real_session_cancel_launch_refresh_and_open_owns_cleanup() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::Notify;
    for phase in ["launch", "refresh", "open"] {
        let root = tempfile::tempdir().unwrap();
        let p = root.path();
        let tmp = p.join("tmp");
        std::fs::create_dir(&tmp).unwrap();
        let ctx = CoreContext::headless(CorePaths {
            data_dir: p.join("data"),
            config_dir: p.join("config"),
            resource_dir: p.join("resources"),
            temp_dir: tmp.clone(),
            home_dir: p.join("home"),
        });
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let arm = Arc::new(AtomicBool::new(phase == "launch"));
        let calls: Calls = Arc::default();
        let e = entered.clone();
        let r = release.clone();
        let a = arm.clone();
        let c = calls.clone();
        let gate_channel = if phase == "open" {
            "shell:open"
        } else {
            SELECTED_TEXT_CHANNEL
        };
        ctx.set_router(Arc::new(move |ch, args| {
            c.lock().unwrap().push((ch.clone(), args));
            let wait = ch == gate_channel && a.swap(false, Ordering::SeqCst);
            let e = e.clone();
            let r = r.clone();
            Box::pin(async move {
                if wait {
                    e.notify_one();
                    r.notified().await;
                }
                Ok(if ch == SELECTED_TEXT_CHANNEL {
                    json!("# Cancellable selected Markdown")
                } else {
                    json!({"ok":true})
                })
            })
        }))
        .unwrap();
        v2_catalog::cli_install(&ctx, "com.tezbar.native.markdown-styler").unwrap();
        let sessions = NativeSessions::default();
        let files = Arc::new(Mutex::new(()));
        let request = json!({"extensionId":"com.tezbar.native.markdown-styler","commandName":"mp-style-markdown"});
        let (op, body) = if phase == "launch" {
            ("extension:run-command", request)
        } else {
            let v = sessions
                .handle(&ctx, &files, "extension:run-command", &[request])
                .await
                .unwrap()
                .unwrap();
            arm.store(true, Ordering::SeqCst);
            let mut b = action(
                &v,
                if phase == "open" {
                    "Open in Browser and Copy"
                } else {
                    "Refresh Selection"
                },
            );
            b["sessionId"] = v["sessionId"].clone();
            ("extension:invoke-action", b)
        };
        let s = sessions.clone();
        let taskctx = ctx.clone();
        let f = files.clone();
        let task = tokio::spawn(async move { s.handle(&taskctx, &f, op, &[body]).await });
        tokio::time::timeout(std::time::Duration::from_secs(2), entered.notified())
            .await
            .unwrap();
        sessions.clear().await;
        let result = task.await.unwrap().unwrap().unwrap();
        assert_eq!(result["ok"], false);
        assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0);
        assert!(calls
            .lock()
            .unwrap()
            .iter()
            .all(|(c, _)| c != "clipboard:write"));
    }
}
#[tokio::test]
async fn dropped_effect_future_checkpoints_intent_and_sample_file_without_retry() {
    use tokio::sync::Notify;
    let root = tempfile::tempdir().unwrap();
    let p = root.path();
    let tmp = p.join("tmp");
    std::fs::create_dir(&tmp).unwrap();
    let ctx = CoreContext::headless(CorePaths {
        data_dir: p.join("data"),
        config_dir: p.join("config"),
        resource_dir: p.join("resources"),
        temp_dir: tmp.clone(),
        home_dir: p.join("home"),
    });
    let calls: Calls = Arc::default();
    let c = calls.clone();
    let block = Arc::new(Notify::new());
    ctx.set_router(Arc::new(move |ch, args| {
        c.lock().unwrap().push((ch.clone(), args));
        let b = block.clone();
        Box::pin(async move {
            if ch == "shell:open" {
                b.notified().await;
            }
            Ok(if ch == SELECTED_TEXT_CHANNEL {
                json!("# Some selected Markdown")
            } else {
                json!({"ok":true})
            })
        })
    }))
    .unwrap();
    let mut state = json!({});
    let initial = adapter::handle(
        &ctx,
        "mp-style-markdown",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    let old = action(&initial, "Try Sample Conversion");
    assert!(tokio::time::timeout(
        std::time::Duration::from_millis(30),
        adapter::handle(
            &ctx,
            "mp-style-markdown",
            "extension:invoke-action",
            &old,
            &mut state
        )
    )
    .await
    .is_err());
    assert_eq!(state["owned"].as_array().unwrap().len(), 2);
    assert!(!state["sample"].is_null());
    assert!(state["message"].as_str().unwrap().contains("unconfirmed"));
    let checkpoint = state.clone();
    for op in [
        "extension:refresh-session",
        "extension:search-text-changed",
        "extension:load-more",
    ] {
        adapter::handle(&ctx, "mp-style-markdown", op, &json!({}), &mut state)
            .await
            .unwrap();
        assert_eq!(state, checkpoint)
    }
    assert!(adapter::handle(
        &ctx,
        "mp-style-markdown",
        "extension:invoke-action",
        &old,
        &mut state
    )
    .await
    .is_err());
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(ch, _)| ch == "shell:open")
            .count(),
        1
    );
    adapter::handle(
        &ctx,
        "mp-style-markdown",
        "extension:dispose-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read_dir(tmp).unwrap().count(), 0);
}
#[tokio::test]
async fn error_states_refresh_bound_resources_and_private_text_safe_detail() {
    let t = Test::new();
    let mut state = json!({});
    let mut v = t
        .handle("extension:run-command", json!({}), &mut state)
        .await;
    for _ in 1..tezbar_markdown_styler::MAX_FILES {
        v = t
            .handle(
                "extension:invoke-action",
                action(&v, "Refresh Selection"),
                &mut state,
            )
            .await;
    }
    assert_eq!(
        state["owned"].as_array().unwrap().len(),
        tezbar_markdown_styler::MAX_FILES
    );
    v = t
        .handle(
            "extension:invoke-action",
            action(&v, "Refresh Selection"),
            &mut state,
        )
        .await;
    assert!(v["root"]["props"]["markdown"]
        .as_str()
        .unwrap()
        .contains("&#"));
    assert!(state["display"]
        .as_str()
        .unwrap()
        .contains("resource limit"));
    assert_eq!(v["actions"].as_array().unwrap().len(), 2);
    t.handle("extension:dispose-session", json!({}), &mut state)
        .await;
    for selection in [
        json!("  "),
        json!("123456789"),
        Value::Null,
        json!(true),
        json!({"error":"private <script>"}),
    ] {
        let root = tempfile::tempdir().unwrap();
        let p = root.path();
        let ctx = CoreContext::headless(CorePaths {
            data_dir: p.join("data"),
            config_dir: p.join("config"),
            resource_dir: p.join("resource"),
            temp_dir: p.to_owned(),
            home_dir: p.join("home"),
        });
        ctx.set_router(Arc::new(move |ch, args| {
            assert_eq!(ch, SELECTED_TEXT_CHANNEL);
            assert!(args.is_empty());
            let v = selection.clone();
            Box::pin(async move { Ok(v) })
        }))
        .unwrap();
        let mut state = json!({});
        let v = adapter::handle(
            &ctx,
            "mp-style-markdown",
            "extension:run-command",
            &json!({"argumentValues":{"source":"ignored"},"preferences":{"source":"ignored"}}),
            &mut state,
        )
        .await
        .unwrap();
        assert_eq!(v["actions"].as_array().unwrap().len(), 2);
        assert_eq!(state["owned"], json!([]));
        assert!(!v.to_string().contains("private"));
    }
}

#[tokio::test]
async fn transport_write_errors_and_positive_ack_null_error_are_truthful() {
    let root = tempfile::tempdir().unwrap();
    let p = root.path();
    let ctx = CoreContext::headless(CorePaths {
        data_dir: p.join("data"),
        config_dir: p.join("config"),
        resource_dir: p.join("resource"),
        temp_dir: p.join("missing"),
        home_dir: p.join("home"),
    });
    ctx.set_router(Arc::new(move |ch, _| {
        Box::pin(async move {
            assert_eq!(ch, SELECTED_TEXT_CHANNEL);
            Ok(json!("# No temp base exists"))
        })
    }))
    .unwrap();
    let mut state = json!({});
    let v = adapter::handle(
        &ctx,
        "mp-style-markdown",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(v["actions"].as_array().unwrap().len(), 2);
    assert_eq!(state["owned"], json!([]));
    assert!(!p.join("missing").exists());
    let t = Test::new();
    let mut state = json!({});
    let mut v = t
        .handle("extension:run-command", json!({}), &mut state)
        .await;
    for reply in [
        json!({"ok":true,"error":null}),
        json!({"ok":true,"error":null,"success":true}),
    ] {
        *t.reply.lock().unwrap() = reply;
        v = t
            .handle(
                "extension:invoke-action",
                action(&v, "Copy Original Markdown"),
                &mut state,
            )
            .await;
        assert!(v["message"]
            .as_str()
            .unwrap()
            .starts_with("Markdown Copied"));
    }
    t.handle("extension:dispose-session", json!({}), &mut state)
        .await;
    let root = tempfile::tempdir().unwrap();
    let p = root.path();
    let ctx = CoreContext::headless(CorePaths {
        data_dir: p.join("data"),
        config_dir: p.join("config"),
        resource_dir: p.join("resource"),
        temp_dir: p.to_owned(),
        home_dir: p.join("home"),
    });
    ctx.set_router(Arc::new(move |_, _| {
        Box::pin(async move { Err("private transport error <secret>".into()) })
    }))
    .unwrap();
    let mut state = json!({});
    let v = adapter::handle(
        &ctx,
        "mp-style-markdown",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(v["actions"].as_array().unwrap().len(), 2);
    assert!(!v.to_string().contains("secret"));
}
#[test]
fn bundled_icon_prerequisite_is_narrow() {
    for path in [
        "/extensions/icons/markdown-styler.png",
        "/extensions/artwork/sample.webp",
    ] {
        assert!(v2_catalog::bundled_icon(path))
    }
    for path in [
        "/tmp/icon.png",
        "/extensions/icons/../secret.png",
        "/extensions/icons/x.png?x=1",
        "/extensions/icons/x/y.png",
        "/extensions/icons/.png",
        "/extensions/icons/a.JPG",
    ] {
        assert!(!v2_catalog::bundled_icon(path))
    }
    let row =
        v2_catalog::store_item(serde_json::from_str(include_str!("../manifest.json")).unwrap());
    assert_eq!(row["iconUrl"], "/extensions/icons/markdown-styler.png");
}

#[test]
fn selected_text_test_router_uses_actual_beta_platform_dispatch() {
    let platform = include_str!("../fixtures/beta/src-tauri/src/native_core/platform.rs");
    let dispatch =
        format!("{SELECTED_TEXT_CHANNEL:?} => selected_text().await.map(Value::String),");
    assert!(platform.lines().any(|line| line.trim() == dispatch));
    assert!(!platform.contains("\"platform:selected-text\" =>"));
}
