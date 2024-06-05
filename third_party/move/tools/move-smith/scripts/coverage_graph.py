import re
import sys
import matplotlib.pyplot as plt


def parse_libfuzzer_output(output_lines):
    coverage = []
    time_pass = []
    for line in output_lines:
        if line.startswith('#'):
            match = re.search(r'cov: (\d+)', line)
            if match:
                coverage.append(int(match.group(1)))
            # exec_match = re.search(r'#(\d+)', line)
            # if exec_match:
            #     time_pass.append(int(exec_match.group(1)))
            time_match = re.search(r'time: (\d+)', line)
            if time_match:
                time_pass.append(int(time_match.group(1))/60)
    return time_pass, coverage

def plot_coverage(time_pass, coverage):
    plt.figure(figsize=(10, 6))
    plt.plot(time_pass, coverage, marker='o', linestyle='-', color='b')
    plt.xlabel('Minutes')
    plt.ylabel('Block Coverage')
    plt.title('Coverage Over Time')
    plt.grid(True)
    plt.savefig('coverage.svg')
    plt.close()
    print("Coverage graph saved as 'coverage.svg'")

if __name__ == "__main__":
    libfuzzer_output = open(sys.argv[1]).readlines()
    time_pass, coverage = parse_libfuzzer_output(libfuzzer_output)
    plot_coverage(time_pass, coverage)
