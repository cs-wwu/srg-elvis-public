# Runs the automated benchmarking of every sim in the sims directory. May take a long time.
# Usage: .\run_benchmark
if command -v python3 < /dev/null 2>&1; then
    echo "Valid Python Version Found"
else
    echo "Invalid Python Version Found"
    sleep 5
    exit
fi

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

echo "Moving to script directory"

cd $SCRIPT_DIR

echo "Installing Python Requirements"

pip install -r requirements.txt | grep -v 'already satisfied'

echo "Moving to sim directory for binary building"

cd ..

cargo build --release

echo "Moving to script directory"

cd $SCRIPT_DIR

cp ../target/release/elvis.exe ./

echo "Beginning benchmarking"

python benchmarking.py 490000 500000 10000

echo "Benchmarking finished, removing binaries"

# TODO: May be elvis for linux, not elvis.exe
rm ./elvis.exe

sleep 5
