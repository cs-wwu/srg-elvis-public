import cmdbench
import matplotlib.pyplot as plt
import json
from os import listdir
from os.path import isfile, join
image_folder = "./benchmarking/benchmarking_graphs/"
sim_directory = "./benchmarking/sims/"
data_directory = "./benchmarking/raw_data/"
def run_sim(file_name, interations):
    benchmark_results = cmdbench.benchmark_command("elvis.exe --ndl " + sim_directory + file_name, iterations_num = interations)
    raw_file_name = file_name[0 : len(file_name)-4]
    #get averages, get statistics, and get resource plot are usefull commands
    testing = benchmark_results.get_statistics()
    # print(str(testing))
    testing_str = str(testing).replace('\'', '"').replace('(', '{').replace(')', '},').replace('mean', '"mean"').replace('stdev', '"stdev"').replace('min', '"min"').replace('max:', '"max":').replace('None', '"None"').replace(',\n  }', '\n  }').replace(',\n}', '\n}').replace(',\n\t}', '\n\t}')
    with open(data_directory + raw_file_name + ".json", "w") as outfile:
        outfile.write(testing_str)
    # We need to do something with the json returned -- TODO
    benchmark_results.get_resources_plot().savefig(image_folder + raw_file_name)

def mem_comparison_graphs():
    yAxis = []
    xAxis = []
    onlyfiles = [f for f in listdir(data_directory) if isfile(join(data_directory, f))]
    onlyfiles.sort(key=lambda a: int(a[a.index('-')+1 : -5]))
    for file_name in onlyfiles:
        f = open(data_directory + file_name, 'r')
        dictionary = json.loads(f.read())
        mem = float(dictionary['memory']['max_perprocess']['mean']) / float(60)
        yAxis.append(mem)
        xAxis.append(file_name[file_name.index('-')+1 : -5])
    # disabling the offset on y axis
    ax = plt.gca()
    ax.ticklabel_format(style='plain')
    plt.grid(True)
    plt.plot(xAxis,yAxis, color='maroon', marker='o')
    plt.title('Memory Usage Comparisons')
    plt.yscale('log')
    plt.xlabel('Machine Counts')
    plt.ylabel('Average Memory Usage in GigaBytes (per process)')
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
        time = dictionary['process']['execution_time']['mean']
        yAxis.append(time)
        xAxis.append(file_name[file_name.index('-')+1 : -5])
    plt.grid(True)
    plt.plot(xAxis,yAxis, color='maroon', marker='o')
    plt.title('Excecution Time Comparisons')
    plt.yscale('log')
    plt.xlabel('Machine Counts')
    plt.ylabel('Average Execution Time in seconds (per process)')
    plt.savefig(image_folder + 'Excecution-Time-Comparisons.png')
    plt.close()

if __name__ == '__main__':
    # run_sim("basic-100.ndl", 10)
    # run_sim("basic-1000.ndl", 10)
    # run_sim("basic-10000.ndl", 10)
    # run_sim("basic-50000.ndl", 10)
    # run_sim("basic-100000.ndl", 10)
    # run_sim("basic-250000.ndl", 10)
    # run_sim("basic-500000.ndl", 10)
    mem_comparison_graphs()
    execution_time_comparison_graphs()