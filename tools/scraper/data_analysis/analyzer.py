import pandas as pd
import matplotlib.pyplot as plt
from urllib import request
#import matplotlib
import numpy as np

def get_img_size(img_link):
    # get file size *and* image size (None if not known)
    # print(img_link)
    try: 
        file = request.urlopen(img_link)
    except: 
        print("Bad Link: " + img_link)
        return -1

    size = file.headers.get("content-length")
    
    if size: 
        size = int(size) / 1000 # convert from bytes to KB
    else: 
        size = 0
    print(img_link + ", " + str(size))
    file.close()
    return(size)

def parse_link_data(link_data, dataset, img_sizes): 
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
                start = line.index('"') + 1
                end = line.rindex('"')
                img_sizes.append(get_img_size(line[start:end]))
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
            elif line.strip() == '"links": [],':
                dataset['num_links'].append(0)
            elif line.strip() == '"images": [':
                counting_images = True
            elif line.strip() == '"images": []':
                dataset['num_images'].append(0)
            else: 
                print("RUH ROH: " + line)

def parse_lines(lines):
    dataset = {
        'link': [],
        'size': [], 
        'num_links': [],
        'num_images': []
    }
    img_sizes = []
    link_data = []
    count = 0
    for line in lines: 
        if (count > 40):
            break
        
        if line.strip() == '{' or line.strip() == '}': 
            continue
        elif line.strip() == '},': 
            parse_link_data(link_data, dataset, img_sizes)
            link_data = []
            count += 1
        else:
            link_data.append(line)
    return [dataset, img_sizes]

def get_weights_csv(df, filename, column, num_buckets, low, high):
    # populate weights
    weights = [0] * num_buckets
    bucket_size = (high-low)/num_buckets
    for index, row in df.iterrows():
        if row[column] > high or row[column] < low:
            continue
        weights[int((row[column]-low) // bucket_size)] += 1
    # populate buckets
    buckets = []
    temp = low
    for i in range(num_buckets):
        buckets.append(temp)
        temp += bucket_size
    # create dataframe 
    data = {
        "buckets": buckets,
        "weights": weights
    }
    result = pd.DataFrame(data)
    path = r'/home/robinpreble/elvis/srg-elvis/tools/scraper/data_analysis/' + filename + '.csv'
    result.to_csv(path, index=False, header=True)

def main():
    #df = pd.read_csv('dataframe.csv')
    img_df = pd.read_csv('img_dataframe.csv')
    #get_weights_csv(df, 'size_weights', 1, 50, 0, 2500)
    #get_weights_csv(df, 'num_links_weights', 2, 50, 0, 1500)
    #get_weights_csv(df, 'num_images_weights', 3, 50, 0, 400)
    get_weights_csv(img_df, "image_size_weights", 0, 50, 0, 350)
    #get_weights_csv(df, 'image_size_weights', 0, 50, 0, 350)
    
"""
def main():
    f = open("visited.json", "r")
    lines = f.readlines()
    data = parse_lines(lines)

    dataset = data[0]
    img_sizes = data[1]
    df = pandas.DataFrame(dataset)
    img_df = pandas.DataFrame(img_sizes)

    df.to_csv(r'/home/prebler/elvis/scraper-min/data_analysis/dataframe.csv', index=False, header=True)
    img_df.to_csv(r'/home/prebler/elvis/scraper-min/data_analysis/img_dataframe.csv', index=False, header=True)

    plt.hist(df['size'], bins = 50, range = [0, 2500], color='blue', edgecolor='black')
    plt.xlabel('Page Size (KB)')
    plt.ylabel('No. of Pages')
    plt.title('Page Size')
    plt.savefig('page_size.pdf')
    plt.close()

    plt.hist(df['num_links'], bins = 50, range = [0, 1500], color='blue', edgecolor='black')
    plt.xlabel('No. of Links on Page')
    plt.ylabel('No. of Pages')
    plt.title('No. of Links')
    plt.savefig('num_links.pdf')
    plt.close()

    plt.hist(df['num_images'], bins = 50, range = [0, 400], color='blue', edgecolor='black')
    plt.xlabel('No. of Images on Page')
    plt.ylabel('No. of Pages')
    plt.title('No. of Images')
    plt.savefig('num_images.pdf')
    plt.close()

    plt.hist(img_sizes, bins = 50, range = [-1, 350], color='blue', edgecolor='black')
    plt.xlabel('Image Size (KB)')
    plt.ylabel('No. of Images')
    plt.title('Image Sizes')
    plt.savefig('image_sizes.pdf')
    plt.close()

    f.close()
"""

if __name__ == "__main__":
    main()