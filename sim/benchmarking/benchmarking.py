import cmdbench
import matplotlib.pyplot as plt
import json
import platform
import psutil
from os import listdir
from os.path import isfile, join
import sys
import numpy as np

# TODO: Fix building of cargo and selection of elvis vs elvis.exe for running based on OS
# TODO: Collect all data needed and build full proper comparison graphs some may be
# TODO: Add new requirements to build file
## memory over time
## CPU utilization
## CPU VS Memory usage
# TODO: Run full suite at 100+ iterations for full data points

image_folder = "./benchmarking_graphs/"
sim_directory = "./sims/"
data_directory = "./raw_data/"
count = 10
def run_sim(file_name, interations):
    raw_file_name = file_name[0 : len(file_name)-4]
    print("Staring benchmark on: " + file_name)
    benchmark_results = cmdbench.benchmark_command("./elvis.exe --ndl "+ sim_directory + file_name, iterations_num = interations)
    memory_arr = benchmark_results.get_values_per_attribute()["memory"]
    process_time_arr = benchmark_results.get_values_per_attribute()['process']
    create_json_data(memory_arr, process_time_arr, raw_file_name)
    
def create_json_data(memory_arr, process_time_arr, raw_file_name):
    run_dict = {
        'memory':
        {
        'mean': 0,
        'max': 0,
        'min': 0,
        'raw': [],
        },
        'processing_time':{
            'mean': 0,
            'max': 0,
            'min': 0,
            'raw': [],
        },
        "platform":{
            'OS': platform.system(),
            'CPU': platform.processor(),
            'CORE_COUNT': psutil.cpu_count(),
            'RAM': psutil.virtual_memory().total
        }
    }
    run_dict['name'] = raw_file_name
    # Find and set the memory data
    run_dict['memory']['mean'] = float(np.mean(memory_arr['max_perprocess']))
    run_dict['memory']['max'] = float(np.amax(memory_arr['max_perprocess']))
    run_dict['memory']['min'] = float(np.amin(memory_arr['max_perprocess']))
    run_dict['memory']['raw'] = memory_arr['max_perprocess']
    # # Find and set the processing data
    run_dict['processing_time']['mean'] = float(np.mean(process_time_arr['execution_time']))
    run_dict['processing_time']['max'] = float(np.amax(process_time_arr['execution_time']))
    run_dict['processing_time']['min'] = float(np.amin(process_time_arr['execution_time']))
    run_dict['processing_time']['raw'] = process_time_arr['execution_time']

    run_json = json.dumps(run_dict)
    with open(data_directory + raw_file_name +".json", "w") as outfile:
        outfile.write(run_json)

# TODO: remove scientific notation
def mem_comparison_graphs():
    yAxis = []
    xAxis = []
    onlyfiles = [f for f in listdir(data_directory) if isfile(join(data_directory, f))]
    onlyfiles.sort(key=lambda a: int(a[a.index('-')+1 : -5]))
    for file_name in onlyfiles:
        f = open(data_directory + file_name, 'r')
        dictionary = json.loads(f.read())
        mem = float(dictionary['memory']['mean'])
        yAxis.append(mem)
        xAxis.append(file_name[file_name.index('-')+1 : -5])
    # disabling the offset on y axis
    ax = plt.gca()
    ax.ticklabel_format(style='plain')
    plt.grid(True)
    plt.subplots_adjust(bottom=0.2, left=0.2)
    plt.plot(xAxis,yAxis, color='maroon', marker='o')
    plt.title('Memory Usage Comparisons')
    plt.yscale('log')
    plt.xlabel('Machine Counts')
    plt.ylabel('Average Memory Usage in Bytes (per process)')
    plt.savefig(image_folder + 'Memory-Usage-Comparisons.png')
    plt.close()

def execution_time_comparison_graphs():
    yAxis = []
    xAxis = []
    onlyfiles = [f for f in listdir(data_directory) if isfile(join(data_directory, f))]
    onlyfiles.sort(key=lambda a: int(a[a.index('-')+1 : -5]))
    for file_name in onlyfiles:
        f = open(data_directory + file_name, 'r')
        dictionary = json.loads(f.read())
        time = dictionary['processing_time']['mean']
        yAxis.append(time)
        xAxis.append(file_name[file_name.index('-')+1 : -5])
    plt.grid(True)
    plt.subplots_adjust(bottom=0.2, left=0.2)
    plt.plot(xAxis,yAxis, color='maroon', marker='o')
    plt.title('Excecution Time Comparisons')
    plt.yscale('log')
    plt.xlabel('Machine Counts')
    plt.ylabel('Average Execution Time in seconds (per process)')
    plt.savefig(image_folder + 'Excecution-Time-Comparisons.png')
    plt.close()

if __name__ == '__main__':
    for file_name in sys.argv[1:]:
        run_sim(file_name, count)
    # # run_sim("basic-100.ndl", count)
    # # run_sim("basic-10000.ndl", count)
    # # run_sim("basic-50000.ndl", count)
    # # run_sim("basic-100000.ndl", count)
    # # run_sim("basic-250000.ndl", count)
    # # run_sim("basic-500000.ndl", count)
    mem_comparison_graphs()
    execution_time_comparison_graphs()
