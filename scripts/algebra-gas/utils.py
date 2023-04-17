import re

def parse_range_from_dataset_path(dataset_path):
    match = re.match(r'(.+)\.(\d+)-(\d+)\.json', dataset_path)
    assert match!=None
    base_path = match.group(1)
    x_begin = int(match.group(2))
    x_end =  int(match.group(3))
    return (base_path, x_begin, x_end)
