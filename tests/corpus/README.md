# The rediscovery corpus

108 logged incidents from building one real part, as structured data. This is
the fixed-defect set the methodology's rediscovery gate reverts and re-finds.
It is **not a test**; it is what the tests will consume.

| file | what it is |
| --- | --- |
| `build.toml` | 77 incidents from the buildability log |
| `mcp.toml` | 31 incidents from the OpenSCAD MCP log |
| `source-*.md` | the two logs, vendored, so provenance is checkable here |

## How it was made, and what that guarantees

`tools/corpus-extract.py` pulls the numbered entries out of the two logs
verbatim and refuses on a gap or a duplicate number, because the logs are
numbered by hand and a parser that disagrees with the document silently loses
incidents. It extracts and never interprets.

The judgment fields -- the failure class, the state, the measurement, the fix --
are a separate pass, and `tools/corpus-build.py` validates them before writing
anything: ids must be exactly the extracted set, the class must be one of the
spec's seven or empty, nothing may be blank, and **every number in
`measurement` and `fix` must appear verbatim in that entry's own body.**

That last rule is the point. A reader summarizing 108 incidents will eventually
produce a figure that is plausible, useful, and not in the source, and that is
failure class B -- a lying instrument. Building the corpus that catalogues
class B by committing one would be a poor start. The check cannot tell whether
a sentence is a fair summary; no mechanical check can. It can make a fabricated
or miscomputed figure impossible to commit, which is the failure that would
actually mislead someone later.

## An empty class is an answer

`class = ""` means none of the seven genuinely fits, and 10 of 108 entries
carry it. Forcing a fit would make the taxonomy look complete and make the
corpus useless for the one thing it could tell us: where the seven classes do
not reach. Entries about ergonomics, serviceability, stress concentration and
material strength all came back empty, and that is a finding about the
taxonomy, not about the entries.

## The two readings of 52.800

The same figure appears in both logs with opposite meanings, and the corpus
keeps them apart:

- `mcp-026`, class B: four flank probes all reported 52.800 mm3 of material in
  a box whose radii put it outside the body wall. All four were a stale STL.
  **The number was a lie.**
- `build-066`, class G: the corrected run read ZERO on the two mouth-side
  flanks and 52.800 on the two outboard ones, which is what diagnosed a
  dovetail socket sheared in half by a coincident cut. **The number was true.**

Four probes at 52.800 is the stale-file signature. Two zeros and two at 52.800
is the real reading.

## Counts

The issue that commissioned this said 106, being 75 plus 31. It is 108: two
entries were added to the buildability log on 2026-11-16, while the dovetail
and press-stop defects that produced them were being fixed.
