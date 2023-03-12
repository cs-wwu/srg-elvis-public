# This file contains the functions to generate graphs from the data generated in benchmarking.
# The functions can be called indivually with filenames, or through calling the file itself with a filename to generate all the graphs

from matplotlib import pyplot as plt
import json
import sys
from os import mkdir, path

def mem_comparison_graphs(file_path):
    yAxis = []
    xAxis = []
    f = open(file_path, 'r')
    dictionary = json.loads(f.read())
    for sim in dictionary:
        for j in dictionary[sim].keys():
            if j == "data":
                mem = float(dictionary[sim][j]['memory']['mean']) / 1000000
                yAxis.append(mem)
        machine_count = ''.join(ch for ch in sim if ch.isdigit())
        if machine_count != '':
            xAxis.append(int(machine_count))
    # disabling the offset on y axis
    ax = plt.gca()
    ax.ticklabel_format(style='plain', axis='y')
    plt.grid(True)
    plt.subplots_adjust(bottom=.2, left=.2)
    plt.plot(xAxis,yAxis, color='maroon', marker='o')
    plt.title('Memory Usage Comparisons')
    plt.xlabel('Machine Counts')
    plt.ylabel('Average Memory Usage in MB')
    plt.savefig(IMAGE_FOLDER + 'Memory-Usage-Comparisons.png')
    plt.close()

def cpu_usage_comparison_graphs(file_path):
    yAxis = []
    xAxis = []
    f = open(file_path, 'r')
    dictionary = json.loads(f.read())
    for sim in dictionary:
        for j in dictionary[sim].keys():
            if j == "data":
                usage = float(dictionary[sim][j]['cpu_usage']['mean'])
                yAxis.append(usage)
        machine_count = ''.join(ch for ch in sim if ch.isdigit())
        if machine_count != '':
            xAxis.append(int(machine_count))
    # disabling the offset on y axis
    ax = plt.gca()
    ax.ticklabel_format(style='plain', axis='y')
    plt.grid(True)
    plt.subplots_adjust(bottom=.2, left=.2)
    plt.plot(xAxis,yAxis, color='maroon', marker='o')
    graph_name = 'CPU Usage Comparisons on ' + dictionary['platform']['CPU']
    plt.title(graph_name)
    plt.xlabel('Machine Counts')
    plt.ylabel('Average CPU Usage in Percentages')
    plt.savefig(IMAGE_FOLDER + graph_name)
    plt.close()

def execution_time_comparison_graphs(file_path):
    yAxis = []
    xAxis = []
    f = open(file_path, 'r')
    dictionary = json.loads(f.read())
    for sim in dictionary:
        for j in dictionary[sim].keys():
            if j == "data":
                time = float(dictionary[sim][j]['processing_time']['mean'])
                yAxis.append(time)
        machine_count = ''.join(ch for ch in sim if ch.isdigit())
        if machine_count != '':
            xAxis.append(int(machine_count))
    ax = plt.gca()
    ax.ticklabel_format(style='plain', axis='y')
    plt.grid(True)
    plt.subplots_adjust(bottom=.2, left=.2)
    plt.plot(xAxis, yAxis, color='maroon', marker='o')
    plt.title('Excecution Time Comparisons')
    plt.xlabel('Machine Counts')
    plt.ylabel('Average Execution Time in seconds')
    plt.savefig(IMAGE_FOLDER + 'Excecution-Time-Comparisons.png')
    plt.close()

def mem_comparison_per_machine_graphs(file_path):
    yAxis = []
    xAxis = []
    f = open(file_path, 'r')
    dictionary = json.loads(f.read())
    for sim in dictionary:
        machine_count = ''.join(ch for ch in sim if ch.isdigit())
        if machine_count != '':
            xAxis.append(int(machine_count))
        for j in dictionary[sim].keys():
            if j == "data":
                mem = float(dictionary[sim][j]['memory']['mean']) / 1000 / float(machine_count)
                yAxis.append(mem)
    # disabling the offset on y axis
    ax = plt.gca()
    ax.ticklabel_format(style='plain', axis='y')
    plt.grid(True)
    plt.subplots_adjust(bottom=.2, left=.2)
    plt.plot(xAxis,yAxis, color='maroon', marker='o')
    plt.title('Memory Usage Comparisons Per Machine')
    plt.xlabel('Machine Counts')
    plt.ylabel('Average Memory Usage in KB Per Machine')
    plt.savefig(IMAGE_FOLDER + 'Memory-Usage-Comparisons-Per-Machine.png')
    plt.close()

def generate_all_graphs(file_path):
    global IMAGE_FOLDER
    IMAGE_FOLDER = "./benchmarking_graphs/" + file_path[file_path.rindex('/') + 1 : -5] + '/'
    if not path.exists(IMAGE_FOLDER):
        mkdir(IMAGE_FOLDER)
    cpu_usage_comparison_graphs(file_path)
    execution_time_comparison_graphs(file_path)
    mem_comparison_per_machine_graphs(file_path)
    mem_comparison_graphs(file_path)

if __name__ == '__main__':
    file_path = sys.argv[1]
    generate_all_graphs(file_path)