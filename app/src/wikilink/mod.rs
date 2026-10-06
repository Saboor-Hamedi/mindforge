//! Bidirectional Wikilink engine: parser, target resolver with nested path support, and backlink indexer.

pub mod hover_wikilink;
pub mod wikilink_autocompletion;
pub use wikilink_autocompletion as autocomplete;

use core::Note;

/// A parsed Wikilink reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLinkRef {
    /// Raw link target, e.g. "folder/sub/note" or "My Note"
    pub target: String,
    /// Optional display alias, e.g. in `[[target|Alias]]`
    pub display: Option<String>,
    /// Start character index in the source buffer
    pub start: usize,
    /// End character index in the source buffer
    pub end: usize,
}

impl WikiLinkRef {
    /// Display label for rendering (alias if present, otherwise target)
    pub fn display_label(&self) -> &str {
        if let Some(ref d) = self.display {
            d.as_str()
        } else {
            self.target.as_str()
        }
    }
}

/// An incoming backlink reference to a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacklinkItem {
    pub source_note_id: i64,
    pub source_note_title: String,
    pub line_number: usize,
    pub snippet: String,
    pub target: String,
}

/// Parses all wikilinks `[[target]]` or `[[target|alias]]` from text.
pub fn extract_wikilinks(text: &str) -> Vec<WikiLinkRef> {
    let mut links = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i + 1 < n {
        if chars[i] == '[' && chars[i + 1] == '[' {
            let start = i;
            let inner_start = i + 2;
            let mut close_idx = None;
            let mut j = inner_start;
            while j + 1 < n {
                if chars[j] == ']' && chars[j + 1] == ']' {
                    close_idx = Some(j);
                    break;
                }
                if chars[j] == '\n' {
                    break; // wikilinks cannot cross newlines
                }
                j += 1;
            }

            if let Some(close) = close_idx {
                let inner: String = chars[inner_start..close].iter().collect();
                let inner = inner.trim();
                if !inner.is_empty() {
                    let (target, display) = if let Some((tgt, alias)) = inner.split_once('|') {
                        (tgt.trim().to_string(), Some(alias.trim().to_string()))
                    } else {
                        (inner.to_string(), None)
                    };

                    links.push(WikiLinkRef {
                        target,
                        display,
                        start,
                        end: close + 2,
                    });
                }
                i = close + 2;
                continue;
            }
        }
        i += 1;
    }

    links
}

/// Resolves a wikilink target against the user's notes.
/// Supports simple names ("My Note"), nested folder paths ("folder/filename"),
/// and optional extension matching ("folder/filename.md").
pub fn resolve_wikilink<'a>(target: &str, notes: &'a [Note]) -> Option<&'a Note> {
    let clean_target = target.trim();
    if clean_target.is_empty() {
        return None;
    }

    // 1. Direct exact or case-insensitive match on note topic/title
    if let Some(note) = notes
        .iter()
        .find(|n| n.topic.eq_ignore_ascii_case(clean_target))
    {
        return Some(note);
    }

    // 2. Strip optional .md suffix and retry
    let target_no_ext = clean_target.strip_suffix(".md").unwrap_or(clean_target);
    if let Some(note) = notes.iter().find(|n| {
        let topic_no_ext = n.topic.strip_suffix(".md").unwrap_or(&n.topic);
        topic_no_ext.eq_ignore_ascii_case(target_no_ext)
    }) {
        return Some(note);
    }

    // 3. Nested folder path matching:
    // If target has slashes (e.g. "folder/note"), match topic ending with target
    if clean_target.contains('/') || clean_target.contains('\\') {
        let norm_target = clean_target.replace('\\', "/");
        if let Some(note) = notes.iter().find(|n| {
            let norm_topic = n.topic.replace('\\', "/");
            norm_topic.eq_ignore_ascii_case(&norm_target)
                || norm_topic.ends_with(&format!("/{}", norm_target))
        }) {
            return Some(note);
        }
    } else {
        // If target is just a leaf filename, match either exact topic or topic with folder prefix
        if let Some(note) = notes.iter().find(|n| {
            let leaf = n.topic.rsplit(&['/', '\\'][..]).next().unwrap_or(&n.topic);
            let leaf_no_ext = leaf.strip_suffix(".md").unwrap_or(leaf);
            leaf.eq_ignore_ascii_case(clean_target)
                || leaf_no_ext.eq_ignore_ascii_case(clean_target)
        }) {
            return Some(note);
        }
    }

    None
}

