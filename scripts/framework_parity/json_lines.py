import json


def read_jsonl(path):
    lines = path.read_text().split('\n')
    if lines[-1] == '':
        lines.pop()
    return [json.loads(line) for line in lines]
