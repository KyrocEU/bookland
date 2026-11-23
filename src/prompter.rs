#[derive(Clone, Debug)]
pub struct Chunk {
    pub text: String,
    pub start_word: usize,
    pub end_word: usize,
}

pub fn words(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect()
}

pub fn chunks(script: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut word_count = 0;
    for paragraph in script.lines() {
        let mut segment = Vec::new();
        let mut segment_start = word_count;
        for token in paragraph.split_whitespace() {
            segment.push(token);
            word_count += words(token).len();
            if segment.len() == 12 {
                chunks.push(Chunk { text: segment.join(" "),
                    start_word: segment_start, end_word: word_count });
                segment.clear();
                segment_start = word_count;
            }
        }
        if !segment.is_empty() {
            chunks.push(Chunk { text: segment.join(" "),
                start_word: segment_start, end_word: word_count });
        } else if paragraph.trim().is_empty() {
            chunks.push(Chunk { text: String::new(),
                start_word: word_count, end_word: word_count });
        }
    }
    chunks
}

// Partial speech results are revised as the speaker talks. Match a short tail
// against nearby script words and never move backward on a weaker revision.
pub fn advance(script: &str, transcript: &str, current: usize) -> usize {
    let script_words = words(script);
    let spoken = words(transcript);
    if script_words.is_empty() || spoken.is_empty() { return current; }
    let search_start = current.saturating_sub(8);
    let search_end = (current + 100).min(script_words.len());
    for length in (2..=spoken.len().min(7)).rev() {
        let tail = &spoken[spoken.len() - length..];
        let mut best = current;
        for index in search_start..search_end.saturating_sub(length - 1) {
            if &script_words[index..index + length] == tail {
                best = best.max(index + length);
            }
        }
        if best > current { return best; }
    }
    let last = spoken.last().unwrap();
    if last.chars().count() >= 4 {
        for index in current..(current + 12).min(script_words.len()) {
            if &script_words[index] == last { return index + 1; }
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_match_moves_forward_without_regressing() {
        let script = "Welcome to Bookland. Today we will see the new island.\nLet's begin.";
        assert_eq!(advance(script, "Welcome to Bookland", 0), 3);
        assert_eq!(advance(script, "Today we will see", 3), 7);
        assert_eq!(advance(script, "Welcome to", 7), 7);
        assert_eq!(advance("שלום עולם. כאן מתחילים", "שלום עולם", 0), 2);
    }

    #[test]
    fn chunks_preserve_word_positions() {
        let chunks = chunks("One two three\n\nFour five");
        assert_eq!(chunks[0].end_word, 3);
        assert_eq!(chunks[2].start_word, 3);
        assert_eq!(chunks[2].end_word, 5);
    }
}
