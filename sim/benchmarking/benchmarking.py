import cmdbench
import json
image_folder = "./benchmarking/benchmarking_graphs/"
sim_directory = "./benchmarking/sims/"
data_directory = "./benchmarking/raw_data/"
def run_sim(file_name, interations):
    benchmark_results = cmdbench.benchmark_command("elvis.exe --ndl " + sim_directory + file_name, iterations_num = interations)
    raw_file_name = file_name[0 : len(file_name)-4]
    benchmark_results.get_iterations()
    #get averages, get statistics, and get resource plot are usefull commands
    testing = benchmark_results.get_statistics()
    # print(str(testing))
    testing_str = str(testing).replace('\'', '"').replace('(', '{').replace(')', '},').replace('mean', '"mean"').replace('stdev', '"stdev"').replace('min', '"min"').replace('max:', '"max":').replace('None', '"None"').replace(',\n  }', '\n  }').replace(',\n}', '\n}').replace(',\n\t}', '\n\t}')
    with open(data_directory + raw_file_name + ".json", "w") as outfile:
        outfile.write(testing_str)
    # We need to do something with the json returned -- TODO
    benchmark_results.get_resources_plot().savefig(image_folder + raw_file_name)

if __name__ == '__main__':
    run_sim("basic-100.ndl", 10)
    run_sim("basic-1000.ndl", 10)
    run_sim("basic-10000.ndl", 10)
    run_sim("basic-50000.ndl", 10)