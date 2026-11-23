on run argv
    if (count of argv) is not 1 then error "Expected artwork output path"
    set outputPath to POSIX file (item 1 of argv)
    with timeout of 8 seconds
        tell application "Music"
            if not (exists current track) then return "none"
            if (count of artworks of current track) is 0 then return "none"
            set artData to raw data of artwork 1 of current track
        end tell
    end timeout
    set outputFile to open for access outputPath with write permission
    try
        set eof of outputFile to 0
        write artData to outputFile
        close access outputFile
    on error errMsg
        try
            close access outputFile
        end try
        error errMsg
    end try
    return "saved"
end run
