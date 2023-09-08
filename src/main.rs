use anyhow::{anyhow, Result};
use clap::Parser;
use song_info::{AudioFormatType, SupportedAudioFormat};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::{
    cmp, fs,
    path::{Path, PathBuf},
};
mod song_info;
use song_info::SongInfo;

/// This app converts all tagged songs in a directory into a Rekordbox friendly format
#[derive(Parser)]
#[command(about, long_about = None)]
struct App {
    /// The folder with the songs you want to convert
    #[arg(short, long)]
    input_dir: String,
    /// Output directory to store converted songs
    #[arg(short, long)]
    output_dir: String,
    /// Tag to search for when looking for songs in the directory to convert. If not given then
    /// convert all songs in the input directory
    #[arg(short, long)]
    rekordbox_tag: Option<String>,
}

/// Function iterates through the directory and grabs file paths
pub fn build_list_of_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            // Iterate through entries in the directory
            for entry in entries {
                if let Ok(e) = entry {
                    let path = e.path();
                    // If entry is a directory, recursively search through it
                    if path.is_dir() {
                        build_list_of_files(path.as_path(), files);
                    } else {
                        files.push(path);
                    }
                } else {
                    tracing::error!("I/O error while reading directory entry: {:?}", entry)
                }
            }
        } else {
            tracing::error!("Error reading directory: {}", dir.display());
        }
    } else {
        tracing::error!("{} is not a directory!", dir.display());
        std::process::exit(1);
    }
}

// TO-DO: Implement control flow so that volumedetect is used if volume normalization is desired
// Because volumedetect is a time-consuming process, user might not want to do it.
// Perhaps implement concurrency to speed up conversions
pub fn convert_song(song: &SongInfo, output_dir: &Path, conversion_tag: &str) -> Result<()> {
    match song.get_format() {
        AudioFormatType::Unsupported => {
            return Err(anyhow!(
                "{} has an unsupported file format!",
                song.get_song_path().to_string_lossy()
            ))
        }
        _ => {
            let song_name = song.get_song_name()?;
            // If a song satisfies Rekordbox audio format, we can skip
            if *song.get_sample_rate() <= 44100 && song.is_rekordbox_format() {
                match song.get_format() {
                    AudioFormatType::Lossless(_) => {
                        if *song.get_bit_info() <= 16 {
                            tracing::warn!(?song_name, "Already Rekordbox format!");
                            return Ok(());
                        }
                    }
                    AudioFormatType::Lossy(_) => {
                        if *song.get_bit_info() <= 320000 {
                            tracing::warn!(?song_name, "Already Rekordbox format!");
                            return Ok(());
                        }
                    }
                    _ => (), //can't occur since this code only gets evaluated if the format type is not unsupported
                }
            }
            // If we are given a conversion tag, if a song does not have the specified conversion tag set to 1
            // move on to the next song
            let conversion_tag_arg;
            if conversion_tag.len() > 0 {
                match song.get_tags() {
                    Some(tags) => match tags.get(conversion_tag) {
                        // If conversion tag is not 1, skip
                        Some(tag) => {
                            if tag != "1" {
                                return Err(anyhow!("Not tagged for conversion! {:?}", song_name));
                            }
                        }
                        // If song does not have conversion tag, skip
                        None => return Err(anyhow!("Not tagged for conversion! {:?}", song_name)),
                    },
                    // if song has no tags, skip
                    None => return Err(anyhow!("Not tagged for conversion! {:?}", song_name)),
                }
                conversion_tag_arg = format!("{}=0", conversion_tag);
            } else {
                conversion_tag_arg = conversion_tag.to_string();
            }

            let output_format;
            let output_bit_info;
            let output_bit_type;
            let output_sample_rate = cmp::min(*song.get_sample_rate(), 44100);
            let output_codec;
            match song.get_format() {
                AudioFormatType::Lossless(_) => {
                    output_format = SupportedAudioFormat::AIFF.to_string();
                    output_bit_type = "-sample_fmt";
                    output_bit_info = format!("s{}", cmp::min(*song.get_bit_info(), 16));
                    output_codec = String::from("pcm_s16le");
                }
                AudioFormatType::Lossy(_) => {
                    output_format = SupportedAudioFormat::MP3.to_string();
                    output_bit_type = "-b:a";
                    output_bit_info = format!("{}k", cmp::min(*song.get_bit_info(), 320000) / 100);
                    output_codec = String::from("mp3");
                }
                _ => return Ok(()), //can't occur as this code block only gets evaluated if the audio format is supported
            }
            let mut output_file_path = output_dir.to_path_buf();
            output_file_path.push(format!("{}.{}", song_name, output_format));

            let mut convert_command = Command::new("ffmpeg");
            convert_command
                .arg("-y")
                .arg("-i")
                .arg(song.get_song_path())
                .arg("-acodec")
                .arg(output_codec)
                .arg("-ar")
                .arg(format!("{}", output_sample_rate))
                .arg("-write_id3v2")
                .arg("1")
                .arg("-metadata")
                .arg("REKORDBOX=1");

            if conversion_tag.len() > 0 {
                convert_command.arg("-metadata").arg(conversion_tag_arg);
            }
            convert_command
                .arg(output_bit_type)
                .arg(output_bit_info)
                .arg(output_file_path);
            // If we ran into an error when converting the file, log it and then move on to the next file
            convert_command.output()?;
            Ok(())
        }
    }
}

