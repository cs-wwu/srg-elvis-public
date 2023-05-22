# Benchmarking Usage

## Using the Tool

You can use the benchmarker by calling the `run_benchmark.sh` with the appropriate command line parameters. Calling the script will make sure the correct version of Elvis is built for your OS as well as checking for a Python3 install and installing all of the Python libraries required via the `requirements.txt` file. Note that you must be in the `sim` directory or the `benchmarking` for this program to run.

To run a set of simulations to run you can use the following syntax:

```./run_benchmark.sh SIM_NAME STARTING_COUNT ENDING_COUNT```

For example calling:

```./run_benchmark.sh base-basic.ndl 1000 5000``` 

Which would specify the simulation `base-basic.ndl` to be ran with machine counts ranging from 1000 to 5000 with increments of 1000. 
It should be noted that you can forgoe this syntax if you wish to run a whole batch of simulations by simply adding called to the python program manually to the bash script. This is not advised but will work. This tool should work both on Windows and Linux but not MacOS.

## Creating Benchmarks
Benchmarks can be created and used just like normal NDL defined simulations, except for counts which will use string indentifiers to fill in data.
For machine counts you can specify something like:

`[Machine name='sender' count='#machine_count']`

Which will tell the python program that this machine count should be replaced with the count passed in.
Similar idea follows for the recieve count on machines:

`[Application name='capture' ip='123.45.67.89' port='0xbeef' message_count='#recieve_count']`

Which also corresponds to the inputted machine count but may be caltulated to be a different value if a machine sends mutliple messages or something of that variety.

## New Fields
New fields are easy to add if you want to add a new string identifier. Simply open the `benchmarking.py` file and find the `create_and_run_sims` function. In there, there is an if statement allowing for more fields to be added.

## Notes

There are a few issues with sub-libraries inside Python currently and causes warnings to appear about math. Note that these can mostly be ignored as the operations they reference are ones mostly not used.

