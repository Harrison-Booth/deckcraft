//! Help › Report an Issue…: the facts and the prefilled GitHub issue address.
//!
//! The address opens `bug_report.yml` with the version, system and document type filled in, the
//! same field ids PhotoCraft's link uses. The user reviews the form and submits it on GitHub;
//! DeckCraft sends nothing. A document contributes its file type only (never its name, path or
//! contents).

use deckcraft_engine::links::NEW_ISSUE;

/// Longest single prefilled value, in bytes.
const VALUE_MAX: usize = 80;
/// Longest `system-info` block, in bytes.
const SYSTEM_MAX: usize = 400;
/// File types DeckCraft opens, by extension; anything else is reported as `other`.
const KNOWN_TYPES: &[&str] = &["deckcraft", "pptx", "potx", "ppsx", "txt", "md"];

/// What the report says about this run of DeckCraft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub version: String,
    pub os: String,
    /// One `Key: value` per line.
    pub system_info: String,
    pub document_info: String,
}

/// `s` cut to at most `limit` bytes on a character boundary.
fn bounded(s: &str, limit: usize) -> &str {
    let mut end = s.len().min(limit);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s.get(..end).unwrap_or_default()
}

/// Percent-encodes everything but RFC 3986's unreserved characters.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn os_name() -> String {
    if cfg!(target_arch = "wasm32") {
        return "Web".into();
    }
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        "freebsd" => "FreeBSD",
        other => other,
    };
    format!("{os} {}", std::env::consts::ARCH)
}

/// The document's file type for the report: its extension when it is one we open, `new (unsaved)`
/// without a path, `none` without a document.
fn document_type(open: bool, path: Option<&str>) -> String {
    let kind = match (open, path) {
        (false, _) => "none".to_string(),
        (true, None) => "new (unsaved)".to_string(),
        (true, Some(p)) => {
            let ext = p.rsplit(['/', '\\']).next().and_then(|name| name.rsplit_once('.')).map(|(_, e)| e.to_ascii_lowercase());
            ext.filter(|e| KNOWN_TYPES.contains(&e.as_str())).unwrap_or_else(|| "other".into())
        }
    };
    format!("Document type: {kind}")
}

/// The facts for a report. `graphics` is the adapter line the host recorded, `scale` the
/// display's pixels per point, `doc` whether a presentation is open and its path.
pub fn facts(graphics: Option<&str>, scale: f32, doc: Option<Option<&str>>) -> Facts {
    let version = format!("DeckCraft {}", crate::dialogs::version_line().trim_start_matches("Version "));
    let system_info = format!(
        "Build: {}\nGraphics: {}\nDisplay scale: {scale}",
        if cfg!(debug_assertions) { "debug" } else { "release" },
        bounded(graphics.unwrap_or("unknown"), VALUE_MAX),
    );
    Facts {
        version: bounded(&version, VALUE_MAX).to_string(),
        os: os_name(),
        system_info: bounded(&system_info, SYSTEM_MAX).to_string(),
        document_info: document_type(doc.is_some(), doc.flatten()),
    }
}

/// The address of the prefilled form.
pub fn url(f: &Facts) -> String {
    format!(
        "{NEW_ISSUE}?template=bug_report.yml&version={}&os={}&system-info={}&document-info={}",
        encode(bounded(&f.version, VALUE_MAX)),
        encode(bounded(&f.os, VALUE_MAX)),
        encode(bounded(&f.system_info, SYSTEM_MAX)),
        encode(bounded(&f.document_info, VALUE_MAX)),
    )
}

/// The same facts as plain text, for pasting into an issue by hand.
pub fn details_text(f: &Facts) -> String {
    format!("Version: {}\nOperating system: {}\n{}\n{}", f.version, f.os, f.system_info, f.document_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(url: &str) -> Vec<(&str, &str)> {
        url.split_once('?').map(|(_, q)| q).unwrap_or_default().split('&').filter_map(|kv| kv.split_once('=')).collect()
    }

    fn decode(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = vec![];
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' {
                out.push(u8::from_str_radix(s.get(i + 1..i + 3).unwrap_or("zz"), 16).unwrap_or(b'?'));
                i += 3;
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap_or_default()
    }

    #[test]
    fn the_address_has_the_form_and_four_fields() {
        let u = url(&facts(Some("Test GPU (Vulkan, DiscreteGpu)"), 2.0, Some(Some("/home/ann/talk.pptx"))));
        assert!(u.starts_with("https://github.com/storytold/deckcraft/issues/new?template=bug_report.yml&"));
        let keys: Vec<_> = params(&u).iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, ["template", "version", "os", "system-info", "document-info"]);
        let info = decode(params(&u)[3].1);
        assert!(info.contains("Graphics: Test GPU (Vulkan, DiscreteGpu)") && info.contains("Display scale: 2"), "{info}");
        assert_eq!(decode(params(&u)[4].1), "Document type: pptx");
    }

    #[test]
    fn hostile_values_stay_inside_their_own_parameter() {
        let f = Facts {
            version: "v & x=1 ? # \n 100% 日本語".into(),
            os: "a&b=c".into(),
            system_info: "Graphics: GPU&template=evil#x".into(),
            document_info: "Document type: pptx".into(),
        };
        let u = url(&f);
        assert_eq!(params(&u).len(), 5, "{u}");
        assert!(!u.contains(['#', ' ', '\n']) && u.is_ascii());
        assert_eq!(decode(params(&u)[1].1), f.version);
        assert_eq!(decode(params(&u)[3].1), f.system_info);
    }

    #[test]
    fn values_are_capped_on_a_character_boundary() {
        let big = "ü".repeat(10_000);
        let f = facts(Some(&big), 1.0, None);
        let u = url(&Facts { version: big.clone(), os: big.clone(), ..f });
        assert!(u.len() < 2048, "{} bytes", u.len());
        for (k, v) in params(&u).into_iter().skip(1) {
            assert!(!decode(v).is_empty(), "{k} cut inside a character");
        }
        assert_eq!(bounded("aü", 2), "a");
    }

    #[test]
    fn only_the_file_type_of_the_document_is_reported() {
        let secret = "/home/ann/Secret Budget.PPTX";
        let f = facts(None, 1.0, Some(Some(secret)));
        let all = format!("{}{}", url(&f), details_text(&f)).to_lowercase();
        assert_eq!(f.document_info, "Document type: pptx");
        assert!(!all.contains("ann") && !all.contains("secret") && !all.contains("budget"), "{all}");
        assert_eq!(facts(None, 1.0, Some(Some("C:\\x\\deck.odp"))).document_info, "Document type: other");
        assert_eq!(facts(None, 1.0, Some(Some("/tmp/.pptx.bak"))).document_info, "Document type: other");
        assert_eq!(facts(None, 1.0, Some(None)).document_info, "Document type: new (unsaved)");
        assert_eq!(facts(None, 1.0, None).document_info, "Document type: none");
    }

    #[test]
    fn details_hold_everything_the_form_would_get() {
        let f = facts(None, 1.0, None);
        let t = details_text(&f);
        for part in [&f.version, &f.os, &f.system_info, &f.document_info] {
            assert!(t.contains(part.as_str()));
        }
        assert!(f.system_info.contains("Graphics: unknown"));
    }
}
