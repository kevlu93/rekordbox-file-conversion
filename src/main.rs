use serde::Deserialize;
use std::io::{self, Result, Write};
use std::process::{Command, Output};
use std::{fs, path, cmp};
mod song_info;

/// Function iterates through the directory and grabs file paths
fn build_list_of_files(dir: &path::Path, files: &mut Vec<String>) {
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

// TO-DO join options together to create ffmpeg output command 
fn convert_song(files: Vec<String>) {
    for song in files.iter().filter_map(|s| song_info::from_file(s.as_str())) {
        match song.get_format_type() {
            Unsupported => {log::error!("{} has an unsupported file format!", song.get_song_name()); continue;},
            _ => {
                // If a song satisfies Rekordbox audio format, we can skip
                if song.get_sample_rate() <= 44100 && input_max_vol == PEAK_DB && song.is_rekordbox_format() {
                    match song.get_format_type() {
                        Lossless => {if song.get_bit_info() <= 16 {continue;}},
                        Lossy => {if song.get_bit_info() <= 320 {continue;}},
                    }
                }
                let output_format;
                let output_bit_info;
                let output_sample_rate = cmp::min(song.get_sample_rate(), 44100);
                let output_codec;
                match song.get_format_type() {
                    Lossless => {
                        output_format = String::from("aiff");
                        output_bit_info = cmp::min(song.get_bit_info(), 16);
                        output_codec = String::from("pcm_s16le");
                    },
                    Lossy => {
                        output_format = String::from("mp3");
                        output_bit_info = cmp::min(song.get_bit_info(), 320000);
                        output_codec = String::from("mp3");
                    }
                }
            }
        }
    }
}

fn get_song_volume(path: &str) {
    // TO-DO parse volume detect to get max volume info
}

fn main() {
    env_logger::init();
    let mut files: Vec<String> = vec![];
    build_list_of_files(path::Path::new("/home/klu/Musi/"), &mut files);
    println!("list of files: {:?}", files);
    ()
}


