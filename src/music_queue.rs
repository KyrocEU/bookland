//! A bounded preview of the current Apple Music playlist order. Music's
//! scripting dictionary does not expose manually queued Up Next items, so
//! never present this as the complete playback queue.

use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub title: String,
    pub artist: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    pub tracks: Vec<Track>,
    pub status: String,
}

const SCRIPT: &str = r#"tell application "Music"
    try
        if player state is stopped then return "NO_TRACK"
        if shuffle enabled then return "SHUFFLE"
        set playlistRef to current playlist
        set playingRef to current track
        set playingIndex to index of playingRef
        set trackCount to count of tracks of playlistRef
        set playingID to persistent ID of playingRef
        if playingIndex < 1 or playingIndex > trackCount then set playingIndex to 0
        if playingIndex > 0 then
            if (persistent ID of track playingIndex of playlistRef) is not playingID then set playingIndex to 0
        end if
        if playingIndex is 0 then
            if trackCount > 2000 then return "LARGE"
            set playlistIDs to persistent ID of every track of playlistRef
            repeat with candidateIndex from 1 to (count of playlistIDs)
                if (item candidateIndex of playlistIDs as text) is playingID then
                    set playingIndex to candidateIndex
                    exit repeat
                end if
            end repeat
        end if
        if playingIndex is 0 then return "UNAVAILABLE"
        if playingIndex is trackCount then return "END"
        set finishAt to playingIndex + 3
        if finishAt > trackCount then set finishAt to trackCount
        set outputText to ""
        repeat with trackIndex from (playingIndex + 1) to finishAt
            set nextRef to track trackIndex of playlistRef
            set outputText to outputText & (name of nextRef as text) & (ASCII character 31) & (artist of nextRef as text) & linefeed
        end repeat
        return outputText
    on error
        return "UNAVAILABLE"
    end try
end tell"#;

pub fn read() -> Preview {
    super::osascript_with_timeout(SCRIPT, Duration::from_secs(3))
        .map(|text| parse(&text))
        .unwrap_or_else(|| Preview {
            tracks: Vec::new(),
            status: "Apple Music did not respond. Check its automation permission.".into(),
        })
}

fn parse(text: &str) -> Preview {
    let status = match text.trim() {
        "NO_TRACK" => "Play a song in Apple Music to see the playlist order.",
        "SHUFFLE" => "Shuffle is on; the next songs are not predictable.",
        "UNAVAILABLE" => "Apple Music did not expose this song's playlist order.",
        "LARGE" => "This playlist is too large for a lightweight preview.",
        "END" => "This is the last song in the current playlist.",
        _ => "",
    };
    if !status.is_empty() {
        return Preview { tracks: Vec::new(), status: status.into() };
    }
    let tracks = text.lines().take(3).filter_map(|line| {
        let (title, artist) = line.split_once('\u{1f}')?;
        let title = title.trim();
        if title.is_empty() { return None; }
        Some(Track { title: title.into(), artist: artist.trim().into() })
    }).collect::<Vec<_>>();
    let status = if tracks.is_empty() {
        "Apple Music did not expose this song's playlist order.".into()
    } else {
        "Next in the current playlist · manual queue items are not shown".into()
    };
    Preview { tracks, status }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_only_three_ordered_tracks() {
        let preview = parse("One\u{1f}Artist A\nTwo\u{1f}Artist B\nThree\u{1f}Artist C\nFour\u{1f}Artist D\n");
        assert_eq!(preview.tracks.len(), 3);
        assert_eq!(preview.tracks[1].title, "Two");
        assert!(preview.status.contains("manual queue"));
    }

    #[test]
    fn shuffle_does_not_claim_a_queue() {
        let preview = parse("SHUFFLE");
        assert!(preview.tracks.is_empty());
        assert!(preview.status.contains("Shuffle"));
    }

    #[test]
    fn large_playlist_stays_bounded() {
        let preview = parse("LARGE");
        assert!(preview.tracks.is_empty());
        assert!(preview.status.contains("lightweight"));
    }
}
