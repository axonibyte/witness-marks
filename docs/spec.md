# Witness Marks

A server that lets an AI *see, measure, and make promises about* the parts it
designs in OpenSCAD — derived from a hundred and six logged failures building
one part, then widened to the general case.

> Rendered copy: https://claude.ai/artifact/NRzUSgAmR53Q1rPiHebUvB
> THIS FILE IS CANONICAL as of 2026-11-14; it moved here from /data/projects/tech/designs/mcp-surface.md, which is now the copy.
> diverge, this one wins — and that divergence is exactly the failure this
> document is about, so fix it rather than living with it.

Evidence: `buildability-tests.md` (75 entries), `openscad-mcp-notes.md` (31
defects), `overhang-scan.scad` (the slicer-criterion test, validated against
four hand-computed shapes), `kobra-neo.md` (one printer's measured
constraints). **Draft 5, 2026-11-16.**

**Witness Marks is a NEW server**, not an extension of the one the harness was
built through. Every claim below is badged: **[evidence]** — a specific logged
incident is behind it; **[general]** — added for general agentic use, reasoned,
not yet failed; **[exists]** — already shipped in `RobertCoop/openscad-mcp`; consume it, do not rebuild it.

---

## 00. The short version, for anyone

OpenSCAD turns a text file into a 3D shape. An AI can already write that text
well. What it *cannot* do is pick the shape up and turn it over. It can't ask
"is this one solid piece or two?", "how much air is under that ledge?", "do
these two parts hit each other when one slides?", or "will my printer actually
build this?" Today every one of those questions means exporting a file and
writing a throwaway script to poke at it — and those scripts turn out to lie
about as often as they tell the truth.

Witness Marks sits between the AI and OpenSCAD and answers those questions
directly, as numbers, each with a receipt saying exactly which version of the
model it came from. It is built from six pieces:

- **Edit** — read the model's knobs and building blocks; render one piece at a
  time; tell the AI exactly what an edit changed in the *shape*, not just the text.
- **See** — pictures from angles the AI chooses, cross-sections at any height,
  one part lit up in a crowd: the view a person gets by holding it.
- **Measure** — volume, bounding box, how many pieces, how much air under a
  face, how much two parts overlap and *where*.
- **Fit & move** — slide a part through its whole travel and report every
  collision. Check it can be put *in*, not just that it fits once seated.
- **Physics, small** — four formulas (springs, screws, friction, press-fits),
  because every mechanical question in the test build was first-order.
