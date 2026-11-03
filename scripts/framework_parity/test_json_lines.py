import json
from pathlib import Path
import tempfile
import unittest

from json_lines import read_jsonl


class JsonLinesTest(unittest.TestCase):
    def test_unicode_separators_stay_inside_records(self):
        fixture = Path(__file__).parents[2] / 'tests/fixtures/framework-jsonl-unicode.jsonl'
        self.assertEqual(read_jsonl(fixture), [
            {'marker': 'NEL', 'source': 'before\u0085after'},
            {'marker': 'LS', 'source': 'before\u2028after'},
            {'marker': 'PS', 'source': 'before\u2029after'},
        ])

    def test_empty_record_is_an_error(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'records.jsonl'
            path.write_text('{}\n\n{}\n')
            with self.assertRaises(json.JSONDecodeError):
                read_jsonl(path)

    def test_empty_stream_has_no_records(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'records.jsonl'
            path.write_text('')
            self.assertEqual(read_jsonl(path), [])
