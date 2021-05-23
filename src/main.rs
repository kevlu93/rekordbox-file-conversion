use serde::Deserialize;
use std::io::{self, Result, Write};
use std::process::{Command, Output};
use std::str;
mod song_info;

/// Executes the ffprobe command to get the stream and format info.
pub fn run_ffprobe(path: &str) -> Result<Output> {
    let output = Command::new("ffprobe")
        .arg(path)
        .arg("-show_streams")
        .arg("-show_format")
        .arg("-print_format")
        .arg("json")
        .output()?;
    Ok(output)
}

fn main() {
    let output = run_ffprobe("/home/klu/Music/Hanna - Intercession, On Behalf.flac");
    if let Ok(result) = output {
        println!("status: {}", result.status);
        io::stdout().write_all(&result.stdout).unwrap();
        io::stdout().write_all(&result.stderr).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        println!("song stream: {}", v["streams"][0]);
    }
}
