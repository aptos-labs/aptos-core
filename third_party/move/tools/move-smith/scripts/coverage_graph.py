import re
import sys
import matplotlib.pyplot as plt
from pathlib import Path


def parse_libfuzzer_output(output_lines):
    coverage = []
    time_pass = []
    lines = [line for line in output_lines if line.startswith('#')]
    for idx, line in enumerate(lines):
        match = re.search(r'cov: (\d+)', line)
        if match:
            num = int(match.group(1))
        else:
            continue
        time_match = re.search(r'time: (\d+)', line)
        if time_match:
            tim = int(time_match.group(1))/3600
        else:
            continue
        if num != 0:
            if len(coverage) != 0 \
                    and num == coverage[-1] \
                    and idx != len(lines)-1:
                continue
            else:
                coverage.append(num)
                time_pass.append(tim)
    return time_pass, coverage

def plot_coverage(time_pass, coverage):
    plt.figure(figsize=(10, 6))
    plt.plot(time_pass, coverage, marker='o', linestyle='-', color='b')
    plt.xlabel('Hours')
    plt.ylabel('Block Coverage')
    plt.title('Coverage Over Time')
    plt.grid(True)
    plt.savefig('coverage.svg')
    plt.close()
    print("Coverage graph saved as 'coverage.svg'")

def draw_comparison(wkd: Path):
    plt.figure(figsize=(10, 6))
    plt.xlabel('Hours')
    plt.ylabel('Block Coverage')
    plt.title('Coverage Over Time')
    plt.grid(True)
    for log in wkd.rglob('fuzz.log'):
        log = log.absolute()
        run_name = log.parent.name

        output = open(log).readlines()
        time_pass, coverage = parse_libfuzzer_output(output)
        plt.plot(time_pass, coverage, marker='o', linestyle='-', label=run_name)
    plt.legend()
    plt.savefig('coverage-comparison.svg')
    plt.close()
    print("Coverage graph saved as 'coverage-comparison.svg'")

if __name__ == "__main__":
    if len(sys.argv) == 1:
        draw_comparison(Path.cwd() / "vm-results")
    else:
        libfuzzer_output = open(sys.argv[1]).readlines()
        time_pass, coverage = parse_libfuzzer_output(libfuzzer_output)
        plot_coverage(time_pass, coverage)
