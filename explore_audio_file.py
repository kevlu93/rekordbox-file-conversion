import ffmpeg
import converter 
import os

files = converter.getListOfFiles('/home/kevlu93/Downloads')
song = [x for x in files[0] if x.find('Rufus') >= 0][0]
song_input = ffmpeg.input(song)
print(ffmpeg.probe(song))
ffmpeg.output(song_input, "test_meta.txt", f = "ffmetadata").run(overwrite_output=True)

