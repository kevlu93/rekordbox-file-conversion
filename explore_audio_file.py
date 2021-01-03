import ffmpeg
import converter 
import os
import subprocess 
from song import *

files = converter.getListOfFiles('test_song.flac')
print(files)
#song_file = [x for x in files if x[0].find('Big Time Sensuality') >= 0]
test_song = Song(files[0][0], files[0][1])

#converter.convert_song(test_song, os.getcwd()) 

#print(Song('Rufus and Chaka - Body Heat.aiff', 'Rufus and Chaka - Body Heat.aiff').get_format_info())