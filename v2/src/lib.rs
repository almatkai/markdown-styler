//! Offline conversion; no selection, clipboard, network, browser, or persistent history.
pub mod document;
pub mod render;
pub mod temp;
use serde::{Deserialize, Serialize};

pub const MAX_INPUT_BYTES: usize = 256 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_EVENTS: usize = 100_000;
pub const MAX_DEPTH: usize = 64;
pub const MAX_FILES: usize = 16;
pub const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;

pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, '\u{9}'..='\u{d}' | '\u{20}' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Classification {
    Empty,
    TooShort,
    Plain,
    Markdown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Converted {
    pub markdown: String,
    pub class: Classification,
    pub preview_utf16: Vec<u16>,
    pub ellipsis: bool,
    pub styled_html: String,
}
impl Converted {
    pub fn status(&self) -> &'static str {
        match self.class {
            Classification::Empty => {
                "❌ No text selected\n\nPlease select some Markdown text and try again."
            }
            Classification::TooShort => {
                "⚠️ Selected text is too short\n\nPlease select longer Markdown content."
            }
            Classification::Plain => "⚠️ Plain text converted (may not need Markdown formatting)",
            Classification::Markdown => "✅ Markdown with formatting converted successfully",
        }
    }
    pub fn display(&self, time: &str, execution_id: &str) -> String {
        if self.styled_html.is_empty() {
            return self.status().into();
        }
        let mut preview = String::new();
        // Preserve JS substring's UTF-16 boundary exactly in the core. JSON/UTF-8 cannot
        // represent an unpaired unit: display it as a visible escape, never U+FFFD.
        for scalar in char::decode_utf16(self.preview_utf16.iter().copied()) {
            match scalar {
                Ok(c) => preview.push(c),
                Err(e) => preview.push_str(&format!("\\u{:04X}", e.unpaired_surrogate())),
            }
        }
        format!("{}\n\n🕐 Processed at: {time}\n🔍 Execution ID: {execution_id}\n\nPreview:\n\n{preview}{}",self.status(),if self.ellipsis {"..."} else {""})
    }
}
pub fn convert(input: &str) -> Result<Converted, &'static str> {
    if input.len() > MAX_INPUT_BYTES {
        return Err("Selected text exceeds the 256 KiB resource limit");
    }
    let markdown = js_trim(input);
    let len = markdown.encode_utf16().count();
    let class = if len == 0 {
        Classification::Empty
    } else if len < 10 {
        Classification::TooShort
    } else if len > 50
        || markdown.contains('\n')
        || markdown.contains(['#', '*', '_', '`', '[', ']', '(', ')', '!', '>', '-'])
    {
        Classification::Markdown
    } else {
        Classification::Plain
    };
    let styled_html = if matches!(class, Classification::Empty | Classification::TooShort) {
        String::new()
    } else {
        render::styled(markdown)?
    };
    Ok(Converted {
        markdown: markdown.into(),
        class,
        preview_utf16: markdown.encode_utf16().take(300).collect(),
        ellipsis: len > 300,
        styled_html,
    })
}
/// Detail uses the host Markdown renderer. Numeric entities prevent selected text
/// from creating code-copy controls, links, images, HTML, or formatting instructions.
pub fn safe_detail(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c == '\n' {
            out.push('\n');
        } else {
            out.push_str(&format!("&#{};", c as u32));
        }
    }
    out
}

use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};
static SERIAL: AtomicU64 = AtomicU64::new(1);
pub fn capability() -> Result<String, String> {
    static NONCE: OnceLock<Result<String, String>> = OnceLock::new();
    let nonce = NONCE.get_or_init(|| {
        let mut b = [0; 16];
        getrandom::getrandom(&mut b)
            .map_err(|_| "Unable to initialize session capabilities".to_string())?;
        {
            let mut text = String::with_capacity(32);
            const HEX: &[u8] = b"0123456789abcdef";
            for byte in b {
                text.push(HEX[(byte >> 4) as usize] as char);
                text.push(HEX[(byte & 15) as usize] as char);
            }
            Ok(text)
        }
    });
    let n = SERIAL
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| "Session capability limit reached")?;
    Ok(format!("{}:{n}", nonce.as_ref().map_err(Clone::clone)?))
}