- **Promises** — the AI writes down what the design must achieve ("this clip
  grips with 2–5 N") and the server re-checks every promise on every change and
  says aloud which ones just went false.

One rule wraps all of it: **the server never says "nothing there" by staying
silent.** "Empty" is an answer with a timestamp. Every probe proves it *could*
have found something before it reports that it didn't. Half the hours lost
building the test part went to instruments that broke this rule.

In one sentence: it gives an AI hands and a tape measure, and a notebook that nags.

## 01. The thesis

Across a hundred and six logged failures building one part, **not one was a
failure to express a shape**. Every one was a question that could not be asked
about what had already been built, or a number that went stale without telling
anyone. So the effort goes to **interrogation** and to **claims**, not to
authoring.

The harness file carries twelve comments stating a derived number — spring
rates, clamp forces, margins. Each was computed once in a scratch script and
pasted in. When the material changed from PLA to PETG one sentence into a
conversation, six of the twelve went wrong at once and nothing in the file knew.

> **The live case.** User: "this is petg." Spring rate is linear in modulus;
> PETG ~2000 MPa against PLA's 3500. The clip's clamp dropped 3.17 N → 1.81 N,
> losing 43% of the one property that sub-assembly exists to provide. A comment four
> lines above the parameter still read `2.4x under yield`.

## 02. Scope: from one harness to general work

Drafts 1 and 2 described a server fitted to one can cutter. This draft describes
one fitted to *any* OpenSCAD work an agent does — and marks which is which.

The harness was accidentally rich: a compliant mechanism, a printed thread, an
insertion sequence, a consumable with real tolerances, a four-part fit. But it
was **one part, one agent, one printer**. What it could not produce: anything
about reading someone else's file, working with a library like BOSL2,
enumerating parts for export, a long render that must not block, or a geometry
diff after an edit. Those are the daily substance of general agentic modeling,
and they are badged **[general]** below.

The logs also have a blind spot shaped like their author, found by grepping
rather than remembering: **zero entries about building the wrong thing.** The
project's most expensive turn — three unrequested rewrites of a working
sub-assembly — is in neither log. No layer here would have prevented it. §10
names that as the test this document cannot pass.

## 03. Prior art, and the decision it forces

Draft 3 named one prior server — the one the harness was built through — and
said "nothing measures." True of that server, false of the field. A survey of
OpenSCAD, CadQuery, build123d, FreeCAD and Fusion MCP servers found one that does
roughly half of this document, well.

### RobertCoop/openscad-mcp

171★, MIT, Python, 12 tools, ~1,500 tests behind an 80% coverage floor, a
15-task geometry eval harness. Read from README, API.md and one source file; not
run.

| Layer | There |
|---|---|
| E · edit | **Yes.** `model` CRUD with content-hash etags; `scad_eval` (typed expression eval in scope — better than `model.echo`); `trace` follows a constant through a project (= `claim.fanout`'s skeleton); BOSL2 lint with autofix. Variables injected by source-wrapping, not `-D`. No geometric diff. |
| V · see | **Yes.** Views, sections with scale bar, per-part ghost/colour/explode, spatial digest (camera, scale, bbox) before every image. |
| 1 · measure | **Yes.** Volume, components, watertight, mesh health, sections, mass; interference with penetration depth and a *witness point*; clearance with closest points. **`UNRESOLVED` below tessellation error** — a refusal this document lacked. |
| 1b · history | Foundation: a CSG-dump parser for holes, bosses, cross-part alignment. Not `eats`, not `coincident`. |
| 2 · motion | **Yes, arguably better.** Linear/rotary sweeps with `first_contact`, `worst`, `min_gap`; full rotation yields an `all_angles` *certificate* from footprints, not samples. No `insert`, no `travel_stop`. |
| 3 · mechanics | **Absent.** No calculators: "beam" and "stress" appear only as prose in `reference.py`, whose own advice says *"for a real stress calculation use the BASF cantilever formulas"*; "spring" and "torque" have no hits at all. Mass properties and a sourced reference table only. |
| 4 · process | Partial, face-normal based (`printability.py`: facet normal vs a 45° default). No critical angle from layer height, no slicer offset criterion, no quantize, no firmware. |
| 5 · claims | Half. `validate predicates` + `sweep` → `first_failure`, `crossing`, `monotonic`. One-shot. **No staleness propagation, dominance or input provenance.** |
| envelope | Mostly. `valid` false on any ERROR regardless of exit code; `empty: true`; empty parts raise `UNRESOLVED` "so a check cannot pass green"; `cached: true`. Not per-result provenance hashes. |
| W · workflow | Parts grammar, CI-mode `check` with exit codes, cache keyed on every file read. **No async jobs** — 30–3600 s timeout then TIMEOUT. |

The unoccupied ground is exactly §06's "where this stops being a wrapper":
mechanics, staleness, dominance, provenance-typed inputs, process-as-model with
the slicer criterion, `csg.eats`/`coincident`, async jobs. Everything below that
line exists under MIT with a test suite; **[exists]** marks it through §05.

### Vital signs

Not vaporware. A careful single-maintainer project with a burst cadence and a
two-month silence. The whole history is 47 commits: Aug 2025 (12), Sep 2025 (2),
a five-month gap, Feb–Mar 2026 (4), Jul (5), Aug (8), Sep (16); nothing since 10
September. Maintainer 39 of 47 (two spellings; 40 under three, if "Robert Coop"
is the same person), one outside contributor with 5, two with 1 each.
Releases 0.2.0 → 0.3.0 → 0.6.0/0.6.1; PyPI publishing began at 0.6.1. CI with
trusted publishing. All six closed outside PRs were merged — the fastest the
**same day**, the slowest after **137 days** — and PR #12 (a fix, 30 Sep) has
had no comment in six weeks.

The 549-line CONTRIBUTING.md is the strongest signal either way: a "Design Rules
That Should Not Be Undone" section that reads like this document's §07, honesty
about 1,280 ruff findings and 77 mypy errors as debt, a ban on wholesale
reformatting inside functional changes — and **a tool-surface budget test**:
21,000 chars of JSON schema total, 3,600 per tool, with the principle that *"a
feature is a mode of an existing tool until it proves it needs a tool of its
own."*

### The forks

Twenty. Seventeen are snapshots — their "ahead" counts are stale divergence,
all 44 behind — and two are byte-identical to current main, carrying nothing of
their own. One is real: `Trebuhgh/openscad-mcp`, 16 commits on current main from
18–19 Sep: portability fixes, a sweep-variant fix, and five `refactor: extract…`
commits pulling parameter parsing, image handling, cache, runtime discovery and
process execution out of `server.py`. Competent, not upstreamed. None is a better
base.

### Build, fork, or contribute

The intent is a full [Portfolio of Oracles](https://blog.axonibyte.com/a-portfolio-of-oracles)
installation: every tier answering a question no cheaper tier can,
mutation-checked assertions, the rediscovery gate. That is a methodology
transplant — dozens of PRs across `tests/`, CI and structure — against a
maintainer whose observed review latency is weeks to months, whose CONTRIBUTING
rightly refuses large diffs inside functional changes, and whose tool-surface
budget would force Layers 3, 5, 1b and W into modes of `measure`/`validate`/
`check` or into a fight for a tool slot each. A design-philosophy conflict, not a
social one, and load-bearing on their side for good reason.

1. **Build Witness Marks as its own server, consuming RobertCoop as the geometry
   provider** — via its MCP surface or Python API, version-pinned. §04 already
   says the kernel is replaceable; this makes the geometry layer replaceable
   too. Witness Marks owns what does not exist.
2. **Contribute upstream only what is unambiguously theirs and small**, one PR
   each, shaped to their recipes: the slicer offset criterion into
   `validate(mode="printability")`; `coincident` as a `check` rule (their
   "Adding a Check Rule" section is the recipe); the Trebuhgh refactors if that
   author consents.
3. **Do the Portfolio of Oracles installation on Witness Marks**, where the tree
   is ours. Two tiers map directly: *cross-language vectors* becomes
   differential testing — our geometry checks against the provider's, neither
   the oracle, a committed vector file is; the *rediscovery gate* is already §10
   test 3 done strictly — the 106 logged failures are the fixed defects to
   revert and re-find, not merely to count. (The methodology calls it the
   rediscovery test.)
4. **Fork only if the provider goes dark.** Two months is not dark; an
   unanswered PR at six weeks is yellow, not red. The pin plus the differential
   suite makes a later fork cheap rather than urgent.

One of their design rules needs a flag here: *"Correctness outranks preserving
the user's formatting… the server may rewrite the project file."* Layer E
assumes the opposite — the harness lost a user's edits twice to exactly that.
Consuming the provider through its MCP surface keeps that rule on their side of
the boundary.

## 04. Architecture

| Layer | Owns | Answers |
|---|---|---|
| 0 · kernel | OpenSCAD: CSG, meshing, export — replaceable | — |
| **E · edit** | model handles, parameters, modules, typed render, echo, diff | what is this file, what did I just change |
| **V · see** | views, sections, highlight, explode | what does it look like from here |
| 1 · measure | typed queries over solids | how big, how many, how much air |
| 1b · history | the construction tree, not just its result | which operation ate which feature |
| 2 · fit & motion | parts, mates, DOF, travel, insertion | does it move, does it go in |
| 3 · mechanics | closed-form beam, screw, friction, press | will it hold |
| 4 · process | printer, material, firmware, slicer as a model | will it build |
| 5 · claims | assertions, staleness, dominance, fan-out | is it still true |
| **W · workflow** | parts, exports, long jobs, cache | what is stale, what is running |
| — · envelope | status, provenance, controls | can I trust this answer |

E, V and W were new in draft 3: an agent spends most of a real session *reading
and changing* a model, not interrogating a finished one. Layers 3–5 remain where
this stops being a wrapper.

**THE PROVIDER BOUNDARY, draft 4.** Layers 0, E (except diff), V, 1, 2 and the
cache half of W are PROVIDED — by `RobertCoop/openscad-mcp` today, by whatever
answers the same questions tomorrow. Witness Marks OWNS 3, 5, the criterion half
of 4, `csg.eats`/`coincident`, async jobs, and the envelope as a rule the
provider's answers are wrapped in, not trusted to carry. The boundary is a
hypothesis; §10 test 5 says how it fails.

**Granularity is architectural.** [evidence] One interference question today is
six steps and a round-trip. So the primitives compose server-side: one call,
both operands rendered, each operand's own volume beside the result. A surface
that is right and chatty does not get used.

## 05. The surface

Signatures are illustrative; what matters is the question each answers and the
shape of its return.

### Layer E — edit (new)

    model.open(path | source)          -> handle {hash, includes[], libraries[], warnings[]}
    model.parameters(h)                -> [{name, value, type, group, comment, line, used_by[]}]
    model.modules(h)                   -> [{name, args[], line, calls[], called_by[]}]
    model.render(h, module?, params?)  -> mesh handle   // params TYPED, not -D strings
    model.echo(h, params?)             -> [{line, values[]}]   // the file's own instruments
    model.diff(h_before, h_after)      -> {added_volume, removed_volume, regions[], params_changed[]}
    model.lint(h)                      -> [{rule, line, text}]

- **A handle, not a string.** [evidence] [exists] Makes two calls knowably about the same
  model, carries the hash provenance needs, lets the server cache meshes.
- **Typed parameters.** [evidence] [exists: by source-wrapping] `-D` appends an assignment, so a misspelled name
  defines an unread variable and changes nothing — and draws no warning,
  because an assignment nobody reads is legal. (A name the file *reads* but
  never assigns warns "Ignoring unknown variable" without `-D` and is silently
  satisfied with it: measured, OpenSCAD 2026.10.14.) Its string values
  fails silently; its strings survive two parsers. A typed map with the
  parameter list as schema makes both unrepresentable.
- **Echo is an instrument.** [evidence] [exists: as `scad_eval`, which is better] Models communicate derived values via
  `echo()`; today it is buried in stderr — and on Windows the `openscad.com`
  wrapper merges it into stdout instead, so a parser keyed on the stream is
  wrong on one platform or the other. [general: the Windows half is read from
  the wrapper's purpose, not run here] Return it structured.
- **Diff by geometry.** [general] The common agentic edit is small and the
  common fear is unintended change. One adjacent incident: a bit-identical
  result after a change was the tell that the change had not applied.
- **Library resolution.** [exists] BOSL2, MCAD, user libraries. A missing
  `include` is a WARNING and the render proceeds without it. [evidence]

### Layer V — see (new)

    view.render(mesh, camera | named, {mode: preview|full, ghost: keep|drop}) -> image
    view.section(mesh, plane)          -> image + polygons
    view.highlight(h, module, camera)  -> image, that module lit, the rest ghosted
    view.explode(parts[], vector)      -> image
    view.standard(mesh)                -> {iso, top, front, right, bottom}

- **The camera the agent asked for.** [evidence] [exists] Prior art forced `--viewall`;
  its default was an unlabelled throwntogether preview.
- **Ghost geometry is a mode, not a surprise.** [evidence] [exists] `%` vanishes in a
  full render and survives in preview; the caller chooses, the result says.
- **Highlight is how an agent finds things.** [exists: per-part ghost/colour] Reading an unfamiliar
  file is mostly "where is this module."
- **Sections are the diagnostic view.** [evidence] [exists] 0.22 mm of air under a leaf
  was found by hand-slicing a dozen heights.

### Layer 1 — measure

    solid.measure(mesh)             -> volume, area, bbox, centroid, genus, manifold
    solid.components(mesh)          -> count, bodies[{volume, bbox}]   // genus CANNOT answer this
    solid.section(mesh, plane)      -> area, perimeter, extents, polygons
    solid.ray(mesh, origin, dir)    -> spans[{enter, exit}]   // spans, never hits
    solid.gap_below(mesh, face)     -> depth, lands_on, reach_to_nearest_support
    solid.clearance(a, b)           -> min_distance, witness_points
    solid.interference(a, b)        -> volume, operand_volumes[2],
                                       regions[{volume, bbox, centroid, cylindrical:{r,theta,z}}]
    solid.locate(h, module)         -> bbox, centroid of what that module contributed
    solid.at(h, point)              -> inside|outside, nearest_face, contributing_modules[]

`ray` returns spans — a surface-only probe cannot tell "void beneath" from
"inside solid." [evidence] `interference` reports regions in cylindrical
coordinates — a blob's angle named the cause every time. [evidence] It returns
`operand_volumes`, because an operand that rendered nothing is the difference
between "these do not touch" and "I measured nothing." [evidence] `locate` and
`at` are how an agent orients in a file it did not write. [exists: `measure` modes `probe`, `features`]

### Layer 1b — history

    csg.operations(h)               -> tree[{op, feature, bbox, derived_from[]}]
    csg.eats(h, feature)            -> ops[] that remove material from this feature
    csg.flank(h, feature, dir)      -> material between the feature and the nearest void
    csg.coincident(h, tol)          -> pairs[{a, b, shared_boundary}]

Every Layer 1 call runs on the *result*. One class of defect lives in the
*operations*: two correct features whose positions coincide. [evidence] The
dovetail sockets sat at exactly the mouth cut's half-angle; the cut sheared a
flank off both. `csg.eats` reports it before a print.

### Layer 2 — fit and motion

    asm.define(parts, mates)            -> handle
    asm.dof(a)                          -> free[], constrained[], overconstrained[]
    asm.sweep(a, joint, from, to, n)    -> collisions[{t, volume, regions}], min_clearance
    asm.insert(part, path)              -> same, for getting it IN rather than operating it
    asm.travel_stop(a, joint)           -> limiting_part, at_t, remaining_range
    asm.fits(a)                         -> matrix[{a, b, kind: clearance|interference, value}]

`sweep` exists in the provider with a footprint certificate for full rotations. [exists] `insert` ≠ `sweep`: clear through the whole range yet impossible to install, or
the reverse — both happened. [evidence] `travel_stop`: not "does it collide" but
*what stops it first* — the blade was stopped by the frame with interference at
rest exactly zero. [evidence] `fits`: every mating pair with its sign checked; a
retainer modelled with a clearance retains nothing. [evidence]

### Layer 3 — mechanics, closed-form

    mech.beam(kind, E, L, w, t, {delta|force})   -> k, force, stress, margin
    mech.screw(mu, pitch, r_mean, load)          -> torque_raise, torque_lower, self_locking
    mech.friction_chain([{radius, mu, normal}])  -> slips_first, margins[]
    mech.press(interference, E, geometry)        -> pressure, holding, insertion_force
    mech.tolerance_chain(links)                  -> amplification, worst_case, dominant_link

No FEA. Four formulas decided every mechanical question in the harness.
[evidence] `tolerance_chain` catches a 3.54:1 lever turning print tolerance into
zero clamp — invisible to every geometric check. [evidence] Whether general use
needs more than first-order is open; ship these five and log what they fail to
answer. [general]

### Layer 4 — process as a model

    proc.declare({material, E, yield, layer_h, nozzle, width, bed, firmware_limits})
    proc.quantize(dimension)        -> layers, actual, error, cubed_error_if_bending
    proc.support(mesh, orientation) -> area, by_layer[], regions[{bridgeable, anchors, span}]
    proc.orient(mesh, loads)        -> ranked[{rotation, support, bed_contact, layer_vs_stress}]
    proc.firmware(request)          -> {commanded, reachable, limited_by}
    proc.features(mesh)             -> thin_walls[], small_holes[], trapped_volumes[], min_feature
    proc.fits_bed(mesh)             -> bool, by_axis, suggested_rotation
    proc.export(mesh, format)       -> path  // binary by default; ascii is a 111 MB mistake
    proc.slice(mesh, profile)       -> time, filament, flagged_regions[], gcode?

`support` implements the slicer's criterion, never face normals:
`unsupported(z) = slice(z) − offset(slice(z−h), +d)`, `d = layer_h/tan(θ)`,
θ measured **from the horizontal** — slicers differ on the convention and 45°
is the one angle where both readings agree, so the convention has to be stated
or the criterion is ambiguous everywhere else. The
critical angle is `atan(layer_h / (0.5·width))` — 45° only because 0.2 mm at
0.4 mm makes the ratio 1; at 0.1 mm the same nozzle holds 26.6°. [evidence]
`firmware`: a slicer field is a *request*; an unset extruder acceleration made
every retraction speed above 20 mm/s unreachable. [evidence] `features` (walls, islands) and `export` exist in the provider. [exists]
`fits_bed` and `slice` are the general case. [general] The ASCII-STL
incident is real. [evidence]

### Layer 5 — claims

    claim.assert(name, expr, range, because)
    claim.verify(h)            -> [{name, value, status, inputs[], weakest_input}]
    claim.stale(change)        -> [names]
    claim.dominance(name)      -> [{input, sensitivity, share_of_variance}]
    claim.fanout(input)        -> chains[] this symbol serves, flagged when unrelated
    claim.provenance(input)    -> measured | literature | assumed | derived, confidence

This is the product; see §06. `fanout` guards one number doing several jobs —
twice in a month in the harness, and the log caught its own recurrence. [evidence]

### Layer W — workflow (new)

    project.parts(h)               -> [{name, module, exported_at, stale: bool}]
    project.stale(h)               -> exports, claims, renders whose source hash moved
    job.start(call)                -> job_id          // anything over a few seconds
    job.status(job_id)             -> running|done|failed, progress, result?

- **Parts are first-class.** [exists] The harness used `show_*` flags and a
  hand-maintained export loop. [evidence]
- **Long jobs never block.** [general] A full overhang scan took 76 s; a timeout
  is a lost turn and a tempting retry. [evidence]
- **Everything derived is regenerated from source.** [evidence] [exists: cache manifest] A hand-copied
  test file went stale and a check passed against the wrong geometry. Cache by
  source hash; a hit says so.

## 06. Three things that are not a wrapper

### 1. A design is a test suite

    claim.assert("clip_clamps",
      mech.beam("cantilever", E, 8.9, 10, spring_t, {delta: spring_pre}).force,
      range: [2.0, 5.0], unit: "N",
      because: "below 2 N the blade lifts while cutting; above 5 N the clip is "
               "hard to seat and creeps under permanent load")

`claim.verify` on every edit turns twelve stale comments into twelve guards. The
single change that would have prevented the most damage. [evidence]

### 2. Staleness propagation

    > proc.declare({material: "PETG", E: 2000})
    6 claims invalidated by E: 3500 -> 2000
      clip_clamps        3.17 N -> 1.81 N     FAILS range [2.0, 5.0]
      clip_stress       20.9 MPa -> 11.9 MPa  ok (margin improved)
      arm_squeeze_rate  17.0 N/mm -> 9.7 N/mm  ok
    2 inputs are now literature values with no measurement: E, yield

That interaction is the product. Everything else is plumbing.

### 3. Dominance, not just value

`claim.dominance("clip_clamps")` → thickness at cubed sensitivity, slot
clearance at a 3.54× lever, modulus linear — and flags thickness is not a
multiple of layer height. One report; three bugs caught before a print, all
found here by accident, late. [evidence]

## 07. The envelope

    { status:  "ok" | "empty" | "refused" | "failed",
      value:   ...,
      subject: { bbox, hash },                       // what was actually measured
      provenance: { model_hash, params_hash, kernel, timestamp },
      controls:   { ran: N, bit: N },
      warnings:   [...] }

- **`empty` is a value, never an absence.** Seven independent incidents.
  [evidence] The kernel itself breaks the rule, and worse in 2D than in 3D:
  an empty 3D result says "Current top level object is empty", an empty 2D one
  says "Current top level object is **not a 2D object**" — emptiness reported
  as a type error — and *both* exit 0 and write no file. [evidence] The
  provider does this. [exists] MEASURED 2026-11-16, and with one narrowing:
  the provider reports both as the same `empty_output` hint with zero errors,
  so it does report emptiness rather than passing silently — and OpenSCAD's
  *2D type error* does not survive the boundary. A caller that needs to tell an
  empty 2D result from an empty 3D one cannot do it through this provider.
- **A pass must not share its representation with a null.** The overhang scan
  reported PASS on a part 80 mm from where it looked. Hence `subject.bbox`; a
  verdict with zero-extent subject is `refused`. [evidence]
- **`failed` never returns a path.** [evidence]
- **Provenance is mandatory.** A number cannot outlive its source. [evidence]
- **Probes self-test.** Known-positive control beside every query; report
  whether it bit. Caught a mutation driver that applied no mutation. [evidence]
- **View state is pinned.** An `explode` parameter is a live input to every
  measurement through that file; entry points pin it to zero. [evidence]

## 08. What it must refuse

- Provenance is typed and propagates: a derived claim inherits its *weakest*
  input's provenance. The server answers "what would I measure to trust this" by
  naming the input with the largest sensitivity × uncertainty. [evidence]
- Refuse to rank orientations without loads; to call a region self-supporting
  without a layer height; to report clearance adequate without a process
  tolerance. [evidence]
- Refuse to report a commanded value as achieved: return asked, reachable, and
  the governing limit. [evidence]
- Refuse to export a non-manifold or multi-body mesh as one part without saying
  so. [general]
- Refuse to measure a model whose render produced warnings the caller has not
  seen. [evidence]

## 09. Evidence: the seven failure classes

| Class | Shape | Sharpest incident | Killed by |
|---|---|---|---|
| A · unaskable | the model knew; no way to ask | 0.22 mm of air under a leaf, found by hand-slicing | 1, V |
| B · lying instrument | a plausible number from nothing | four probes read 52.800 mm³ from a stale file | envelope |
| C · stale claim | derived number, invalidated elsewhere, still current | six comments wrong at once on "this is petg"; draft 1 of this doc | 5 |
| D · unmodelled relationship | pairs of parts, or parts over time | 3.54× lever, 0.15 mm slack → zero clamp; frame as travel stop, zero interference at rest | 2, 3 |
| E · unowned process input | a value outside the model governs its mechanics | 4.5 layers → 4, 30% of clamp; an unset firmware limit | 4 |
| F · overloaded parameter | one symbol, several unrelated jobs | `thr_clear`: retention negative by 0.10 mm, part looked right | 5 |
| G · coincident boundary | two correct features whose positions collide | sockets at exactly the mouth cut's half-angle; a rectangle footed on a cylinder | 1b |

Full incidents live in the two logs. One is the whole argument in miniature:
checking whether draft 2 already stated the empty-is-a-value rule, a grep
returned zero hits for text read seconds earlier — `\|` is a literal in ERE. My
own instrument reported absence, confidently, four times, in the session about
instruments reporting absence.

## 10. How to falsify this

1. **A second harness produces classes outside A–G.** Draft 1's version partly
   fired: 60 → 106 entries on the same part produced two new classes without a
   second harness. Seven may be the shape of a hundred and six entries, not of
   the problem. Fasteners, seals, anything thermal are likely sources of H.
2. **A second agent's failures cluster differently.** Untested.
3. **Claims are too expensive to write.** Untested; cheapest experiment —
   retrofit claims onto the harness, count what `claim.verify` would have caught.
4. **General use is mostly Layer E, and Layer E is mostly unevidenced.** Every
   [general] badge is a guess about daily agentic modeling. The first week on
   files the author did not write will confirm the badges or replace half.
   Log that week the same way.
5. **The provider boundary is in the wrong place.** New in draft 4. Staleness
   propagation needs to know which geometric facts a claim consumed. If that
   requires the provider's *internals* — which faces, which CSG nodes — rather
   than its answers, Witness Marks cannot sit above an MCP surface and must
   vendor the kernel. The first claim that cannot be expressed over provider
   outputs alone is the signal.
6. **The cheaper half of the problem.** Zero logged entries record building the
   wrong thing, and the project's most expensive turn was exactly that. If
   scope drift costs more than measurement in real use, a server built to these
   seven classes solves the cheaper half with great precision. No layer here
   prevents it — an argument against reading this as complete, not against the
   layers.

---

Draft 5, 2026-11-16. Every [evidence] badge traces to a logged incident; every
[general] badge is reasoned and unfailed.

**Every [exists] badge has now been RUN.** Draft 4 said they "were read from
the provider's own API.md, not run", and that sentence is what this draft
retires. `tools/provider-audit.py` launches the pinned provider over MCP stdio
and records what it actually has; `tools/check-provider-claims.py` judges the
badges against that capture; `tests/fixtures/provider-claims.toml` holds them
as data. Measured against **OpenSCAD MCP Server 4.2.0**, protocol 2024-11-05,
12 tools, 20321 bytes of schema: **15 claims measured, 0 failed, 0 unverified**
— 13 decided from the tool surface and 2 from behavior, with the other three
badges describing the badge system itself.

Two things that measurement changed, neither of which reading could have:

- **Modes are not in the schema.** `mode` is a bare string with a default and
  no enum, so a reader of the JSON schema sees none of them. The audit asks the
  provider instead — an invalid mode draws `mode must be one of: ...` — which
  is the only account of its modes that cannot drift from its behavior. A first
  attempt that parsed the modes out of each tool's prose found 4 of `measure`'s
  9 and 2 of `check`'s 6, missing `probe`, `features` and `motion`: precisely
  the three this document's claims rest on.
- **Two servers answer to "openscad-mcp" on the author's machine**, one of them
  an unrelated 1.0.0 with 11 render-only tools. Reading that one and reporting
  on this one produced a confident, wholly wrong conclusion that the provider
  boundary was fiction. The audit therefore reconciles `serverInfo` against
  `PROVIDER.toml` and refuses before capturing anything, and refuses distinctly
  when the provider cannot be reached at all — an unreachable provider is not
  "no claims failed".

The boundary itself still has not been tested against a second part or a second
agent.

**Corrections, 2026-11-15.** A second agent audited draft 4; every claim of
theirs that could be checked was checked here before it was taken, and all of
them held. Measured on OpenSCAD 2026.10.14: `-D` appends an assignment rather
than overriding only existing names (the original claim was false); an empty 2D
result is reported as a type error and an empty 3D one as emptiness, both at
exit 0 with no file. Read from the provider's repository at `bde0b23`: the
history is 47 commits and not 100, September 2025 holds 2 of them, the gap is
five months, the maintainer has 39 of 47, two outside contributors have one
commit each, the merged outside PRs ran from the same day to 137 days, and the
forks are 17 diverged / 2 identical / 1 ahead; "beam" and "stress" do occur in
the provider, as prose in `reference.py`. Fixed from the text itself: the
provider boundary is falsified by §10 test 5 and not 6, the rediscovery gate is
test 3 done strictly rather than test 3, and the clamp *lost* 43% rather than
dropping to it.

TWO THINGS NOT TAKEN, and why. The auditor reported that the PETG figures
"back-solve to a 0.9 mm leaf, 4.5 layers": that is consistent (k = 9.05 N/mm,
δ = 0.350 mm) but not determined, since the preload is not published and
t = 0.8 mm with δ = 0.499 mm fits the same force — so it is arithmetic that
agrees, not a recovered dimension. The 4.5-layer observation is kept on its own
merit: a 0.9 mm leaf at 0.2 mm layers is not a whole number of layers, which is
the defect `proc.quantize` exists for. And the Windows `openscad.com` behavior
is carried as [general] rather than [evidence]: it was not run on Windows here.
