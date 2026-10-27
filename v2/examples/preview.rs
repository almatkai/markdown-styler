//! Emits fixture-only documents to stdout for headless helper verification.
use tezbar_markdown_styler::{self as core, document};
fn main() {
    let sample = core::render::styled(document::SAMPLE).unwrap();
    let attack="# Fixture heading\n\n<script>window.untrusted=1</script>\n\n<img src=\"https://never.invalid/pixel\" onerror=\"window.untrusted=2\">\n\n<iframe srcdoc=\"<script>parent.untrusted=3</script>\"></iframe>\n\n<meta http-equiv=refresh content=\"0;url=https://never.invalid\">\n\n[unsafe](javascript:alert(1))\n\n```html\n<script>window.untrusted=4</script>\n```";
    let styled = core::render::styled(attack).unwrap();
    println!(
        "{}",
        serde_json::json!({"sample":document::preview(&sample,true).unwrap(),"normal":document::preview(&styled,false).unwrap(),"styled":styled,"detail":core::safe_detail(attack)})
    );
}
