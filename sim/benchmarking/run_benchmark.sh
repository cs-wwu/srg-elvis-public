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

echo "Starting sim file collection"

# dir_path="./sims"
# file_list=""
# for file in "$dir_path"/*; do
#     if [[ -f "$file" ]]; then
#         file_list="$file_list $(basename "$file")"
#     fi
# done

# echo "Found sims:"
# echo $file_list

echo "Beginning benchmarking"

# python benchmarking.py $file_list
# python benchmarking.py 100 500000 100
python benchmarking.py 100 1000 100

echo "Benchmarking finished, removing binaries"

# TODO: May be elvis for linux, not elvis.exe
rm ./elvis.exe

sleep 5