pub fn convert_songs_parallel(songs: &Vec<PathBuf>, output_path: &str, tag: &str) -> Result<()> {
    let mut handles: Vec<JoinHandle<Result<()>>> = vec![];
    let n_converted = Arc::new(Mutex::new(0));
    let n_iterated = Arc::new(Mutex::new(0));
    for song in songs
        .iter()
        .filter_map(|s| song_info::from_file(s.as_path()).ok())
    {
        let n_converted_lock = n_converted.clone();
        let n_iterated_lock = n_iterated.clone();
        let output_path_copy = output_path.to_string();
        let tag_copy = tag.to_string();
        let handle = thread::spawn(move || {
            {
                let mut i = n_iterated_lock.lock().unwrap();
                *i += 1;
                tracing::debug!(n_songs = *i, "Current number of songs iterated through");
            }
            {
                let out_path = Path::new(&output_path_copy);
                if let Err(e) = convert_song(&song, out_path, &tag_copy) {
                    tracing::error!(?e);
                } else {
                    let mut c = n_converted_lock.lock().unwrap();
                    *c += 1;
                    tracing::debug!(n_converted = *c, "Current number of converted songs");
                }
            }
            Ok(())
        });
        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.join().unwrap();
    }
    let n_converted = Arc::try_unwrap(n_converted)
        .expect("Should not have more than reference to n_converted")
        .into_inner()
        .unwrap();
    let n_iterated = Arc::try_unwrap(n_iterated)
        .expect("Should not have more than reference to n_converted")
        .into_inner()
        .unwrap();
    tracing::info!(?n_converted, ?n_iterated, "Results of conversion");
    Ok(())
}

/**
// Helper function to find the peak RMS of an audio file
fn get_max_volume(path: &str) -> Option<f64> {
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(path)
        .arg("-filter:a")
        .arg("volumedetect")
        .arg("-f")
        .arg("null")
        .arg("dummy.mp3") //dummy output that ffmpeg requires
        .output()
        .expect("failed to get volume");
    let mut max_volume = None;
    let vol_output = String::from_utf8(output.stderr);
    if let Ok(vol_output) = vol_output {
        // Find the line with max volume
        let line: String = vol_output
            .lines()
            .filter(|s| s.ends_with("dB"))
            .filter(|s| s.contains("max_volume"))
            .collect();
        // Parse the max volume line to find the level
        let mut parsed_num: Vec<f64> = line
            .split(' ')
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if parsed_num.len() != 1 {
            log::error!("Volume for {} not parsed correctly!", path);
        } else {
            max_volume = parsed_num.pop();
        }
    } else {
        log::error!("Could not parse output from volumedetect for {}", path);
    }
    max_volume
}
*/

fn main() {
    //Initialize tracing
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let app = App::parse();

    let in_folder = Path::new(app.input_dir.as_str());
    let out_path = Path::new(app.output_dir.as_str());
    if !out_path.is_dir() {
        tracing::error!("Provided output path is not a directory!");
        std::process::exit(1);
    }
    let mut songs = Vec::new();
    build_list_of_files(in_folder, &mut songs);
    //okay to unwrap here because out was converted from a str originally
    let _ = convert_songs_parallel(
        &songs,
        &app.output_dir,
        app.rekordbox_tag.unwrap_or_default().as_str(),
    )
    .unwrap();
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_volume() {
        assert_eq!(get_max_volume("/home/klu/Music/Hanna - Intercession, On Behalf.flac").unwrap(), -1.0);
        assert!(get_max_volume("dummy.mp3").is_none());
    }
}
*/
