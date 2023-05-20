# Benchmarking Usage

## Using the Tool

You can use the benchmarker by going into the `run_benchmark.sh` file and under the `Beginning benchmarking` you can add your calls to the python function. This is fairly similar to the normal call to run Elvis, but with a few changes.

First you must use python3 instead of ./elvis

Then you call the `benchmarking.py` python file

Then you specify the NDL benchmarking file (see creation section)

Then the ranges of numbers of computers to test.

For example:

`python3 benchmarking.py base-basic.ndl 1000 10000 1000`

This specifies the base-basic.ndl simualtion, and coimputer counts ranging from 1000 to 10000 with an incrementor of 1000. This tool should work both on Windows and Linux, not MacOS however.

## Creating Benchmarks
Benchmarks can be created and used just like normal NDL defined simulations, except for counts which will use string indentifiers to fill in data.
For machine counts you can specify something like:

`[Machine name='sender' count='#machine_count']`

Which will tell the python program that this machine count should be replaced with the count passed in.
Similar idea follows for the recieve count on machines:

`[Application name='capture' ip='123.45.67.89' port='0xbeef' message_count='#recieve_count']`

Which also corresponds to the inputted machine count but may be caltulated to be a different value if a machine sends mutliple messages or something of that variety.

## New Fields
New fields are easy to add if you want to add a new string identifier. Simply open the `benchmarking.py` file and find the `create_and_run_sims` function. In there there is an if statemate allowing for more fields to be added.

