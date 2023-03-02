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

cd $SCRIPT_DIR

echo "Installing Python Requirements"

pip install -r requirements.txt

cd ..

cargo build --release

cd $SCRIPT_DIR

cp ../target/release/elvis.exe ./

dir_path="./sims"
file_list=""
for file in "$dir_path"/*; do
    if [[ -f "$file" ]]; then
        # file_list="$file_list $(basename "$file")"
        py benchmarking.py $(basename "$file")
    fi
done

echo "$file_list"
# py benchmarking.py $file_list

# py benchmarking.py basic-1000.ndl
# echo "should have waited"
# py benchmarking.py basic-10000.ndl

# py benchmarking.py basic-1000.ndl basic-10000.ndl basic-50000.ndl basic-100000.ndl basic-250000.ndl basic-500000.ndl

rm ./elvis.exe

sleep 10