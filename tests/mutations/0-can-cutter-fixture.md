# Mutation record: the vendored can-cutter fixture

`tools/tests/t_fixture_manifest.sh` holds the vendored model files to the
hashes MANIFEST.toml records. Four mutants, four kills, each against a COPY of
the fixture directory via `FIXTURE_ROOT`.

| mutation | killed by |
| --- | --- |
| a vendored model edited, hash left alone | the hash arm |
| a recorded file deleted | the missing-file arm |
| a `.scad` added and not pinned | the unpinned arm |
| the hash block removed from the manifest | the blind-parser refusal |

THE THIRD ARM IS THE ONE THAT IS EASY TO MISS. Checking that every recorded
file still matches is the obvious direction; checking that every file PRESENT
is recorded is the other one, and without it a fixture can be added that
nothing pins. The fourth is the usual refusal: zero hash lines all matching is
vacuously true.

THE FAILURE MESSAGE SAYS WHAT TO DO. A changed file reports "REFRESH THE
NUMBERS, do not just update the hash", because the file changing is not the
problem. The problem is the file changing while the measured numbers beside it
stay, which turns the manifest into a precise description of a part that no
longer exists.

## Tiers

Touched: Tier 3, source-as-data. The measured numbers in MANIFEST.toml came
from running OpenSCAD 2026.10.14 against these exact files by hand, which no
tier covers and which is reported as what it is: one run, recorded, with the
version and date beside it. Skipped: every other tier.
