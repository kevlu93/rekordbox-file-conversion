use serde::Deserialize;
use std::io::{self, Result, Write};
use std::process::{Command, Output};
use std::{cmp, fs, path};
use song_info::AudioFormatType;
use std::env;
mod song_info;

/// Function iterates through the directory and grabs file paths
pub fn build_list_of_files(dir: &path::Path, files: &mut Vec<String>) {
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
                        // Else add file string to list
                        if let Some(s) = path.to_str() {
                            files.push(String::from(s));
                        } else {
                            log::error!("Error converting {:?} to a string", path);
                        }
                    }
                } else {
                    log::error!("I/O error while reading directory entry: {:?}", entry)
                }
            }
        } else {
            log::error!("Error reading directory: {}", dir.display());
        }
    } else {
        log::error!("{} is not a directory!", dir.display());
    }
}

// TO-DO: Implement control flow so that volumedetect is used if volume normalization is desired
// Because volumedetect is a time-consuming process, user might not want to do it.
// Perhaps implement concurrency to speed up conversions
pub fn convert_song(files: Vec<String>, output_dir: &str, conversion_tag: &str) {
    let n_songs = files.len();
    let mut n_iterated = 0;
    for song in files.iter().filter_map(|s| song_info::from_file(s)) {
        match song.get_format_type() {
            AudioFormatType::Unsupported => {log::error!("{} has an unsupported file format!", song.get_song_path()); continue;},
            _ => {
                let song_name;
                match song.get_song_name() {
                    Some(s) => {song_name = s;},
                    None => {continue;},
                }
                // If a song satisfies Rekordbox audio format, we can skip
                if *song.get_sample_rate() <= 44100 && song.is_rekordbox_format() {
                    match song.get_format_type() {
                        AudioFormatType::Lossless => {if *song.get_bit_info() <= 16 {continue;}},
                        AudioFormatType::Lossy => {if *song.get_bit_info() <= 320 {continue;}},
                        _ => {continue;}, //can't occur since this code only gets evaluated if the format type is not unsupported
                    }
                }
                // If a song does not have the specified converion tag set to 1
                // move on to the next song
                match song.get_tags() {
                    Some(tags) => match tags.get(conversion_tag) {
                        // If conversion tag is not 1, skip
                        Some(tag) => {if tag != "1" {continue;}},
                        // If song does not have conversion tag, skip
                        None => continue,
                    },
                    // if song has no tags, skip
                    None => continue,
                }
                let output_format;
                let output_bit_info;
                let output_bit_type;
                let output_sample_rate = cmp::min(*song.get_sample_rate(), 44100);
                let output_codec;
                match song.get_format_type() {
                    AudioFormatType::Lossless => {
                        output_format = String::from("aiff");
                        output_bit_type = "-sample_fmt";
                        output_bit_info = format!("s{}", cmp::min(*song.get_bit_info(), 16));
                        output_codec = String::from("pcm_s16le");
                    },
                    AudioFormatType::Lossy => {
                        output_format = String::from("mp3");
                        output_bit_type = "-audio_bitrate";
                        output_bit_info = format!("{}", cmp::min(*song.get_bit_info(), 320000));
                        output_codec = String::from("mp3");
                    },
                    _ => {continue;}, //can't occur as this code block only gets evaluated if the audio format is not unsupported
                }
                let convert_output = Command::new("ffmpeg")
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
                    .arg("REKORDBOX_READY=1")
                    .arg("-metadata")
                    .arg("CONVERT_FOR_REKORDBOX=0")
                    .arg(output_bit_type)
                    .arg(output_bit_info)
                    .arg(format!("{}/{}.{}", output_dir, song_name, output_format))
                    .output();
                // If we ran into an error when converting the file, log it and then move on to the next file
                match convert_output {
                    Ok(o) => {
                        if !o.status.success() {
                            log::error!("Error with converting {}", song.get_song_path());
                            io::stderr().write_all(&o.stderr).unwrap();
                            continue;
                        }
                    },
                    Err(e) => {
                        log::error!("Error with converting {}: {}", song.get_song_path(), e);
                        continue;   
                    },
                }
                n_iterated = n_iterated + 1;
                if n_iterated % 10 == 0 {
                    println!("Gone through {} of {} songs", n_iterated, n_songs);
                }
            }
        }
    }
}


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

fn main() {
    env_logger::init();
    let mut args = env::args();
    //iterate past first argument, which is the program name
    args.next();
    //grab argument. if there is none, exit the program
    let folder;
    if let Some(a) = args.next() {
        folder = path::Path::new(a.as_str());
        if let Some(a) = args.next() {
            let tag = a;
            let mut songs = Vec::new();
            build_list_of_files(folder, &mut songs);
            convert_song(songs, "/home/kevlu93/Music/file_conversion_test_output/",tag.as_str());
        } else {
            println!("Please provide the tag you used for the songs you want to convert! (ie. CONVERT_FOR_REKORDBOX");
        } 
    } else {
        println!("Please provide a directory with your music!");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_volume() {
        assert_eq!(get_max_volume("/home/klu/Music/Hanna - Intercession, On Behalf.flac").unwrap(), -1.0);
        assert!(get_max_volume("dummy.mp3").is_none());
    }
}
