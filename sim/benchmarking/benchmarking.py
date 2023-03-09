import cmdbench
from matplotlib import pyplot as plt
import json
import platform
import psutil
from os import remove as remove_file
import sys
import numpy as np
import re
# TODO: Fix building of cargo and selection of elvis vs elvis.exe for running based on OS
# TODO: fixing stdout prints
# TODO: Run full suite at 100+ iterations for full data points
# TODO: Fix subplot scaling

image_folder = "./benchmarking_graphs/"
sim_directory = "./sims/"
temp_sim_directory = sim_directory + "temp/"
data_directory = "./raw_data/"
iteration_count = 10
final_dict = {
    "platform":{
        'OS': platform.system(),
        'CPU': platform.processor(),
        'CORE_COUNT': psutil.cpu_count(),
        'RAM': psutil.virtual_memory().total
    }
}


def run_sim(file_name, interations):
    raw_file_name = file_name[0 : len(file_name)-4].replace(temp_sim_directory, "")
    print("Staring benchmark on: " + raw_file_name)
    sys.stdout.flush()
    binary_file = ""
    if platform.system() == 'Linux':
        binary_file = "elvis"
    elif platform.system() == 'Windows':
        binary_file = "elvis.exe"
    else:
        print('Unsupported operating system')
    benchmark_results = cmdbench.benchmark_command("./" + binary_file + " --ndl " + file_name, iterations_num = interations)
    memory_arr = benchmark_results.get_values_per_attribute()["memory"]
    process_time_arr = benchmark_results.get_values_per_attribute()['process']
    # print(benchmark_results.get_values_per_attribute())
    create_json_data(memory_arr, process_time_arr, raw_file_name)
    
def create_json_data(memory_arr, process_time_arr, raw_file_name):
    cur_sim = "Sim-" + raw_file_name
    core_data_dict = {
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
    }
    run_dict = {
        "Machine_Count": "".join(re.findall(r'\d+', raw_file_name)),
        "Iteration_Count": str(iteration_count),
        "data": core_data_dict
    }
    # # Find and set the memory data
    run_dict["data"]['memory']['mean'] = float(np.mean(memory_arr['max_perprocess']))
    run_dict["data"]['memory']['max'] = float(np.amax(memory_arr['max_perprocess']))
    run_dict["data"]['memory']['min'] = float(np.amin(memory_arr['max_perprocess']))
    run_dict["data"]['memory']['raw'] = memory_arr['max_perprocess']
    # # Find and set the processing data
    run_dict["data"]['processing_time']['mean'] = float(np.mean(process_time_arr['execution_time']))
    run_dict["data"]['processing_time']['max'] = float(np.amax(process_time_arr['execution_time']))
    run_dict["data"]['processing_time']['min'] = float(np.amin(process_time_arr['execution_time']))
    run_dict["data"]['processing_time']['raw'] = process_time_arr['execution_time']
    with open(data_directory + "core_data.json", "r") as outfile:
        temp_data = outfile.read()
        if temp_data != "":
            final_dict = json.loads(temp_data)
        else:
            final_dict = {
                "platform":{
                    'OS': platform.system(),
                    'CPU': platform.processor(),
                    'CORE_COUNT': psutil.cpu_count(),
                    'RAM': psutil.virtual_memory().total
                }
            }
    final_dict[cur_sim] = run_dict
    with open(data_directory + "core_data.json", "w") as outfile:
        json.dump(final_dict, outfile)
    final_dict = {
        "platform":{
            'OS': platform.system(),
            'CPU': platform.processor(),
            'CORE_COUNT': psutil.cpu_count(),
            'RAM': psutil.virtual_memory().total
        }
    }

def mem_comparison_graphs():
    yAxis = []
    xAxis = []
    f = open(data_directory + "core_data.json", 'r')
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
    plt.savefig(image_folder + 'Memory-Usage-Comparisons.png')
    plt.close()

def execution_time_comparison_graphs():
    yAxis = []
    xAxis = []
    f = open(data_directory + "core_data.json", 'r')
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
    plt.savefig(image_folder + 'Excecution-Time-Comparisons.png')
    plt.close()

def mem_comparison_per_machine_graphs():
    yAxis = []
    xAxis = []
    f = open(data_directory + "core_data.json", 'r')
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
    plt.savefig(image_folder + 'Memory-Usage-Comparisons-Per-Machine.png')
    plt.close()

# Generates sim files with machine counts from start to end counts. Increments machine counts by increment value.
def create_and_run_sims(base_file, start_count, end_count, increment):
    with open(data_directory + "core_data.json", "w") as outfile:
        pass
    f = open(sim_directory + base_file, 'r')
    sim = f.read()
    message_count = 1000
    for cur_count in range(start_count, end_count + increment, increment):
        cur_file_name = temp_sim_directory + base_file[0:-4] + '-' + str(cur_count) + ".ndl"
        sim = sim.replace('#message_count', str(message_count))
        sim = sim.replace('#machine_count', str(cur_count))
        if '#message_count' in sim:
            sim = sim.replace('#recieve_count', str(message_count*cur_count))
        else:
            sim = sim.replace('#recieve_count', str(cur_count))
        with open(cur_file_name, "w") as outfile:
            outfile.write(sim)
        run_sim(cur_file_name, iteration_count)
        remove_file(cur_file_name)


if __name__ == '__main__':
    create_and_run_sims(sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]))
    mem_comparison_graphs()
    execution_time_comparison_graphs()
    mem_comparison_per_machine_graphs()
