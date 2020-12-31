import unittest
from song import *
import converter

class Song_Test(unittest.TestCase):

    @classmethod
    def setUpClass(cls):
        cls.file_paths = converter.getListOfFiles('/home/kevlu93/Downloads')
        cls.test_file = [x for x in cls.file_paths[0] if x.find('Rufus') >= 0][0]
        cls.test_song = Song(cls.test_file)

    #test get sample rate method
    def test_sample_rate(self):
        self.assertEqual(self.test_song.get_sample_rate(), 44100)

    #test the get_bit_rate method
    def test_bit_rate(self):
        self.assertEqual(self.test_song.get_bit_rate(), 24)

    def test_format(self):
        self.assertEqual(self.test_song.get_format(), 'flac')

    def test_codec(self):
        self.assertEqual(self.test_song.get_codec(), 'flac')

    def test_has_tag(self):
        self.assertTrue(self.test_song.has_tag('VOCALS'))
        self.assertFalse(self.test_song.has_tag('fake tag'))

if __name__ == 'main':
    unittest.main()