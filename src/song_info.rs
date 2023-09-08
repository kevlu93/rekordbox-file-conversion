use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

/// enum for various audio formats
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AudioFormatType {
    Lossless(SupportedAudioFormat),
    Lossy(SupportedAudioFormat),
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SupportedAudioFormat {
    AIFF,
    FLAC,
    WAV,
    MP3,
    OGG,
    AAC,
}

impl FromStr for SupportedAudioFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "aiff" => Ok(SupportedAudioFormat::AIFF),
            "flac" => Ok(SupportedAudioFormat::FLAC),
            "wav" => Ok(SupportedAudioFormat::WAV),
            "mp3" => Ok(SupportedAudioFormat::MP3),
            "ogg" => Ok(SupportedAudioFormat::OGG),
            "aac" => Ok(SupportedAudioFormat::AAC),
            _ => Err(anyhow!("Not a supported file format")),
        }
    }
}

impl std::fmt::Display for SupportedAudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = format!("{:?}", self);
        write!(f, "{}", s.to_lowercase())
    }
}

impl From<SupportedAudioFormat> for AudioFormatType {
    fn from(value: SupportedAudioFormat) -> Self {
        match &value {
            SupportedAudioFormat::AIFF | SupportedAudioFormat::FLAC | SupportedAudioFormat::WAV => {
                AudioFormatType::Lossless(value)
            }
            SupportedAudioFormat::MP3 | SupportedAudioFormat::OGG | SupportedAudioFormat::AAC => {
                AudioFormatType::Lossy(value)
            }
        }
    }
}

impl FromStr for AudioFormatType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(format) = s
            .trim()
            .to_lowercase()
            .as_str()
            .parse::<SupportedAudioFormat>()
        {
            Ok(format.into())
        } else {
            Ok(AudioFormatType::Unsupported)
        }
    }
}
/// Song struct that contains information to use for ffmpeg conversion command
#[derive(Clone, Debug)]
pub struct SongInfo {
    codec: String,
    format: AudioFormatType,
    song_path: PathBuf,
    sample_rate: usize,
    bit_info: usize,
    tags: Option<serde_json::Value>,
}

/// Helper struct that represents initial read from ffprobe
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Probe {
    streams: Option<Vec<ProbeStream>>,
    format: Option<ProbeFormat>,
}

/// Helper struct that represents a stream from ffprobe
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProbeStream {
    codec_name: String,
    codec_type: String,
    #[serde(default)]
    #[serde(deserialize_with = "from_string")]
    sample_rate: Option<usize>,
    #[serde(default)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProbeFormat {
    format_name: AudioFormatType,
    #[serde(default)]
    tags: Option<serde_json::Value>,
}

/// Executes the ffprobe command to get the stream and format info.
#[tracing::instrument(level = "info", ret)]
fn run_ffprobe(path: &Path) -> Result<Probe> {
    // Run ffprobe
    let output = Command::new("ffprobe")
        .arg(path.to_path_buf())
        .arg("-show_streams")
        .arg("-show_format")
        .arg("-print_format")
        .arg("json")
        .output()?;
    // Store the results as a struct
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Initializes a Song struct
pub fn from_file(path: &Path) -> Result<SongInfo> {
    let probe_result = run_ffprobe(path)?;
    match (probe_result.streams, probe_result.format) {
        (Some(s), Some(f)) => {
            // splitting the path will return the full file name
            // then extract the name before the period
            // since this part of code only runs if a valid path was found
            // unwraps are guaranteed to work, so this will not panic

            // based on the format type, bit info will either be the sample_fmt, or bit_rate
            let bit_info = match f.format_name {
                AudioFormatType::Lossless(_) => s[0].sample_fmt.unwrap_or(0),
                AudioFormatType::Lossy(_) => s[0].bit_rate.unwrap_or(0),
                _ => 0,
            };
            Ok(SongInfo {
                codec: s[0].codec_name.clone(),
                format: f.format_name,
                song_path: path.to_path_buf(),
                sample_rate: s[0].sample_rate.unwrap_or(0),
                bit_info,
                tags: f.tags,
            })
        }
        _ => Err(anyhow!("Missing streams or format for {:?}", path)),
    }
}

impl SongInfo {
    pub fn get_codec(&self) -> &str {
        self.codec.as_str()
    }

    pub fn get_format(&self) -> &AudioFormatType {
        &self.format
    }

    pub fn get_song_path(&self) -> &PathBuf {
        &self.song_path
    }

    pub fn get_song_name(&self) -> Result<String> {
        if self.song_path.is_file() {
            Ok(self
                .song_path
                .file_name()
                .unwrap()
                .to_str()
                .context(format!(
                    "Song path is not valid UTF-8: {:?}",
                    self.song_path
                ))?
                .to_string())
        } else {
            Err(anyhow!("Song path is not a file: {:?}", self.song_path))
        }
    }

    pub fn get_sample_rate(&self) -> &usize {
        &self.sample_rate
    }

    pub fn get_bit_info(&self) -> &usize {
        &self.bit_info
    }

    pub fn get_tags(&self) -> &Option<serde_json::Value> {
        &self.tags
    }

    pub fn is_rekordbox_format(&self) -> bool {
        match &self.format {
            AudioFormatType::Lossless(format) | AudioFormatType::Lossy(format) => match format {
                SupportedAudioFormat::AIFF
                | SupportedAudioFormat::WAV
                | SupportedAudioFormat::MP3
                | SupportedAudioFormat::AAC => true,
                _ => false,
            },
            _ => false,
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffprobe_file_exists() {
        let mut probe =
            run_ffprobe(&Path::new("/home/klu/Music/soy division mix.mp3").to_path_buf()).unwrap();
        let mut format = probe.format.unwrap();
        let mut streams = probe.streams.unwrap();
        assert_eq!("mp3", format.format);
        assert_eq!(None, streams[0].sample_fmt);

        probe = run_ffprobe(
            &Path::new("/home/klu/Music/Hanna - Intercession, On Behalf.flac").to_path_buf(),
        )
        .unwrap();
        format = probe.format.unwrap();
        streams = probe.streams.unwrap();
        assert_eq!("flac", format.format);
        assert_eq!(None, streams[0].bit_rate);
        assert_eq!(32, streams[0].sample_fmt.unwrap());
        assert_eq!("House", format.tags.unwrap().get("OVERALL GENRE").unwrap());
    }

    #[test]
    fn test_ffprobe_file_doesnt_exist() {
        let probe = run_ffprobe(Path::new("fake.mp3")).unwrap();
        assert!(probe.format.is_none());
        assert!(probe.streams.is_none());
    }

    #[test]
    fn test_create_songinfo_from_file() {
        let info = from_file(Path::new("/home/klu/Music/soy division mix.mp3")).unwrap();
        assert_eq!(None, info.tags);
        assert_eq!(
            "/home/klu/Music/soy division mix.mp3",
            info.song_path.to_str().unwrap()
        );
        assert_eq!(320000, info.bit_info);
        assert!(info.is_rekordbox_format());
        assert_eq!(44100, info.sample_rate);

        assert!(from_file(Path::new("missing.mp3")).is_ok());
    }

    #[test]
    fn test_get_song_name_from_file() {
        let info = from_file(Path::new("/home/klu/Music/soy division mix.mp3")).unwrap();
        assert_eq!("soy division mix", info.get_song_name().unwrap());

        assert!(from_file(Path::new("dummy"))
            .unwrap()
            .get_song_name()
            .is_ok());
    }
}
*/
