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

def parse_link_data(link_data, dataset): 
    link_count = 0
    image_count = 0
    counting_links = False
    counting_images = False
    for line in link_data: 
        if counting_links: 
            if line.strip() == '],':
                dataset['numLinks'].append(link_count)
                counting_links = False
            else:
                link_count += 1
        elif counting_images:
            if line.strip() == ']':
                dataset['numImages'].append(image_count)
                counting_images = False
            else:
                image_count += 1
        else: 
            if line.startswith('  "'): 
                end = line.rindex('"')
                dataset["link"].append(line[3:end])
            elif line.startswith('    "size":'): 
                end = line.rindex(',')
                dataset["size"].append(int(line[12:end]))
            elif line.strip() == '"links": [':
                counting_links = True
            elif line.strip() == '"images": [':
                counting_images = True


def parse_lines(lines):
    dataset = {
        'link': [],
        'size': [], 
        'numLinks': [],
        'numImages': []
    }
    link_data = []
    for line in lines: 
        if line.strip() == '{' or line.strip() == '}': 
            continue
        elif line.strip() == '},': 
            parse_link_data(link_data, dataset)
            link_data = []
        else:
            link_data.append(line)
    return dataset

def old_parse_lines(lines):
    dataset = {
        'link': [],
        'size': [], 
        'numLinks': [],
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
    dataset = parse_lines(lines)
    print(dataset['link'])
    print(dataset['size'])
    print(dataset['numLinks'])
    print(dataset['numImages'])

    f.close()


if __name__ == "__main__":
    main()