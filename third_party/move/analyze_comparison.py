#!/usr/bin/env python3
"""
Analyze stackless vs file format comparison CSV data.
"""
import csv
import sys
import statistics


def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_comparison.py <csv_file>")
        sys.exit(1)

    filename = sys.argv[1]

    # Initialize counters
    total_stackless_instructions = 0
    total_file_format_instructions = 0
    total_stackless_locals = 0
    total_file_format_locals = 0

    # Track max values
    max_stackless_inst = {"value": 0, "function": ""}
    max_file_format_inst = {"value": 0, "function": ""}
    max_stackless_locals = {"value": 0, "function": ""}
    max_file_format_locals = {"value": 0, "function": ""}

    # Lists to store all values for median calculation
    stackless_inst_values = []
    file_format_inst_values = []
    stackless_locals_values = []
    file_format_locals_values = []

    # Counters for comparisons
    stackless_inst_less_or_equal = 0
    stackless_locals_less_or_equal = 0
    total_functions = 0

    # Track cases where file format is better (fewer instructions)
    file_format_better_cases = []

    # Read CSV file
    with open(filename, 'r') as f:
        reader = csv.DictReader(f)

        for row in reader:
            function = row["function"]
            stackless_inst = int(row["stackless instruction count"])
            file_format_inst = int(row["file format instruction count"])
            stackless_loc = int(row["stackless locals count"])
            file_format_loc = int(row["file format locals count"])

            total_functions += 1

            # Update totals
            total_stackless_instructions += stackless_inst
            total_file_format_instructions += file_format_inst
            total_stackless_locals += stackless_loc
            total_file_format_locals += file_format_loc

            # Store values for median calculation
            stackless_inst_values.append(stackless_inst)
            file_format_inst_values.append(file_format_inst)
            stackless_locals_values.append(stackless_loc)
            file_format_locals_values.append(file_format_loc)

            # Update comparison counters
            if stackless_inst <= file_format_inst:
                stackless_inst_less_or_equal += 1
            else:
                # File format has fewer instructions - calculate percentage increase
                pct_increase = ((stackless_inst - file_format_inst) / file_format_inst) * 100
                file_format_better_cases.append({
                    "function": function,
                    "stackless": stackless_inst,
                    "file_format": file_format_inst,
                    "pct_increase": pct_increase
                })

            if stackless_loc <= file_format_loc:
                stackless_locals_less_or_equal += 1

            # Update max values
            if stackless_inst > max_stackless_inst["value"]:
                max_stackless_inst = {"value": stackless_inst, "function": function}

            if file_format_inst > max_file_format_inst["value"]:
                max_file_format_inst = {"value": file_format_inst, "function": function}

            if stackless_loc > max_stackless_locals["value"]:
                max_stackless_locals = {"value": stackless_loc, "function": function}

            if file_format_loc > max_file_format_locals["value"]:
                max_file_format_locals = {"value": file_format_loc, "function": function}

    # Calculate medians
    median_stackless_inst = statistics.median(stackless_inst_values)
    median_file_format_inst = statistics.median(file_format_inst_values)
    median_stackless_locals = statistics.median(stackless_locals_values)
    median_file_format_locals = statistics.median(file_format_locals_values)

    # Calculate percentages
    pct_stackless_inst_less_or_equal = (stackless_inst_less_or_equal / total_functions) * 100
    pct_stackless_locals_less_or_equal = (stackless_locals_less_or_equal / total_functions) * 100

    # Print results
    print(f"Total functions analyzed: {total_functions}")
    print()

    print("=== Totals ===")
    print(f"Total stackless instructions: {total_stackless_instructions}")
    print(f"Total file format instructions: {total_file_format_instructions}")
    print(f"Total stackless locals: {total_stackless_locals}")
    print(f"Total file format locals: {total_file_format_locals}")
    print()

    print("=== Medians ===")
    print(f"Median stackless instructions: {median_stackless_inst}")
    print(f"Median file format instructions: {median_file_format_inst}")
    print(f"Median stackless locals: {median_stackless_locals}")
    print(f"Median file format locals: {median_file_format_locals}")
    print()

    print("=== Max Values ===")
    print(f"Max stackless instructions: {max_stackless_inst['value']} ({max_stackless_inst['function']})")
    print(f"Max file format instructions: {max_file_format_inst['value']} ({max_file_format_inst['function']})")
    print(f"Max stackless locals: {max_stackless_locals['value']} ({max_stackless_locals['function']})")
    print(f"Max file format locals: {max_file_format_locals['value']} ({max_file_format_locals['function']})")
    print()

    print("=== Comparisons ===")
    print(f"Stackless instructions <= file format instructions: {pct_stackless_inst_less_or_equal:.2f}% ({stackless_inst_less_or_equal}/{total_functions})")
    print(f"Stackless locals <= file format locals: {pct_stackless_locals_less_or_equal:.2f}% ({stackless_locals_less_or_equal}/{total_functions})")
    print()

    # Analyze cases where file format is better
    if file_format_better_cases:
        pct_increases = [case["pct_increase"] for case in file_format_better_cases]
        avg_pct_increase = statistics.mean(pct_increases)
        median_pct_increase = statistics.median(pct_increases)
        max_pct_increase = max(pct_increases)

        print("=== File Format Better Cases (stackless has MORE instructions) ===")
        print(f"Number of cases: {len(file_format_better_cases)} ({len(file_format_better_cases)/total_functions*100:.2f}% of all functions)")
        print(f"Average percentage increase of stackless over file format: {avg_pct_increase:.2f}%")
        print(f"Median percentage increase: {median_pct_increase:.2f}%")
        print(f"Max percentage increase: {max_pct_increase:.2f}%")
        print()

        # Show top 5 worst cases
        sorted_cases = sorted(file_format_better_cases, key=lambda x: x["pct_increase"], reverse=True)
        print("Top 5 cases where stackless is worst (highest % increase):")
        for i, case in enumerate(sorted_cases[:5], 1):
            print(f"  {i}. {case['function']}: stackless={case['stackless']}, file_format={case['file_format']}, increase={case['pct_increase']:.2f}%")
    else:
        print("=== File Format Better Cases ===")
        print("No cases where file format has fewer instructions than stackless.")


if __name__ == "__main__":
    main()
