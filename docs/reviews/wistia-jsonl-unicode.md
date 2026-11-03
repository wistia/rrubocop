# Unicode-safe audit record loading

JSONL records use LF boundaries. Python `str.splitlines()` also splits raw NEL, U+2028 and U+2029 characters that valid JSON serializers can leave inside quoted strings. LinkToBlank's original whitespace cases exposed truncated audit records rather than a cop failure.

The shared reader now splits only LF and removes one terminal empty segment. Interior or extra empty records still fail JSON parsing; malformed/truncated records are not ignored. Three protocol regression tests cover actual raw Unicode separators, rejected empty records and empty streams. All 5,202 native and 1,734 original records from the 578-case Unicode-bearing audit load successfully. Existing comparison, correction, compilation and idempotence requirements are unchanged.
