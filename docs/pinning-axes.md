# The pinning axes

Every figure this project records was taken against a configuration. This
enumerates that configuration, says which parts are pinned, and says what
refuses when one moves.

## Why this is an enumeration and not a source review

cal asked, on 2026-11-17, for "a comprehensive review of OpenSCAD's source code
along with a comprehensive review of openscad-mcp's source code" to find the
spots that need pinning. The instinct was right; the instrument is cheaper than
that. OpenSCAD answers the question in **35 flags** of `--help`, and the
provider declares every knob it has in **one 445-line module**
(`openscad_mcp/utils/config.py`). A review of hundreds of thousands of lines of
C++ would be the most expensive instrument for the cheapest question.

The reason to enumerate at all is the score:

| how an axis was found | count |
| --- | --- |
| by something breaking | 3 |
| by reading two documented surfaces, once | 7 |

Three axes in weeks, by accident. Seven more in one sitting, on purpose. That
is the same rule this project already holds for guards — **enumerate, never
name** — applied to the pin.

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

**A `.env` reconfigures everything above, invisibly.** The provider calls
`load_dotenv()` with no argument on every `Config.from_env()`, and
python-dotenv searches from the working directory **upward**. So a stray `.env`
anywhere above wherever the server is started supplies any knob in this
document, and none of it is visible in this checkout. Measured 2026-11-18: none
reachable.

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
