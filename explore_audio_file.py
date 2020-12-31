import ffmpeg
import converter 
import os
from song import *
import subprocess 

files = converter.getListOfFiles('/home/kevlu93/Downloads')
song = [x for x in files[0] if x.find('Rufus') >= 0][0]

test_song = Song(song)
print("bit rate:" + str(test_song.get_bit_rate()))
print("codec:" + test_song.get_codec())

print(test_song.get_max_volume())
