from pathlib import Path
import pandas as pd
from datetime import datetime
import matplotlib.pyplot as plt

CURR = Path(__file__).parent
MOVE_SMITH = CURR.parent
VM_RESULTS = MOVE_SMITH / "vm-results"
OUTPUT_DIR = MOVE_SMITH / "coverage_graphs"

FEATURES = ["function", "line", "block", "branch"]


def parse_coverage_number(llvm_num: str):
    nums = llvm_num.split("(")[1].replace(")", "")
    nums = nums.split("/")
    return (int(nums[0]), int(nums[1]))


def get_coverage_detail_from(llvm_html: Path):
    tables = pd.read_html(llvm_html.open("r"))

    if len(tables) != 1:
        raise ValueError("No table or more than 1 table found in the HTML file")

    table = tables[0]
    table.columns = table.iloc[0]
    table = table[1:]
    table.set_index(table.columns[0], inplace=True)
    table_dict = table.to_dict(orient="index")

    result = {}
    total = {
        "function": [0, 0],
        "line": [0, 0],
        "block": [0, 0],
        "branch": [0, 0],
    }
    for component in table_dict.keys():
        if not component.startswith("move-"):
            continue
        covs = {}
        nums = parse_coverage_number(table_dict[component]["Function Coverage"])
        total["function"][0] += nums[0]
        total["function"][1] += nums[1]
        covs["function"] = nums

        nums = parse_coverage_number(table_dict[component]["Line Coverage"])
        total["line"][0] += nums[0]
        total["line"][1] += nums[1]
        covs["line"] = nums

        nums = parse_coverage_number(
            table_dict[component]["Region Coverage"]
        )
        total["block"][0] += nums[0]
        total["block"][1] += nums[1]
        covs["block"] = nums

        nums = parse_coverage_number(
            table_dict[component]["Branch Coverage"]
        )
        total["branch"][0] += nums[0]
        total["branch"][1] += nums[1]
        covs["branch"] = nums

        result[component] = covs
    result["total"] = total
    return result

def get_run_name(f: Path):
    return f.as_posix().split("/")[10]

def extract_date(fpath: Path):
    base_name = get_run_name(fpath)
    date_part = base_name.split("-")[:2]  # Extract the month and day part
    date_str = "-".join(date_part)
    return datetime.strptime(date_str, "%b-%d")


def sort_directories_by_date(fpaths):
    sorted_directories = sorted(fpaths, key=extract_date)
    return sorted_directories


def make_plot(results, names, component: str):
    pass
    plt.figure(figsize=(10, 9))
    plt.xlabel("Runs")
    plt.ylabel("Percentage")
    plt.title(f"{component}")
    plt.grid(True)

    for f in FEATURES:
        data = []
        for r in results:
            data.append((r[component][f][0] / r[component][f][1]) * 100)
        plt.plot(names, data, marker="o", linestyle="-", label=f)
    plt.xticks(rotation=45)
    plt.legend()
    OUTPUT_DIR.mkdir(exist_ok=True, parents=True)
    plt.savefig(OUTPUT_DIR / f"{component.split("/")[0]}.svg")
    plt.close()

def generate_index():
    svgs = [
        "total.svg",
        "move-vm.svg",
        "move-compiler-v2.svg",

        "move-compiler.svg",
        "move-bytecode-verifier.svg",
        "move-binary-format.svg",
        "move-borrow-graph.svg",
        "move-core.svg",
        "move-ir.svg",
        "move-ir-compiler.svg",
        "move-symbol-pool.svg",
        "move-model.svg",
        "move-prover.svg",
        "move-stdlib.svg",
        "move-command-line-common.svg",
    ]

    html_content = '''
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <style>
            body {
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                min-height: 100vh;
                margin: 0;
                font-family: Arial, sans-serif;
            }
            ul {
                list-style-type: none;
                padding: 0;
            }
            li {
                text-align: center;
                margin-bottom: 20px;
            }
            img {
                max-width: 100%;
                height: auto;
            }
        </style>
        <title>SVG Index</title>
    </head>
    <body>
        <h1>SVG Files</h1>
        <ul>
    '''

    for svg in svgs:
        html_content += f'''
        <li>
            <img src="{svg}" alt="{svg}" style="max-width:100%; height:auto;">
        </li>
        '''

    html_content += '''
        </ul>
    </body>
    </html>
    '''
    index_file = OUTPUT_DIR / "index.html"
    index_file.write_text(html_content)
    print("Coverage graphs can be viewed at index.html")

def main():
    files = list(VM_RESULTS.rglob("**/third_party/move/index.html"))
    files = sort_directories_by_date(files)
    print(f"Found {len(files)} coverage results")
    names = [get_run_name(f) for f in files]
    print(names)

    results = []
    for f in files:
        llvm_html = Path(f)
        try:
            coverage_detail = get_coverage_detail_from(llvm_html)
            results.append(coverage_detail)
        except Exception as e:
            print(f"Failed to get coverage detail from {llvm_html}: {e}")

    components = list(results[0].keys())
    for component in components:
        make_plot(results, names, component)
    generate_index()

if __name__ == "__main__":
    main()
