import cmdbench
import matplotlib.pyplot as plt
import json
from os import listdir, getcwd
from os.path import isfile, join
import sys
image_folder = "./benchmarking_graphs/"
# sim_directory = "./sims/"
data_directory = "./raw_data/"
count = 10
def run_sim(file_name, interations):
    benchmark_results = cmdbench.benchmark_command("elvis.exe --ndl " + file_name, iterations_num = interations)
    raw_file_name = file_name[0 : len(file_name)-4]
    testing = benchmark_results.get_statistics()
    testing_str = str(testing).replace('\'', '"').replace('(', '{').replace(')', '},').replace('mean', '"mean"').replace('stdev', '"stdev"').replace('min', '"min"').replace('max:', '"max":').replace('None', '"None"').replace(',\n  }', '\n  }').replace(',\n}', '\n}').replace(',\n\t}', '\n\t}')
    with open(data_directory + raw_file_name + ".json", "w") as outfile:
        outfile.write(testing_str)
    # We need to do something with the json returned -- TODO
    benchmark_results.get_resources_plot().savefig(image_folder + raw_file_name)
# TODO: remove scientific notation
def mem_comparison_graphs():
    yAxis = []
    xAxis = []
    onlyfiles = [f for f in listdir(data_directory) if isfile(join(data_directory, f))]
    onlyfiles.sort(key=lambda a: int(a[a.index('-')+1 : -5]))
    for file_name in onlyfiles:
        f = open(data_directory + file_name, 'r')
        dictionary = json.loads(f.read())
        mem = float(dictionary['memory']['max_perprocess']['mean']) / float(1000000000)
        print(mem)
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
# TODO: Running manually without the bash script seems to be broken
if __name__ == '__main__':
    # for file_name in sys.argv[1:]:
    #     run_sim(file_name, count)
    mem_comparison_graphs()
    execution_time_comparison_graphs()

# TODO: Rewrite some library files to handle low times/percentages
#   File "C:\Users\Jacob\Desktop\ELVIS\srg-elvis\sim\benchmarking\benchmarking.py", line 68, in <module>
#     run_sim(file_name, count)
#   File "C:\Users\Jacob\Desktop\ELVIS\srg-elvis\sim\benchmarking\benchmarking.py", line 21, in run_sim
#     benchmark_results.get_resources_plot().savefig(image_folder + raw_file_name)
#   File "C:\Users\Jacob\AppData\Local\Programs\Python\Python310\lib\site-packages\cmdbench\result.py", line 172, in get_resources_plot
#     time_series_obj = self.get_averages()
#   File "C:\Users\Jacob\AppData\Local\Programs\Python\Python310\lib\site-packages\cmdbench\result.py", line 112, in get_averages
#     for from_ms in np.arange(sample_min_ms, sample_max_ms, avg_ms_per_sample):
# ValueError: Maximum allowed size exceeded