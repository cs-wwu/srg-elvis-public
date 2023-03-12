import cmdbench
import json
import platform
import psutil
import cpuinfo
from os import remove as remove_file
import sys
import numpy as np
import re
from generate_graphs import generate_all_graphs

# TODO: 
# git ignore?

SIM_DIRECTORY = "./sims/"
TEMP_SIM_DIRECTORY = SIM_DIRECTORY + "temp/"
DATA_DIRECTORY = "./raw_data/"
ITERATION_COUNT = 10
final_dict = {
    "platform": {
        'OS': platform.system(),
        'CPU': cpuinfo.get_cpu_info()['brand_raw'],
        'CORE_COUNT': psutil.cpu_count(),
        'RAM': psutil.virtual_memory().total
    }
}


def run_sim(file_name, interations):
    raw_file_name = file_name[0: len(
        file_name)-4].replace(TEMP_SIM_DIRECTORY, "")
    print("Staring benchmark on: " + raw_file_name)
    sys.stdout.flush()
    binary_file = ""
    if platform.system() == 'Linux':
        binary_file = "elvis"
    elif platform.system() == 'Windows':
        binary_file = "elvis.exe"
    else:
        print('Unsupported operating system')
    benchmark_results = cmdbench.benchmark_command(
        "./" + binary_file + " --ndl " + file_name, iterations_num=interations)
    memory_arr = benchmark_results.get_values_per_attribute()["memory"]
    process_time_arr = benchmark_results.get_values_per_attribute()['process']
    cpu_usage = []
    for arr in benchmark_results.get_values_per_attribute()['time_series']['cpu_percentages']:
        cpu_usage.append(float(np.average(arr)) / float(psutil.cpu_count()))
    create_json_data(memory_arr, process_time_arr, cpu_usage, raw_file_name)


def create_json_data(memory_arr, process_time_arr, cpu_usage, raw_file_name):
    cur_sim = "Sim-" + raw_file_name
    core_data_dict = {
        'memory':
        {
            'mean': 0,
            'max': 0,
            'min': 0,
            'raw': [],
        },
        'processing_time': {
            'mean': 0,
            'max': 0,
            'min': 0,
            'raw': [],
        },
        'cpu_usage': {
            'mean': 0,
            'max': 0,
            'min': 0,
            'raw': [],
        },
    }
    run_dict = {
        "Machine_Count": "".join(re.findall(r'\d+', raw_file_name)),
        "Iteration_Count": str(ITERATION_COUNT),
        "data": core_data_dict
    }
    # # Find and set the memory usage data
    run_dict["data"]['memory']['mean'] = float(
        np.mean(memory_arr['max_perprocess']))
    run_dict["data"]['memory']['max'] = float(
        np.amax(memory_arr['max_perprocess']))
    run_dict["data"]['memory']['min'] = float(
        np.amin(memory_arr['max_perprocess']))
    run_dict["data"]['memory']['raw'] = memory_arr['max_perprocess']
    # # Find and set the process time data
    run_dict["data"]['processing_time']['mean'] = float(
        np.mean(process_time_arr['execution_time']))
    run_dict["data"]['processing_time']['max'] = float(
        np.amax(process_time_arr['execution_time']))
    run_dict["data"]['processing_time']['min'] = float(
        np.amin(process_time_arr['execution_time']))
    run_dict["data"]['processing_time']['raw'] = process_time_arr['execution_time']

     # # Find and set the cpu usage data
    run_dict["data"]['cpu_usage']['mean'] = float(
        np.mean(cpu_usage))
    run_dict["data"]['cpu_usage']['max'] = float(
        np.amax(cpu_usage))
    run_dict["data"]['cpu_usage']['min'] = float(
        np.amin(cpu_usage))
    run_dict["data"]['cpu_usage']['raw'] = cpu_usage
    with open(DATA_DIRECTORY + SAVED_DATA_FILE, "r") as outfile:
        temp_data = outfile.read()
        if temp_data != "":
            final_dict = json.loads(temp_data)
        else:
            final_dict = {
                "platform": {
                    'OS': platform.system(),
                    'CPU': cpuinfo.get_cpu_info()['brand_raw'],
                    'CORE_COUNT': psutil.cpu_count(),
                    'RAM': psutil.virtual_memory().total
                }
            }
    final_dict[cur_sim] = run_dict
    with open(DATA_DIRECTORY + SAVED_DATA_FILE, "w") as outfile:
        json.dump(final_dict, outfile)
    final_dict = {
        "platform": {
            'OS': platform.system(),
            'CPU': cpuinfo.get_cpu_info()['brand_raw'],
            'CORE_COUNT': psutil.cpu_count(),
            'RAM': psutil.virtual_memory().total
        }
    }


# Generates sim files with machine counts from start to end counts. Increments machine counts by increment value.
def create_and_run_sims(base_file, start_count, end_count, increment):
    # file name should be: sim_type-min_runs-max_runs-step_count.json
    global SAVED_DATA_FILE
    SAVED_DATA_FILE = base_file[0:-4] + '_' + str(start_count) + '_' + str(end_count) + '_' + str(increment) + '.json'
    
    with open(DATA_DIRECTORY + SAVED_DATA_FILE, "w") as outfile:
        pass
    f = open(SIM_DIRECTORY + base_file, 'r')
    sim = f.read()
    message_count = 1000
    for cur_count in range(start_count, end_count + increment, increment):
        temp_sim = sim
        cur_file_name = TEMP_SIM_DIRECTORY + \
            base_file[0:-4] + '-' + str(cur_count) + ".ndl"

        if '#message_count' in temp_sim:
            temp_sim = temp_sim.replace(
                '#recieve_count', str(message_count*cur_count))
        else:
            temp_sim = temp_sim.replace('#recieve_count', str(cur_count))

        temp_sim = temp_sim.replace('#message_count', str(message_count))

        temp_sim = temp_sim.replace('#machine_count', str(cur_count))

        with open(cur_file_name, "w") as outfile:
            outfile.write(temp_sim)
        run_sim(cur_file_name, ITERATION_COUNT)
        remove_file(cur_file_name)


if __name__ == '__main__':
    create_and_run_sims(sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]))
    generate_all_graphs(DATA_DIRECTORY + SAVED_DATA_FILE)

