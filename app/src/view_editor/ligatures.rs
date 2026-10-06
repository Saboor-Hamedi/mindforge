//! Coding font ligature scanner and tests.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LigatureKind {
    ArrowRight,            // ->
    ArrowLeft,             // <-
    FatArrowRight,         // =>
    TripleEquals,          // ===
    NotTripleEquals,       // !==
    DoubleEquals,          // ==
    NotEquals,             // !=
    LessOrEqual,           // <=
    GreaterOrEqual,        // >=
    LongArrowRight,        // -->
    LongArrowLeft,         // <--
    LongFatArrowRight,     // ==>
    LongFatArrowLeft,      // <==
    VeryLongArrowRight,    // --->
    VeryLongArrowLeft,     // <---
    VeryLongFatArrowRight, // ===>
    VeryLongFatArrowLeft,  // <===
}

impl LigatureKind {
    pub fn char_len(&self) -> usize {
        match self {
            Self::VeryLongArrowRight
            | Self::VeryLongArrowLeft
            | Self::VeryLongFatArrowRight
            | Self::VeryLongFatArrowLeft => 4,
            Self::TripleEquals
            | Self::NotTripleEquals
            | Self::LongArrowRight
            | Self::LongArrowLeft
            | Self::LongFatArrowRight
            | Self::LongFatArrowLeft => 3,
            Self::ArrowRight
            | Self::ArrowLeft
            | Self::FatArrowRight
            | Self::DoubleEquals
            | Self::NotEquals
            | Self::LessOrEqual
            | Self::GreaterOrEqual => 2,
        }
    }
}

pub fn detect_ligature(chars: &[char], i: usize) -> Option<LigatureKind> {
    if i + 4 <= chars.len() {
        match (chars[i], chars[i + 1], chars[i + 2], chars[i + 3]) {
            ('=', '=', '=', '>') => return Some(LigatureKind::VeryLongFatArrowRight),
            ('<', '=', '=', '=') => return Some(LigatureKind::VeryLongFatArrowLeft),
            ('-', '-', '-', '>') => return Some(LigatureKind::VeryLongArrowRight),
            ('<', '-', '-', '-') => return Some(LigatureKind::VeryLongArrowLeft),
            _ => {}
        }
    }
    if i + 3 <= chars.len() {
        match (chars[i], chars[i + 1], chars[i + 2]) {
            ('=', '=', '>') => return Some(LigatureKind::LongFatArrowRight),
            ('<', '=', '=') => return Some(LigatureKind::LongFatArrowLeft),
            ('-', '-', '>') => return Some(LigatureKind::LongArrowRight),
            ('<', '-', '-') => return Some(LigatureKind::LongArrowLeft),
            ('=', '=', '=') => return Some(LigatureKind::TripleEquals),
            ('!', '=', '=') => return Some(LigatureKind::NotTripleEquals),
            _ => {}
        }
    }
    if i + 2 <= chars.len() {
        match (chars[i], chars[i + 1]) {
            ('-', '>') => return Some(LigatureKind::ArrowRight),
            ('<', '-') => return Some(LigatureKind::ArrowLeft),
            ('=', '>') => return Some(LigatureKind::FatArrowRight),
            ('<', '=') => return Some(LigatureKind::LessOrEqual),
            ('>', '=') => return Some(LigatureKind::GreaterOrEqual),
            ('=', '=') => return Some(LigatureKind::DoubleEquals),
            ('!', '=') => return Some(LigatureKind::NotEquals),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::assert_eq;

    #[test]
    pub fn test_detect_ligatures() {
        let chars: Vec<char> = "fn foo() -> i32 => val === 1 !== 0 <= 5 >= 2 != 3 == 4"
            .chars()
            .collect();
        // find ->
        let arrow_pos = chars.windows(2).position(|w| w == ['-', '>']).unwrap();
        assert_eq!(
            detect_ligature(&chars, arrow_pos),
            Some(LigatureKind::ArrowRight)
        );

        // find =>
        let fat_pos = chars.windows(2).position(|w| w == ['=', '>']).unwrap();
        assert_eq!(
            detect_ligature(&chars, fat_pos),
            Some(LigatureKind::FatArrowRight)
        );

        // find ===
        let triple_pos = chars.windows(3).position(|w| w == ['=', '=', '=']).unwrap();
        assert_eq!(
            detect_ligature(&chars, triple_pos),
            Some(LigatureKind::TripleEquals)
        );

        // find !==
        let not_triple_pos = chars.windows(3).position(|w| w == ['!', '=', '=']).unwrap();
        assert_eq!(
            detect_ligature(&chars, not_triple_pos),
            Some(LigatureKind::NotTripleEquals)
        );

        // find <=
        let le_pos = chars.windows(2).position(|w| w == ['<', '=']).unwrap();
        assert_eq!(
            detect_ligature(&chars, le_pos),
            Some(LigatureKind::LessOrEqual)
        );

        // find >=
        let ge_pos = chars.windows(2).position(|w| w == ['>', '=']).unwrap();
        assert_eq!(
            detect_ligature(&chars, ge_pos),
            Some(LigatureKind::GreaterOrEqual)
        );

        // find !=
        let ne_pos = chars
            .windows(2)
            .position(|w| w == ['!', '='] && chars.get(arrow_pos).is_some())
            .unwrap();
        assert_eq!(
            detect_ligature(&chars, ne_pos),
            Some(LigatureKind::NotTripleEquals)
        );

        // find the standalone !=
        let standalone_ne = chars
            .windows(2)
            .enumerate()
            .find(|&(idx, w)| w == ['!', '='] && chars.get(idx + 2) != Some(&'='))
            .unwrap()
            .0;
        assert_eq!(
            detect_ligature(&chars, standalone_ne),
            Some(LigatureKind::NotEquals)
        );

        // find ==
        let eq_pos = chars
            .windows(2)
            .enumerate()
            .find(|&(idx, w)| {
                w == ['=', '=']
                    && (idx == 0 || chars[idx - 1] != '=')
                    && chars.get(idx + 2) != Some(&'=')
            })
            .unwrap()
            .0;
        assert_eq!(
            detect_ligature(&chars, eq_pos),
            Some(LigatureKind::DoubleEquals)
        );

        // Test multi-char arrows ==>, ===>, -->, --->
        let fat_chars: Vec<char> = "==> ===> --> --->".chars().collect();
        assert_eq!(
            detect_ligature(&fat_chars, 0),
            Some(LigatureKind::LongFatArrowRight)
        );
        assert_eq!(
            detect_ligature(&fat_chars, 4),
            Some(LigatureKind::VeryLongFatArrowRight)
        );
        assert_eq!(
            detect_ligature(&fat_chars, 9),
            Some(LigatureKind::LongArrowRight)
        );
        assert_eq!(
            detect_ligature(&fat_chars, 13),
            Some(LigatureKind::VeryLongArrowRight)
        );
    }
}
