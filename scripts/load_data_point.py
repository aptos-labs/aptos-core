import argparse
import json
from pathlib import Path
import re

def main(path):
    match = re.match(r'.*/(\d+)$', path)
    if match==None: return None
    x = int(match.group(1))
    jsonstr = Path(f'{path}/new/estimates.json').read_text()
    data = json.loads(jsonstr)
    y = data['median']['point_estimate']
    assert type(y)==float
    return x,y

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--path', required=True, help='Example: target/criterion/ristretto255/vartime_multiscalar_mul/12')
    args = parser.parse_args()
    x,y = main(args.path)
    print(json.dumps({'x':x, 'y':y}))
