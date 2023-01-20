"""
Plan: 
Make a series using panda with the following data for each link: 
    - The actual link
    - size of page
    - num links 
    - num images 
    - size of each image on the page

Then using that data use matplotlib to create pretty representations for
    - size of pages
    - num links
    - num images
    - average size of images 
"""
import pandas

def parse_lines(lines):
    dataset = {
        'link': [],
        'size': [], 
        'numImages': [], 
    }

    for line in lines: 
        if line.strip() == "{": 
            # do nothing
            print("start")
        elif line.startswith('  "'): 
            end = line.rindex('"')
            #print(line[3:end])
            dataset["link"].append(line[3:end])
        elif line.startswith('    "size":'):
            end = line.rindex(',')
            #print("size: " + line[12:end])
            dataset["size"].append(int(line[12:end]))
        elif line.startswith('    "size":'):
            end = line.rindex(',')
            #print("size: " + line[12:end])

    print(dataset['size'])
def main():
    f = open("test.txt", "r")
    lines = f.readlines()
    parse_lines(lines)
    f.close()


if __name__ == "__main__":
    main()