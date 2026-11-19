# The pinning axes

Every figure this project records was taken against a configuration. This
enumerates that configuration, says which parts are pinned, and says what
refuses when one moves.

## Why this is an enumeration and not a source review

cal asked, on 2026-11-17, for "a comprehensive review of OpenSCAD's source code
along with a comprehensive review of openscad-mcp's source code" to find the
spots that need pinning. The instinct was right; the instrument is cheaper than
that. OpenSCAD answers the question in **35 flags** of `--help`, and the
provider's `Config.from_env` reads **20** environment knobs in one 445-line
module (`openscad_mcp/utils/config.py`). A review of hundreds of thousands of
lines of C++ would be the most expensive instrument for the cheapest question.

**What that module does NOT declare, and the correction this note carries.**
An earlier version of this page said the provider "declares every knob it has"
in `config.py`. It does not, and the counter-example is one of the axes in the
table below: `OPENSCADPATH` is read in `server.py` — at `:352`, building the
subprocess environment, and at `:423`, enumerating library search paths — and
appears **zero** times in `config.py` (measured 2026-11-19, rook's finding F2 on
`thread:wm-review`). BOSL2's version is on neither module; it is read out of
`BOSL2/version.scad` in the library tree. So the enumeration of the documented
surface did not produce two of the seven axes it was credited with, and would
not have produced them: **an enumeration is only as complete as the surface is,
and a configuration surface is not a closed set just because one module is
named `config`.**

The honest score, by where each axis actually came from:

| how an axis was found | count |
| --- | --- |
| by something breaking | 3 |
| by reading the two documented surfaces (`--help`, `config.py`), once | 5 |
| by looking for where a mechanism resolves, when the surface was silent | 2 |

Three axes in weeks, by accident. Seven more in one sitting, on purpose — five
of them by enumeration and two by chasing a mechanism the enumeration had no
entry for. That is still the rule this project holds for guards —
**enumerate, never name** — but with the limit stated: enumeration beats
accident, and does not reach a knob read outside the surface enumerated.

**The unit, stated, because the counts above mix two of them.** The table below
counts one row per *mechanism*, so **resource bounds** is one axis carrying four
knobs (`MCP_RENDER_TIMEOUT`, `MCP_MAX_FILE_SIZE_MB`, `MCP_MAX_MEMORY_MB`,
`MCP_ALLOWED_PATHS`): each turns a success into a refusal, which is the same
kind of answer-change. Counted as knobs instead of mechanisms, the seven are
ten. Neither count is wrong; quoting one without its unit is.

## The axes

| axis | mechanism | state |
| --- | --- | --- |
| provider version | PyPI sdist hash + peeled git tag | pinned, verified, refuses (rc 4/5) |
| kernel version | `openscad --version` | pinned, verified, refuses (rc 7) |
| kernel backend | `--backend=CGAL\|Manifold` | pinned, verified, refuses (rc 8) |
| **STL export flavor** | `--export-format`, and the default | pinned, verified, refuses (rc 9) |
| **library versions** | `BOSL2/version.scad` | pinned, verified, refuses (rc 10) |
| **kernel binary selection** | `OPENSCAD_PATH` | pinned unset, refuses (rc 11) |
| **library search path** | `OPENSCADPATH` | pinned unset, refuses (rc 11) |
| **diagnostic strictness** | `MCP_HARD_WARNINGS` | pinned unset, refuses (rc 11) |
| **resource bounds** | `MCP_RENDER_TIMEOUT`, `MCP_MAX_FILE_SIZE_MB`, `MCP_MAX_MEMORY_MB`, `MCP_ALLOWED_PATHS` | pinned unset, refuses (rc 11) |
| **dotenv** | `load_dotenv()` searching upward | refuses when one is reachable (rc 12) |
| **section contour cap** | `response_format`, `400 if detailed else 60` | recorded; the parser refuses a truncated list, and the deliberate arm drives both caps |

`provider/PROVIDER.toml` is the record; `tools/provider-verify.py` reads it and
refuses. The recorded value is also how a knob is set **deliberately**: `""`
means must-be-unset, and writing a value there is the supported way to change
one, per this project's rule to prefer configurability with a default that
derives.

## The four that are worth the words

**The STL flavor is announced to change.** OpenSCAD's own `--help` says it:
"ASCII export is the current stl default, but binary stl is planned as the
future default so asciistl should be explicitly specified in scripts when
needed." Every digest this project takes over an export — `digest_of_export`
and `digest_of_bytes` both — is taken over those bytes. The day the default
flips, every recorded digest changes, and before this, nothing we held would
have said why. Measured 2026-11-18: the bytes open `solid OpenSCAD_M`, 1497 for
`cube([10,10,10])`, so ASCII.

