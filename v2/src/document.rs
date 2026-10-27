//! External browser document. Styled HTML bytes are retained for export parity;
//! only the browser shell gains CSP isolation and a fixed helper correction.
use sha2::{Digest, Sha256};
pub const SAMPLE: &str = include_str!("../templates/sample.md");
pub fn original(styled: &str, sample: bool) -> String {
    let template = if sample {
        include_str!("../templates/sample.html")
    } else {
        include_str!("../templates/normal.html")
    };
    template.replace("@@STYLED_HTML@@", styled)
}
pub fn preview(styled: &str, sample: bool) -> Result<String, &'static str> {
    let document = original("@@STYLED_HTML@@", sample);
    let start = document
        .find("<script>")
        .ok_or("Missing fixed preview helper")?;
    let end = document[start..]
        .find("</script>")
        .ok_or("Missing fixed preview helper")?
        + start;
    let mut helper = document[start + 8..end].to_owned();
    if sample {
        helper=helper.replace("document.execCommand('copy');\n      selection.removeAllRanges();\n      alert('Content copied! Paste it into your content editor.');", "let copied = false;\n      try { copied = document.execCommand('copy') === true; } catch (_) {}\n      selection.removeAllRanges();\n      alert(copied ? 'Content copied! Paste it into your content editor.' : 'Copy failed. Please try again.');");
        // CSP rejects inline event handlers. Preserve the button and attach the
        // same fixed helper with an allowed script instead.
        helper.push_str(
            "\n    document.querySelector('.button').addEventListener('click', copyContent);\n",
        );
    }
    // Hash based CSP allows only this fixed helper, not source-derived scripts,
    // event handlers, frames, forms, navigation, fetches, remote images or CSS URLs.
    let hash = base64(&Sha256::digest(helper.as_bytes()));
    let csp=format!("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src 'sha256-{hash}'; script-src-attr 'none'; style-src 'unsafe-inline'; img-src data:; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'\">\n  ");
    let mut doc = format!(
        "{}<script>{helper}</script>{}",
        &document[..start],
        &document[end + 9..]
    );
    if sample {
        doc = doc.replace(" onclick=\"copyContent()\"", "")
    }
    // Browser markup is cleaned separately; styled/export bytes stay exact.
    let isolated = neutralize(styled);
    doc = doc.replace("@@STYLED_HTML@@", &isolated);
    doc = doc.replacen(
        "<meta charset=\"UTF-8\">",
        &format!("<meta charset=\"UTF-8\">\n  {csp}"),
        1,
    );
    if doc.len() > crate::MAX_OUTPUT_BYTES {
        return Err("Browser document exceeds the 8 MiB resource limit");
    }
    Ok(doc)
}
fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = ((c[0] as u32) << 16)
            | ((c.get(1).copied().unwrap_or(0) as u32) << 8)
            | c.get(2).copied().unwrap_or(0) as u32;
        s.push(ABC[((n >> 18) & 63) as usize] as char);
        s.push(ABC[((n >> 12) & 63) as usize] as char);
        s.push(if c.len() > 1 {
            ABC[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            ABC[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    s
}
// Parse as HTML, drop scripts/meta/base/frames/forms/events and source IDs/classes.
// Keep inline styling; CSP blocks CSS URL fetches and external resources.
fn neutralize(html: &str) -> String {
    use std::collections::HashSet;
    let tags: HashSet<&str> = [
        "section",
        "p",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "blockquote",
        "ul",
        "ol",
        "li",
        "pre",
        "code",
        "a",
        "strong",
        "img",
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        "em",
        "del",
        "br",
        "hr",
        "span",
        "div",
        "input",
    ]
    .into_iter()
    .collect();
    let mut builder = ammonia::Builder::default();
    builder
        .tags(tags)
        .add_generic_attributes(["style", "data-tool", "data-website"])
        .add_tag_attributes("input", ["type", "disabled", "checked"])
        .set_tag_attribute_value("input", "type", "checkbox")
        .set_tag_attribute_value("input", "disabled", "")
        .url_schemes(["https", "http", "mailto", "data"].into_iter().collect())
        .url_relative(ammonia::UrlRelative::Deny)
        .link_rel(None);
    let clean = builder.clean(html).to_string();
    clean.replacen("<section ", "<section id=\"mp-styler\" ", 1)
}
