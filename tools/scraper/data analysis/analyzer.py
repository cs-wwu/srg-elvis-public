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
import matplotlib.pyplot as plt
import matplotlib
import numpy as np


def parse_link_data(link_data, dataset): 
    link_count = 0
    image_count = 0
    counting_links = False
    counting_images = False
    for line in link_data: 
        if counting_links: 
            if line.strip() == '],':
                dataset['num_links'].append(link_count)
                counting_links = False
            else:
                link_count += 1
        elif counting_images:
            if line.strip() == ']':
                dataset['num_images'].append(image_count)
                counting_images = False
            else:
                image_count += 1
        else: 
            if line.startswith('  "'): 
                end = line.rindex('"')
                dataset["link"].append(line[3:end])
            elif line.startswith('    "size":'): 
                end = line.rindex(',')
                dataset["size"].append((int(line[12:end])/1000)) #save size in KB
            elif line.strip() == '"links": [':
                counting_links = True
            elif line.strip() == '"images": [':
                counting_images = True


def parse_lines(lines):
    dataset = {
        'link': [],
        'size': [], 
        'num_links': [],
        'num_images': []
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

def main():
    f = open("test.txt", "r")
    lines = f.readlines()
    dataset = parse_lines(lines)
    """
    print(dataset['link'])
    print(dataset['size'])
    print(dataset['num_links'])
    print(dataset['num_images'])
    """
    dataframe = pandas.DataFrame(dataset)
    print(dataframe)
    #dataframe.plot()
    #plt.hist(dataframe.to_numpy())
    #dataframe.hist(column='size')
    #plt.title("Number of links")
    
    #figsize(7, 5)
 
    plt.hist(dataframe['size'], color='blue', edgecolor='black')
    plt.xlabel('Page Size (KB)')
    plt.ylabel('No. of Pages')
    plt.title('Page Size')
    plt.savefig('page_size.pdf')
    plt.close()

    plt.hist(dataframe['num_links'], color='blue', edgecolor='black')
    plt.xlabel('No. of Links on Page')
    plt.ylabel('No. of Pages')
    plt.title('No. of Links')
    plt.savefig('num_links.pdf')
    plt.close()

    plt.hist(dataframe['num_images'], color='blue', edgecolor='black')
    plt.xlabel('No. of Images on Page')
    plt.ylabel('No. of Pages')
    plt.title('No. of Images')
    plt.savefig('num_images.pdf')
    plt.close()

 

    f.close()


if __name__ == "__main__":
    main()