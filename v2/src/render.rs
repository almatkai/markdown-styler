//! Native event parser with source-compatible renderer callbacks. The finite TSX
//! oracle defines tested marked 9.1.6 compatibility; this is not universal parity.
use crate::{MAX_DEPTH, MAX_EVENTS, MAX_OUTPUT_BYTES};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn styles() -> &'static BTreeMap<String, String> {
    static STYLES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    STYLES.get_or_init(|| {
        serde_json::from_str(include_str!("../fixtures/oracle/styles.json"))
            .expect("pinned inline styles")
    })
}
fn style(key: &str) -> &'static str {
    &styles()[key]
}
fn escape(s: &str, all: bool) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        rest = &rest[c.len_utf8()..];
        match c {
            '&' if !all && entity_len(rest).is_some() => out.push('&'),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
fn entity_len(s: &str) -> Option<usize> {
    let i = s
        .bytes()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == b'#')
        .count();
    if i > 0 && s.as_bytes().get(i) == Some(&b';') {
        Some(i + 1)
    } else {
        None
    }
}
fn text(raw: &str, parsed: &str) -> String {
    if raw.contains('&') && !raw.contains(['\n', '\r']) {
        let mut fixed = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' && chars.peek().is_some_and(|c| c.is_ascii_punctuation()) {
                fixed.push(chars.next().expect("peeked"));
            } else {
                fixed.push(c)
            }
        }
        fixed
    } else {
        parsed.to_owned()
    }
}
#[derive(Debug)]
struct Frame {
    tag: Tag<'static>,
    html: String,
    image_alt: String,
    suffix: String,
    indent: usize,
}
fn simple(tag: &str, key: &str, body: &str, tool: bool) -> String {
    format!(
        "<{tag}{} style=\"{}\">{body}</{tag}>",
        if tool { " data-tool=\"mp-styler\"" } else { "" },
        style(key)
    )
}
fn finish(f: Frame) -> String {
    let b = f.html;
    match f.tag {
        Tag::Paragraph=>simple("p","p",&b,true),
        Tag::Heading{level,..}=>simple(&level.to_string(),&level.to_string(),&b,true),
        Tag::BlockQuote(_)=>simple("blockquote","blockquote",&b,true),
        Tag::List(ordered)=>{let tag=if ordered.is_some(){"ol"}else{"ul"};simple(tag,tag,&b,true)},
        Tag::Item=>format!("<li style=\"{}\"><section style=\"margin-right: 0px; margin-left: 0px; margin-top: 5px; margin-bottom: 5px; color: rgb(1, 1, 1); font-size: 16px; letter-spacing: 0em; text-align: left; font-weight: normal; line-height: 1.8 !important;\">{b}</section></li>",style("li")),
        Tag::CodeBlock(_)=>format!("<pre style=\"{}\"><code style=\"{}\">{}</code></pre>",style("pre"),style("preCode"),b.strip_suffix('\n').unwrap_or(&b)),
        Tag::Emphasis=>format!("<em>{b}</em>"),
        Tag::Strong=>simple("strong","strong",&b,false),
        Tag::Strikethrough=>format!("<del>{b}</del>"),
        Tag::Link{dest_url,title,link_type,..}=>format!("<a href=\"{}\" style=\"{}\" {}>{b}</a>",if link_type==LinkType::Email{format!("mailto:{dest_url}")}else{dest_url.into_string()},style("a"),if title.is_empty(){String::new()}else{format!("title=\"{}\"",escape(&title,false))}),
        Tag::Image{dest_url,title,..}=>format!("<img src=\"{}\" alt=\"{}\" style=\"{}\" {}>",dest_url,f.image_alt,style("img"),if title.is_empty(){String::new()}else{format!("title=\"{}\"",escape(&title,false))}),
        Tag::Table(_)=>format!("<table data-tool=\"mp-styler\" style=\"{}\">{b}</tbody></table>",style("table")),
        Tag::TableHead=>format!("<thead><tr>{b}</tr></thead><tbody>"),
        Tag::TableRow=>format!("<tr>{b}</tr>"),
        Tag::TableCell=>b,
        Tag::HtmlBlock=>b+&f.suffix,
        _=>b,
    }
}
pub fn styled(markdown: &str) -> Result<String, &'static str> {
    let html = parse(markdown)?;
    let result=format!("<section id=\"mp-styler\" data-tool=\"mp-styler\" data-website=\"https://github.com\" style=\"{}\">{html}</section>",style("section"));
    if result.len() > MAX_OUTPUT_BYTES {
        return Err("Styled HTML exceeds the 8 MiB resource limit");
    }
    Ok(result)
}
pub fn parse(input: &str) -> Result<String, &'static str> {
    if input.len() > crate::MAX_INPUT_BYTES {
        return Err("Selected text exceeds the 256 KiB resource limit");
    }
    // Marked normalizes CRLF and bare CR before block lexing.
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let normalized = normalized
        .split('\n')
        .map(|line| {
            let leading = line.len() - line.trim_start_matches(' ').len();
            let tabs = line[leading..].chars().take_while(|c| *c == '\t').count();
            format!(
                "{}{}{}",
                &line[..leading],
                "    ".repeat(tabs),
                &line[leading + tabs..]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut stack: Vec<Frame> = Vec::new();
    let mut output = String::new();
    let mut header = false;
    let mut raw_inline = false;
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    for (count, (event, range)) in Parser::new_ext(&normalized, options)
        .into_offset_iter()
        .enumerate()
    {
        if count >= MAX_EVENTS {
            return Err("Markdown exceeds the 100000 event work limit");
        }
        let code = stack
            .last()
            .is_some_and(|f| matches!(f.tag, Tag::CodeBlock(_)));
        let s = match event {
            Event::Start(mut tag) => {
                if let Tag::Link {
                    dest_url, title, ..
                }
                | Tag::Image {
                    dest_url, title, ..
                } = &mut tag
                {
                    if let Some((href, t)) = source_link(&normalized[range.clone()]) {
                        *dest_url = href.into();
                        if let Some(t) = t {
                            *title = t.into();
                        }
                    }
                }
                if stack.len() >= MAX_DEPTH {
                    return Err("Markdown exceeds the 64 level nesting limit");
                }
                if matches!(tag, Tag::TableHead) {
                    header = true
                }
                let image_alt = if matches!(tag, Tag::Image { .. }) {
                    let raw = &normalized[range.clone()];
                    let raw = raw.strip_prefix('!').unwrap_or(raw);
                    label_end(raw)
                        .map(|end| {
                            escape(&raw[1..end].replace("\\[", "[").replace("\\]", "]"), false)
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let suffix = if matches!(tag, Tag::HtmlBlock) {
                    normalized[range.end..]
                        .chars()
                        .take_while(|c| c.is_whitespace())
                        .collect()
                } else {
                    String::new()
                };
                let indent = if matches!(tag, Tag::Item) {
                    normalized[range.start..]
                        .split('\n')
                        .next()
                        .unwrap_or("")
                        .find(' ')
                        .unwrap_or(0)
                        + 1
                } else {
                    0
                };
                stack.push(Frame {
                    tag: tag.into_static(),
                    html: String::new(),
                    image_alt,
                    suffix,
                    indent,
                });
                continue;
            }
            Event::End(end) => {
                let mut f = stack.pop().ok_or("Invalid Markdown nesting")?;
                if end == TagEnd::TableCell {
                    let t = if header { "th" } else { "td" };
                    f.html = simple(t, t, &f.html, false)
                }
                if end == TagEnd::TableHead {
                    header = false
                }
                finish(f)
            }
            Event::Text(s) if code => s.to_string(),
            Event::Text(s) => {
                let rendered = text(&normalized[range], &s);
                if raw_inline {
                    rendered
                } else if stack
                    .iter()
                    .any(|f| matches!(f.tag, Tag::Link { .. } | Tag::Image { .. }))
                {
                    escape(&rendered, false)
                } else {
                    autolinks(&rendered)
                }
            }
            Event::Code(s) => simple("code", "code", &escape(&s, true), false),
            Event::Html(s) | Event::InlineHtml(s) => {
                let lower = s.to_ascii_lowercase();
                for tag in ["code", "pre", "kbd", "script"] {
                    if lower.starts_with(&format!("<{tag}>"))
                        || lower.starts_with(&format!("<{tag} "))
                    {
                        raw_inline = true;
                    }
                    if lower.starts_with(&format!("</{tag}>")) {
                        raw_inline = false;
                    }
                }
                s.to_string()
            }
            Event::SoftBreak => {
                let spaces = normalized[range.end..]
                    .bytes()
                    .take_while(|b| *b == b' ')
                    .count();
                let indent = stack
                    .iter()
                    .rev()
                    .find(|f| matches!(f.tag, Tag::Item))
                    .map_or(0, |f| f.indent);
                format!("\n{}", " ".repeat(spaces.saturating_sub(indent)))
            }
            Event::HardBreak => "<br>".into(),
            Event::Rule => "<hr>\n".into(),
            Event::TaskListMarker(checked) => {
                let mut checkbox: String = if checked {
                    "<input checked=\"\" disabled=\"\" type=\"checkbox\"> ".into()
                } else {
                    "<input disabled=\"\" type=\"checkbox\"> ".into()
                };
                if stack
                    .last()
                    .is_some_and(|f| matches!(f.tag, Tag::Paragraph))
                {
                    checkbox += "\n";
                }
                checkbox
            }
            _ => return Err("Unsupported Markdown event"),
        };
        let target = if let Some(f) = stack.last_mut() {
            &mut f.html
        } else {
            &mut output
        };
        if target.len() + s.len() > MAX_OUTPUT_BYTES {
            return Err("Styled HTML exceeds the 8 MiB resource limit");
        }
        target.push_str(&s);
    }
    if !stack.is_empty() {
        return Err("Invalid Markdown nesting");
    }
    Ok(output)
}

fn autolinks(text: &str) -> String {
    static URL: OnceLock<regex::Regex> = OnceLock::new();
    let re=URL.get_or_init(||regex::Regex::new(r"(?i)(?:ftp://|https?://|www\.)(?:[a-z0-9-]+\.?)+[^\s<]*|[a-z0-9._+-]+@[a-z0-9_-]+(?:\.[a-z0-9_-]*[a-z0-9])+").expect("fixed GFM URL grammar"));
    let mut out = String::new();
    let mut offset = 0;
    for m in re.find_iter(text) {
        out.push_str(&escape(&text[offset..m.start()], false));
        let raw = m.as_str();
        let mut end = raw.len();
        let opens = raw.bytes().filter(|b| *b == b'(').count();
        let mut closes = raw.bytes().filter(|b| *b == b')').count();
        loop {
            let old = end;
            let t = &raw[..end];
            if let Some(c) = t.chars().last() {
                if "?!.,:;*_'\"~".contains(c) || (c == ')' && closes > opens) {
                    end -= c.len_utf8();
                    if c == ')' {
                        closes = closes.saturating_sub(1);
                    }
                }
            }
            if old == end {
                break;
            }
        }
        let url = &raw[..end];
        let href = if url.contains('@') && !url.contains("://") {
            format!("mailto:{url}")
        } else if url.to_ascii_lowercase().starts_with("www.") {
            format!("http://{url}")
        } else {
            url.to_string()
        };
        out.push_str(&format!(
            "<a href=\"{href}\" style=\"{}\" >{}</a>{}",
            style("a"),
            escape(url, false),
            escape(&raw[end..], false)
        ));
        offset = m.end();
    }
    out.push_str(&escape(&text[offset..], false));
    out
}

fn label_end(raw: &str) -> Option<usize> {
    if !raw.starts_with('[') {
        return None;
    }
    let mut depth = 0;
    let mut escaped = false;
    for (i, c) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '[' {
            depth += 1
        }
        if c == ']' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}
fn unescape_punctuation(raw: &str) -> String {
    let mut out = String::new();
    let mut iter = raw.chars().peekable();
    while let Some(c) = iter.next() {
        if c == '\\' && iter.peek().is_some_and(|x| x.is_ascii_punctuation()) {
            out.push(iter.next().expect("peeked"));
        } else {
            out.push(c)
        }
    }
    out
}
fn source_link(raw: &str) -> Option<(String, Option<String>)> {
    let raw = raw.strip_prefix('!').unwrap_or(raw);
    let end = label_end(raw)?;
    let rest = raw[end + 1..].strip_prefix('(')?.trim_start();
    let mut escaped = false;
    let mut depth = 0;
    let mut end = rest.len();
    if let Some(angle) = rest.strip_prefix('<') {
        let e = angle.find('>')?;
        let tail = angle[e + 1..].trim();
        return Some((unescape_punctuation(&angle[..e]), link_title(tail)));
    }
    for (i, c) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '(' {
            depth += 1
        }
        if c == ')' {
            if depth == 0 {
                end = i;
                break;
            }
            depth -= 1
        }
        if c.is_whitespace() && depth == 0 {
            end = i;
            break;
        }
    }
    Some((
        unescape_punctuation(&rest[..end]),
        link_title(rest[end..].trim()),
    ))
}
fn link_title(tail: &str) -> Option<String> {
    let q = tail.chars().next()?;
    if !['\"', '\'', '('].contains(&q) {
        return None;
    }
    let close = if q == '(' { ')' } else { q };
    let tail = tail.strip_suffix(')')?.trim_end();
    let inner = tail[q.len_utf8()..].strip_suffix(close)?;
    Some(unescape_punctuation(inner))
}
