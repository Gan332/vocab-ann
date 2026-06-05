use crate::models::ParsedWord;

pub fn parse_txt(text: &str) -> Vec<ParsedWord> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts = if line.contains('\t') {
                let mut s = line.splitn(2, '\t');
                Some((s.next()?.to_string(), s.next()?.to_string()))
            } else if line.contains(" - ") {
                let mut s = line.splitn(2, " - ");
                Some((s.next()?.to_string(), s.next()?.to_string()))
            } else if line.contains(" | ") {
                let mut s = line.splitn(2, " | ");
                Some((s.next()?.to_string(), s.next()?.to_string()))
            } else if line.contains('|') {
                let mut s = line.splitn(2, '|');
                Some((s.next()?.to_string(), s.next()?.to_string()))
            } else if line.contains('：') {
                let mut s = line.splitn(2, '：');
                Some((s.next()?.to_string(), s.next()?.to_string()))
            } else if line.contains(':') {
                let mut s = line.splitn(2, ':');
                Some((s.next()?.to_string(), s.next()?.to_string()))
            } else {
                None
            };
            parts.map(|(word, def)| ParsedWord {
                word: word.trim().to_string(),
                definition: def.trim().to_string(),
            })
        })
        .collect()
}