# Rekordbox File Conversion Tool
The purpose of this tool is to search through a user's music folder for songs that have been given a tag, such as "CONVERT_FOR_REKORDBOX", indicating it should be converted to a Rekordbox friendly format (for example, flac -> aiff). To run this program, do the following (the assumption is that you have installed Rust already, and have cloned this repo):

```rust
cargo build
cargo run home/music home/music/converted_for_rekordbox CONVERT_FOR_REKORDBOX
```
Here the first argument is the path to your music folder, the second argument is the path where you want the converted songs written to, and the last argument is the name of the tag you used to specify which songs you wanted to convert.