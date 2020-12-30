import os
import re
import ffmpeg

'''Given a file path, the function pulls relevant audio file paths to consider
   returns a list of the file paths as well as the actual name of the file 
'''
def getListOfFiles(dirName):
    # create a list of file and sub directories 
    # names in the given directory 
    valid_file_ext = "\.(aif|aiff|flac|mp3|wav)$" #grabs files with an audio file extension
    listOfFile = os.listdir(dirName)
    allFiles = list()
    fileNames = list()
    # Iterate over all the entries
    for entry in listOfFile:
        # Create full path
        fullPath = os.path.join(dirName, entry)
        # If entry is a directory then get the list of files in this directory 
        if os.path.isdir(fullPath):
            allFiles = allFiles + getListOfFiles(fullPath)[0]
            fileNames = fileNames + getListOfFiles(fullPath)[1]
            
        else:
            #if file has a valid extension, add it to the list of files
            if re.search(valid_file_ext, fullPath) and entry.find("._") != 0 and entry != "Converted for Rekordbox":
                allFiles.append(fullPath)
                fileNames.append(entry)
    return allFiles, fileNames   

def main():
    files = getListOfFiles("/home/kevlu93/Downloads/Bjork")
    print(files[1])
    input_stream = ffmpeg.probe(files[0][1])
    print(input_stream)

if __name__ == "__main__":
    main()