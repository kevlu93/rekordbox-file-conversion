import converter 
import song 
import argparse
import os

def main():
    #get arguments from command line input
    parser = argparse.ArgumentParser(
        description="""This script converts music files into a Pioneer CDJ friendly format"""
    )
    parser.add_argument('input_path', help = "The file or folder that you want to convert")
    parser.add_argument('output_path', help = "The folder you want to put the converted files in")
    parser.add_argument('-conversion_tag', help = "The name of the tag used to flag whether you want the file to be converted for Rekordbox or not. If blank, then converts all files")
    args = parser.parse_args()
    MUSIC_DIR = args.input_path
    CONVERTED_DIR = args.output_path
    CONVERSION_TAG = args.conversion_tag

    files = converter.getListOfFiles(MUSIC_DIR)
    if not os.path.isdir(CONVERTED_DIR):
        os.mkdir(CONVERTED_DIR)
    for file in files:
        full_path = file[0]
        file_name = file[1]
        cur_song = song.Song(full_path, file_name)
        #run conversion only for files with the specified conversion flag. if no conversion flag is given, convert all files in the directory
        if CONVERSION_TAG is not None:
            if cur_song.has_tag(CONVERSION_TAG):
                if cur_song.get_tags()[CONVERSION_TAG] == '1':
                    converter.convert_song(cur_song, CONVERTED_DIR)
        else:
            converter.convert_song(cur_song, CONVERTED_DIR)

if __name__ == "__main__":
    main()