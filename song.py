import ffmpeg
import subprocess
import re

class Song:

    #constructor
    def __init__(self, path):
        #instance variables
        #input stream of the file
        self.input_stream = ffmpeg.input(path)
        #ffprobe info
        self.stream_info = ffmpeg.probe(path)['streams'][0]
        self.format_info = ffmpeg.probe(path)['format']
        #codec info
        self.codec = self.stream_info['codec_name']
        #song quality info
        self.sample_rate = int(self.stream_info['sample_rate'])
        self.bit_rate = int(self.stream_info['bits_per_raw_sample'])
        self.format = self.format_info['format_name']
        #tags as a dictionary
        self.tags = self.format_info['tags']
        #initialize volume info
        self.volume = {}
        self.get_volume_info()

    #create getters for the song characteristics
    def get_stream(self):
        return self.input_stream

    def get_stream_info(self):
        return self.stream_info

    def get_format_info(self):
        return self.format_info

    def get_codec(self):
        return self.codec

    def get_sample_rate(self):
        return self.sample_rate

    def get_bit_rate(self):
        return self.bit_rate

    def get_format(self):
        return self.format

    def get_tags(self):
        return self.tags

    def get_volume_info_output(self):
        return self.volume

    '''checks if song has a given tag'''
    def has_tag(self, tag_name):
        return tag_name in self.tags.keys()

    '''method that finds the peak volume of a file'''
    def get_volume_info(self):
        process = subprocess.Popen(
            self.input_stream
                .filter('volumedetect')
                .output('-', format = 'null')
                .compile()
            , stderr = subprocess.PIPE
            , encoding = 'utf8'
        )

        cmd_output = process.stderr.readlines()
        for line in cmd_output:
            if re.search('(mean|max)_volume', line):
                print('found volume line')
                db = float(line.split(':')[1].replace('dB', '').strip())
                if re.search('mean', line):
                    self.volume['mean'] = db
                if re.search('max', line):
                    self.volume['max'] = db
 
    '''get max volume'''
    def get_max_volume(self):
        return self.volume['max']

    '''get mean volume'''
    def get_mean_volume(self):
        return self.volume['mean']
