use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::io;
use std::process::Command;

/// enum for various audio formats
pub enum AudioFormatType {
    Lossless,
    Lossy,
    Unsupported,
}

/// Song struct that contains information to use for ffmpeg conversion command
pub struct SongInfo {
    codec: String,
    format: String,
    format_type: AudioFormatType,
    song_name: String,
    sample_rate: usize,
    bit_info: usize,
    tags: Option<serde_json::Value>,
}

/// Helper struct that represents initial read from ffprobe
#[derive(Deserialize, Serialize)]
struct Probe {
    streams: Option<Vec<ProbeStream>>,
    format: Option<ProbeFormat>,
}

/// Helper struct that represents a stream from ffprobe
#[derive(Deserialize, Serialize)]
struct ProbeStream {
    codec_name: String,
    #[serde(deserialize_with = "from_string")]
    sample_rate: Option<usize>,
    #[serde(deserialize_with = "from_string")]
    sample_fmt: Option<usize>,
    // bit_rate field only exists for lossy such as mp3.
    #[serde(default)]
    #[serde(deserialize_with = "from_string")]
    bit_rate: Option<usize>,
}

/// Helper function to help Serde deserialize values that we want to be numeric,
/// but coded as a string by ffprobe
fn from_string<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s: String = Deserialize::deserialize(deserializer)?;
    s = s.replace("s", "");
    // See if we can parse the sample_fmt to get the bit depth. If not return 0.
    Ok(s.parse::<usize>().ok())
}

/// Helper struct that represents a format from ffprobe
#[derive(Debug, Deserialize, Serialize)]
struct ProbeFormat {
    format_name: String,
    tags: Option<serde_json::Value>,
}

/// Executes the ffprobe command to get the stream and format info.
fn run_ffprobe(path: &str) -> io::Result<Probe> {
    // Run ffprobe
    let output = Command::new("ffprobe")
        .arg(path)
        .arg("-show_streams")
        .arg("-show_format")
        .arg("-print_format")
        .arg("json")
        .output()?;
    // Store the results as a struct
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Initializes a Song struct
pub fn from_file(path: &str) -> Option<SongInfo> {
    let probe_result = run_ffprobe(path);
    if let Ok(p) = probe_result {
        match (p.streams, p.format) {
            (Some(s), Some(f)) => {
                let song_format = f.format_name;
                let format_type = match song_format.as_str() {
                    "aiff" | "flac" | "wav" => AudioFormatType::Lossless,
                    "mp3" | "ogg" | "aac" => AudioFormatType::Lossy,
                    _ => AudioFormatType::Unsupported,
                };
                // splitting the path will return the full file name
                // then extract the name before the period
                // since this part of code only runs if a valid path was found
                // unwraps are guaranteed to work, so this will not panic
                let song_name =
                    String::from(path.split('/').last().unwrap().split('.').next().unwrap());
                // based on the format type, bit info will either be the sample_fmt, or bit_rate
                let bit_info = match format_type {
                    AudioFormatType::Lossless => s[0].sample_fmt.unwrap_or(0),
                    AudioFormatType::Lossy => s[0].bit_rate.unwrap_or(0),
                    _ => 0,
                };
                Some(SongInfo {
                    codec: s[0].codec_name.clone(),
                    format: song_format,
                    format_type,
                    song_name,
                    sample_rate: s[0].sample_rate.unwrap_or(0),
                    bit_info,
                    tags: f.tags,
                })
            }
            _ => {
                log::error!("Missing streams or format for {}", path);
                None
            }
        }
    } else {
        log::error!("ffprobe could not handle {} correctly!", path);
        None
    }
}

impl SongInfo {
    pub fn get_format_type(&self) -> &AudioFormatType {
        &self.format_type
    }

    pub fn get_song_name(&self) -> &str {
        self.song_name.as_str()
    }

    pub fn is_rekordbox_format(&self) -> bool {
        match self.format.as_str() {
            "aiff" | "wav" | "mp3" | "aac" => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffprobe_file_exists() {
        let mut probe = run_ffprobe("/home/klu/Music/soy division mix.mp3").unwrap();
        let mut format = probe.format.unwrap();
        let mut streams = probe.streams.unwrap();
        assert_eq!("mp3", format.format_name);
        assert_eq!(None, streams[0].sample_fmt);

        probe = run_ffprobe("/home/klu/Music/Hanna - Intercession, On Behalf.flac").unwrap();
        format = probe.format.unwrap();
        streams = probe.streams.unwrap();
        assert_eq!("flac", format.format_name);
        assert_eq!(None, streams[0].bit_rate);
        assert_eq!(32, streams[0].sample_fmt.unwrap());
        assert_eq!("House", format.tags.unwrap().get("OVERALL GENRE").unwrap());
    }

    #[test]
    fn test_ffprobe_file_doesnt_exist() {
        let probe = run_ffprobe("fake.mp3").unwrap();
        assert!(probe.format.is_none());
        assert!(probe.streams.is_none());
    }

    #[test]
    fn test_create_songinfo_from_file() {
        let info = from_file("/home/klu/Music/soy division mix.mp3").unwrap();
        assert_eq!(None, info.tags);
        assert_eq!("soy division mix", info.song_name);
        assert_eq!(320000, info.bit_info);

        assert!(from_file("missing.mp3").is_none());
    }
}
