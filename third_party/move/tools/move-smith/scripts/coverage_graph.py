import re
import sys
import matplotlib.pyplot as plt


def parse_libfuzzer_output(output_lines):
    coverage = []
    exec_count = []
    for line in output_lines:
        if line.startswith('#'):
            match = re.search(r'cov: (\d+)', line)
            if match:
                coverage.append(int(match.group(1)))
            exec_match = re.search(r'#(\d+)', line)
            if exec_match:
                exec_count.append(int(exec_match.group(1)))
    return exec_count, coverage

def plot_coverage(exec_count, coverage):
    plt.figure(figsize=(10, 6))
    plt.plot(exec_count, coverage, marker='o', linestyle='-', color='b')
    plt.xlabel('Execution Count')
    plt.ylabel('Block Coverage')
    plt.title('Coverage Over Time')
    plt.grid(True)
    plt.savefig('coverage.svg')
    plt.close()
    print("Coverage graph saved as 'coverage.svg'")

if __name__ == "__main__":
    libfuzzer_output = open(sys.argv[1]).readlines()
    exec_count, coverage = parse_libfuzzer_output(libfuzzer_output)
    plot_coverage(exec_count, coverage)