/// Finds all incoming backlinks pointing to `target_topic` across all notes in the vault.
pub fn find_backlinks(
    target_topic: &str,
    notes: &[Note],
    current_id: Option<i64>,
) -> Vec<BacklinkItem> {
    let mut backlinks = Vec::new();
    let clean_target = target_topic.trim();
    if clean_target.is_empty() {
        return backlinks;
    }

    let target_leaf = clean_target
        .rsplit(&['/', '\\'][..])
        .next()
        .unwrap_or(clean_target);

    for note in notes {
        // Exclude self-references if note has an id
        if let Some(cur_id) = current_id {
            if note.id == cur_id {
                continue;
            }
        }

        for (line_idx, line) in note.body.lines().enumerate() {
            let links = extract_wikilinks(line);
            for link in links {
                let matches = link.target.eq_ignore_ascii_case(clean_target)
                    || link.target.eq_ignore_ascii_case(target_leaf)
                    || link.target.ends_with(&format!("/{}", clean_target));

                if matches {
                    let snippet = line.trim().to_string();
                    backlinks.push(BacklinkItem {
                        source_note_id: note.id,
                        source_note_title: note.topic.clone(),
                        line_number: line_idx + 1,
                        snippet,
                        target: link.target,
                    });
                }
            }
        }
    }

    backlinks
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_extract_wikilinks_simple_and_nested() {
        let text = "Check [[Architecture]] and [[docs/specs/networking]] or [[Guide|User Manual]].";
        let links = extract_wikilinks(text);
        assert_eq!(links.len(), 3);

        assert_eq!(links[0].target, "Architecture");
        assert_eq!(links[0].display, None);
        assert_eq!(links[0].display_label(), "Architecture");

        assert_eq!(links[1].target, "docs/specs/networking");
        assert_eq!(links[1].display, None);

        assert_eq!(links[2].target, "Guide");
        assert_eq!(links[2].display, Some("User Manual".into()));
        assert_eq!(links[2].display_label(), "User Manual");
    }

    #[test]
    fn test_resolve_nested_wikilinks() {
        let now = Local::now().naive_local();
        let notes = vec![
            Note {
                id: 1,
                topic: "docs/architecture.md".into(),
                body: "Content".into(),
                struggled_with: None,
                created_at: now,
            },
            Note {
                id: 2,
                topic: "Inbox".into(),
                body: "Notes".into(),
                struggled_with: None,
                created_at: now,
            },
        ];

        // Match leaf name
        let found = resolve_wikilink("architecture", &notes);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, 1);

        // Match nested path
        let found_nested = resolve_wikilink("docs/architecture", &notes);
        assert!(found_nested.is_some());
        assert_eq!(found_nested.unwrap().id, 1);
    }

    #[test]
    fn test_find_backlinks() {
        let now = Local::now().naive_local();
        let notes = vec![
            Note {
                id: 1,
                topic: "Source Note".into(),
                body: "As discussed in [[Target Note]] today.".into(),
                struggled_with: None,
                created_at: now,
            },
            Note {
                id: 2,
                topic: "Target Note".into(),
                body: "I am the target.".into(),
                struggled_with: None,
                created_at: now,
            },
        ];

        let bl = find_backlinks("Target Note", &notes, Some(2));
        assert_eq!(bl.len(), 1);
        assert_eq!(bl[0].source_note_id, 1);
        assert_eq!(bl[0].source_note_title, "Source Note");
    }
}
