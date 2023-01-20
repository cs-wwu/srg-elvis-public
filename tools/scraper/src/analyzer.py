def parse_lines(lines):
    for line in lines: 
        if line.strip() == "{": 
            # do nothing
            print("start")
        elif line.startswith('  "'): 
            end = line.rindex('"')
            print(line[3:end])
        elif line.startswith('    "size":'):
            end = line.rindex(',')
            print("size: " + line[12:end])
        elif line.startswith('    "size":'):
            end = line.rindex(',')
            print("size: " + line[12:end])


def main():
    f = open("test.txt", "r")
    lines = f.readlines()
    parse_lines(lines)
    f.close()


if __name__ == "__main__":
    main()