**`OPENSCAD_PATH` defeats the kernel pin.** It selects which binary the
provider runs. A `kernel_version` verified against `openscad` on `PATH` is then
a statement about a binary that is not the one used — a pin defeated by a
variable rather than by a disagreement, and silent.

**The library set is not a property of this checkout.** The corpus uses BOSL2,
whose version lives in `~/.local/share/OpenSCAD/libraries` and is reachable
only through `OPENSCADPATH` and two default directories. Measured 2026-11-18:
**2.0.716**, the same figure the fork's recorded baseline names.

**A `.env` reconfigures everything above, invisibly — and the first version of
this guard watched the wrong tree.** The provider calls `load_dotenv()` with no
argument on every `Config.from_env()`. `find_dotenv` with no path does **not**
start from the working directory: it takes the calling frame's `__file__`
(`dotenv/main.py:305-306`) and reaches for `os.getcwd()` only when
`usecwd=True`, or when the interpreter is interactive or frozen (`:300`). The
caller here is the installed `openscad_mcp/utils/config.py`, so the tree that
reconfigures this project is
`provider/.venv/lib/python3.12/site-packages/openscad_mcp/utils/` **and its
parents** — which on this box is a different tree from the checkout.

`tools/provider-verify.py` walked up from `Path.cwd()` and this page said
"from the working directory upward"; both were wrong, so rc 12 would have
passed a `.env` sitting beside the provider (rook, F2 on `thread:wm-review`,
2026-11-19). The guard now resolves the installed `config.py` first and
**refuses** (rc 12) when it cannot find it, because an answer about a tree
nobody looked in is the same defect one level up. Measured 2026-11-19: none
reachable from that directory.

**The section contour cap, which is where a figure would have been invented.**
Added 2026-11-19, found while fixing rook's F11. `measure(mode=section)` caps
each contour's point list at **`400 if detailed else 60`** (`server.py:3556`)
and replaces the remainder with `["...", "N more"]`. Measured: a `$fn=100`
cylinder returns **61 entries** without `response_format = "detailed"` and
**100** with it. That matters because `wm-model` computes each contour's
centroid, area and bbox **from those points** — so a truncated list would
yield figures that are wrong and look measured. `wire::section_from` refuses a
truncated list rather than averaging what arrived, and
`crates/wm-provider/examples/section-from-provider.rs` drives both caps
against the live provider. The points are `round(x, 4)` in either mode:
`detailed` moves the cap, not the precision.

## Enumerated and deliberately not refused

Stated so the omission is a decision rather than a gap.

**The render cache** (`MCP_CACHE_ENABLED`, `_SIZE_MB`, `_TTL_HOURS`; on by
default, 500 MB, 24 h). The cache directory on this box holds 8 entries dated
**2026-11-16** — the day the provider figures were taken — which looks alarming
and is not. Measured: the cache has exactly **one** reader, `_check_cache` at
`server.py:944`, inside `render_scad_to_png`. It serves PNG renders and nothing
else; no `measure`, `validate`, `check` or `scad_eval` result is cached, and
those 8 entries carry `/tmp/pytest-of-cpower` paths from the provider's own
suite. No recorded figure is implicated.

**Transport, logging, image size and concurrency**: `MCP_PORT`, `MCP_HOST`,
`MCP_TRANSPORT`, `MCP_LOG_LEVEL`, `MCP_LOG_FILE`, `MCP_RATE_LIMIT`,
`MCP_MAX_IMAGE_WIDTH`, `MCP_MAX_IMAGE_HEIGHT`, `MCP_MAX_CONCURRENT_RENDERS`,
`MCP_TEMP_DIR`, `IMAGEMAGICK_PATH`. None can move a geometric figure.

**Kernel flags we do not pass and the provider does not pass**: `--animate`,
`--animate_sharding`, `--csglimit`, `--render`, `--preview`, `--projection`,
`--view`, `--camera`, `--imgsize`, `--colorscheme`, `--autocenter`,
`--viewall`, `--reset-window-settings`. These are PNG-export and GUI concerns.
`--trace-depth` and `--trace-usermodule-parameters` become axes the day #37
lands; `--check-parameters` and `--check-parameter-ranges` change whether a bad
call warns, and are candidates once anything of ours reads those warnings.

## One thing this disproved

`--summary` looked like a better fix for #53 than forcing the slow CGAL
backend: it emits JSON statistics and takes a `geometry` keyword. Measured
2026-11-18, `--summary all --summary-file -` returns cache, camera, time, and
`geometry` as bounding box, `convex`, `dimensions`, `facets`, `triangular`.
**No manifoldness, no volume count.** It is not a substitute for the CGAL
banner, and #53's measured one-flag fix stands.
