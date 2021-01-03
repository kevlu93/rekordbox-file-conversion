import os
import re
import ffmpeg

'''Given a file path, the function pulls relevant audio file paths to consider
   returns a list of the file paths as well as the actual name of the file 
'''
LOSSLESS_FORMATS = ['aiff', 'flac', 'wav']
LOSSY_FORMATS = ['mp3', 'ogg', 'aac']
PEAK_DB = -0.5
SUPPORTED_FORMATS = LOSSLESS_FORMATS + LOSSY_FORMATS

def getListOfFiles(dirName):
    # create a list of file and sub directories 
    # names in the given directory 
    formats_str = "|".join(SUPPORTED_FORMATS) + "|aif"
    valid_file_ext = "\.({})$".format(formats_str) #grabs files with an audio file extension
    files = list()
    if not os.path.isdir(dirName):
        files.append((dirName, dirName.split('/')[-1]))
    else:
        listOfFile = os.listdir(dirName)
        # Iterate over all the entries
        for entry in listOfFile:
            # Create full path
            fullPath = os.path.join(dirName, entry)
            # If entry is a directory and directory isnt the folder with converted files, then get the list of files in this directory 
            if os.path.isdir(fullPath) and entry != "Converted for Rekordbox":
                files = files + getListOfFiles(fullPath)
            else:
                #if file has a valid extension, add it to the list of files
                if re.search(valid_file_ext, fullPath) and entry.find("._") != 0:
                    files.append((fullPath, entry))
    return files

'''converts a song'''
def convert_song(song, output_dir):
    input_format = song.get_format()
    input_bit_info = song.get_bit_depth() if input_format in LOSSLESS_FORMATS else song.get_bit_rate()
    input_sample_rate = song.get_sample_rate()
    input_max_volume = song.get_max_volume()
    #only convert if a file is not CDJ compatible or volume is not at the specified peak db
    if not (input_sample_rate <= 44100 and input_max_volume == PEAK_DB and ((input_format in ['aiff', 'wav'] and input_bit_info <= 16) or 
            (input_format in ['mp3', 'aac'] and input_bit_info <= 320))): 
        #set up output parameters
        volume_offset = PEAK_DB - input_max_volume 
        if input_format in LOSSLESS_FORMATS: 
            output_format = 'aiff'
            output_bit_depth = min(input_bit_info, 16)
            output_sample_rate = min(input_sample_rate, 44100)
            output_codec = 'pcm_s16le'

        if input_format in LOSSY_FORMATS:
            output_format = 'mp3'
            output_bit_rate = min(input_bit_info,320000) 
            output_sample_rate = min(input_sample_rate, 44100)
            output_codec = 'mp3'
        #if song needs to be normalized to specified peak dB, do so
        if volume_offset == 0:
            input_stream = song.get_stream()
        else:
            input_stream = ffmpeg.filter(song.get_stream(), 'volume', volume = '{}dB'.format(volume_offset))
        #set up ffmpeg output options
        output_options = {}
        output_options['acodec'] = output_codec
        output_options['f'] = output_format
        output_options['ar'] = str(output_sample_rate)
        output_options['write_id3v2'] = 1
        output_options['metadata'] = ['REKORDBOX_READY=1', 'CONVERT_FOR_REKORDBOX=0']
        if output_format == 'aiff':
            output_options['sample_fmt'] = 's{}'.format(output_bit_depth)
        else: 
            output_options['audio_bitrate'] = output_bit_rate
        #convert the song to the provided directory
        print("Converting '{}'".format(song.get_song_name()))
        output_song = (
            ffmpeg
            .output(
                input_stream
                , '{OUTPUT_DIRECTORY}/{SONG_NAME}.{OUTPUT_FORMAT}'.format(OUTPUT_DIRECTORY = output_dir, SONG_NAME = song.get_song_name(), OUTPUT_FORMAT = output_format)
                , **output_options)
            .overwrite_output()
            .run(quiet = True)
        )
    else:
        print("'{}' does not need to be converted.".format(song.get_song_name()))
