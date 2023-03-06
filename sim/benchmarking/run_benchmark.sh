# Runs the automated benchmarking of every sim in the sims directory. May take a long time.
# Usage: .\run_benchmark
if command -v python3 < /dev/null 2>&1; then
    echo "Valid Python Version Found"
else
    echo "Invalid Python Version Found"
    sleep 5
    exit
fi

if command -v rustc > /dev/null 2>&1; then
    echo "Rust is installed"
else
    echo "Invalid Rust Version Found"
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
# Check if OS type is Linux or Windows based
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    BINARY_NAME="elvis.exe"
# Check if the OS is Linux
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    BINARY_NAME="elvis"
else
    echo "Unsupported operating system"
    exit
fi

echo "Moving ELVIS binary to benchmarking directory"

cp ../target/release/$BINARY_NAME ./

echo "Beginning benchmarking"

python benchmarking.py 100 1000 100

echo "Benchmarking finished, removing binaries"

rm ./$BINARY_NAME
