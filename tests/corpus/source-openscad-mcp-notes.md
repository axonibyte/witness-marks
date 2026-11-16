# openscad MCP: annoyances and what I'd build instead
Running list, kept at cal's request 2026-11-07. Newest appended at the bottom.

## Annoyances actually hit, in the order I hit them

1. **An artifact `path` I could not open.** Every tool returns
   `artifact.path` pointing inside the service's `PrivateTmp` namespace.
   Nothing outside the service can read it, and the server declares no MCP
   resources. The single most expensive wrong turn of the session: I went
   looking for a file-serving/tunnel solution for a problem that was a
   boolean away from solved.

2. **The capability that solved it was undiscoverable.** `inline_artifact_bytes`
   existed the whole time and no tool description mentioned it --
   `render_preview` said only "Preview options (imgsize, camera, projection,
   mode)". Four live sessions were on that box; I doubt I was the only one
   stuck.

3. **`ok: true` on a render of nothing.** A missing `include` is a WARNING to
   OpenSCAD, exit code 0, so `ok = exit_code == 0 and output exists` called it
   success and handed back a 4 KB PNG of an empty scene. Worse variant found
   later: source with a bad include AND a valid `cube(10)` returned a real STL
   and an ordinary-looking picture -- a confident answer to a different
   question.

4. **`include`/`use` by path can never work** and nothing says so. The server
   is in its own filesystem namespace; it cannot see the caller's disk. The
   failure mode is a silent warning, not an error (see 3).

5. **`--viewall` is forced into every invocation**, so the `camera` distance
   argument is inert. I could not zoom in on a 0.2 mm micro-bevel; I had to
   model a cut-away instead.

6. **`--preview=throwntogether` is the default and is not labelled.** In that
   mode OpenSCAD draws *subtracted* solids in green rather than removing them.
   I described holes and notches from an image that was showing the cutting
   cylinders, not the voids. A caller who does not know the flag will
   confidently misread every `difference()` in the model.

7. **No way to render the file under discussion.** Every call re-sends the
   whole source. For a 318-line file being iterated on, that is ~3k tokens a
   look, and worse, retyping risks verifying a model that is not the one on
   disk. I fell back to a local OpenSCAD for iteration purely for fidelity.

## What I would build instead

- **Return the image, always, by default.** An artifact the caller cannot open
  is not a result. Inline images; keep a size cap; let `export_model` opt in,
  since an 84 KB STL as base64 is context arson.
- **Make `ok` mean "the thing you asked for is what I rendered."** Exit code is
  not enough. Promote missing-include / missing-library to a failure and emit
  no artifact. Keep ordinary warnings non-fatal -- that distinction is the
  whole value.
- **Name the render mode in the result and let it be chosen per call.** Default
  to the true geometry (`--render`), not the CSG preview. If preview is
  returned, say so in the payload where a reader will see it.
- **Let the camera be specified fully**, including "fit" as an explicit option
  rather than an always-on flag that silently overrides distance.
- **Give the server a document to work on.** A `put_source`/`render_ref` pair,
  or a declared resource root, so a caller can iterate on one artifact by
  reference instead of re-uploading it. Content-addressed is fine: upload once,
  render many.
- **Declare artifacts as MCP resources** so a client that *wants* the file can
  fetch it, rather than being handed a path into a namespace it cannot enter.
- **Say the constraints in the tool description.** "This server cannot read
  your filesystem; `include` resolves only against X" would have saved the
  single largest detour here.

## Added while designing the cutter (2026-11-07, later)

8. **The most valuable signal in the whole tool is buried in stderr.** The
   `--render` pass prints:
       Top level object is a 3D object (manifold):
          Status: NoError
          Genus:  -1
   `Genus: -1` is how a DETACHED part announces itself -- two components
   instead of one. It caught a floating finger tab for me before any render
   angle showed it, and `Genus: 0` afterwards proved the body+arm is a single
   printable solid. This belongs in the structured result as
   `{manifold: true, components: 1, genus: 0, vertices, facets}`, not as free
   text a client has to regex. For a design agent it is the single best
   cheap check available, and nothing advertises it.

9. **`%` (background/ghost) geometry silently disappears in `--render`.** So
   "show me the part against the can for clearance" and "show me the true
   solid" cannot be the same call. Nothing says so; the can just isn't there
   and you wonder whether your transform is wrong. An MCP should either keep
   ghosts in render mode or say in the result which modifiers were dropped.

10. **Forced `--viewall` makes the `camera` distance argument a lie.** You can
    pass a distance; it is overridden. To inspect a 0.2 mm feature you must
    model a cut-away instead of zooming. Fit-to-view should be an explicit
    option, not an always-on flag.

11. **No section/cut facility.** Checking an internal collision meant
    `sed`-ing the top-level call into an `intersection()` with a slab, on a
    scratch copy of the file. A `section: {axis, at, thickness}` option would
    make the single most common spatial check a parameter instead of a hack.

12. **Nothing measures.** Every dimensional question -- does the edge reach the
    can, how wide is the slot window, how far must a layer bridge, is the tab
    touching the shelf -- I answered in a separate Python process, because the
    tool returns pictures and the picture is exactly what you cannot trust for
    spatial reasoning. A `measure` call (bounding box, min distance between two
    named subtrees, interference volume) would do more for an LLM's spatial
    competence than any number of extra camera angles.

## Added while designing the cutter (continued)

13. **`-D` only overrides variables that ALREADY exist in the file.** A
    variable introduced solely on the command line is silently "unknown" and
    the expression using it evaluates to undef, which then propagates into
    geometry as a degenerate shape rather than an error. I lost a probe cycle
    to it. An MCP wrapping this should either pre-declare `params` or refuse
    when a passed parameter is not present in the source.

14. **The geometry summary only appears for some export targets.** Exporting
    OFF to a file printed no `Genus:` line; exporting PNG did. So the single
    most useful diagnostic is coupled to the output format by accident. A
    tool should always return it.

15. **There is no "is this one piece?" question I can ask.** I had to infer
    component count from a genus number, and the inference is ambiguous (see
    the testing list, item 27). `components`, `manifold`, `volume`,
    `bounding_box` as structured fields would replace a lot of guesswork.

16. **No interference query.** Checking whether two parts fit meant writing a
    scratch file that intersects them, exporting STL, and union-finding the
    triangles in Python to count blobs and locate them. `interfere(a, b)`
    returning volume and the centroid of each overlap region would have
    turned a forty-minute hunt into one call -- and it is the single question
    an assembly needs answered most often.

17. **No component count.** See above; `components` as a field would have
    caught a floating part immediately. I only found it because cal looked at
    the picture and asked.

18. **Empty results are indistinguishable from failures.** An empty boolean
    produces no output file and a line on stderr. A wrapper that checks for
    the file cannot tell "correctly empty" from "crashed", and I wrote that
    bug twice in one session. The result should carry `empty: true` as a
    fact, separately from `ok`.

19. **No parameter sweep.** Checking a mechanism through its travel means
    re-invoking the whole render per station from a shell loop, re-parsing
    text output each time. `sweep(param, values, check)` returning a verdict
    per station -- or just letting one call take a list of parameter sets and
    return a result array -- would make motion checking routine instead of
    bespoke. batch_render_scad does this for IMAGES but there is no
    equivalent that answers a geometric question.

20. **`--animate` aborts the whole export on the first empty frame.** Driven:
    a 6-frame sweep that overlapped then cleared wrote 2 files and stopped,
    printing "Current top level object is empty". For interference sweeps
    that is backwards -- empty IS the pass -- so the run dies at the first
    success. Workaround is a sentinel solid parked far from the work so no
    frame is ever empty. An MCP should either not treat empty as terminal, or
    expose `animate` as a proper parameter sweep that returns a result per
    frame including `empty: true`.

21. **Animation output is files-on-disk only.** `--animate` names frames
    out00000.png / out00000.stl in the working directory; nothing is
    returned. Through an MCP whose filesystem the caller cannot see (item 1),
    an animated sweep is completely unreachable. A `sweep` that returned an
    array of per-frame results -- image, empty flag, volume -- would make
    motion testing a first-class operation instead of an unusable one.

22. **No volume, no bounding box, no centroid.** Every diagnostic I rely on
    I compute myself from exported triangles: interference volume by the
    divergence theorem, component count by union-find, blob positions by
    averaging vertices. These are cheap for the server and expensive for the
    client, and they are what actually answers "is this buildable".

23. **Nothing can answer "how much air is under this face."** It is the single
    most important printability number and there is no way to ask for it. I
    had to export an STL, slice it myself at a dozen heights, and compare
    cross-sections, to learn that the gap under a leaf spring was 0.22 mm --
    a fact that inverted my whole diagnosis (it needed no support; it was
    going to FUSE). `gap_below(face)` or even `cross_section(z)` would have
    given it in one call.

24. **Every interference query costs an OpenSCAD round-trip.** My tool takes
    one pre-intersected STL, so asking "do these two parts overlap" means
    writing a scad that includes the model, calling two modules by name,
    rendering, exporting, and only then measuring. An
    `intersect(module_a, module_b) -> {volume, bbox, regions}` call would
    collapse that to one step and remove a whole class of my own errors --
    today I handed the measuring script two files, it silently read only the
    first, and reported the stage's own volume as the overlap.

25. **`include` of an absolute path works locally and not through the server.**
    The split namespace means the one trick that makes a model introspectable
    from a scratch file -- include it, then call its modules -- is available
    to me over SSH and not over the MCP.

26. **OpenSCAD writes NO FILE for empty geometry, and exits 0.** This is the
    single worst instrument failure available here, because the previous run's
    file is still sitting there and reads perfectly. Four flank probes
    confidently reported 52.800 mm3 of material in a box whose radii put it
    outside the body wall -- all four were a stale STL. The exit status does
    not distinguish "empty result" from "wrote a result", so a harness must
    delete the target before every run and treat absence as zero, which is
    also the only way the pass condition ("no overhangs") can be read at all.
    An MCP call must return `{volume, triangles, empty: true}` as a VALUE and
    never express emptiness as the absence of an artifact.

27. **`intersection()` ignores an empty operand instead of annihilating.**
    `intersection() { target(); box(); }` where `target()` renders nothing
    returns the BOX. Not empty -- the box. So a probe whose target silently
    failed to render hands back the probe's own volume dressed as a
    measurement, and the number is plausible because it is exactly the full
    box you were testing for. Combined with note 26 this is two independent
    ways for the same harness to report a confident wrong answer. Any
    intersection API needs to render each operand's volume alongside the
    result, so an operand of zero is visible rather than inferred.

28. **A parameter file cannot be imported without running it.** `use` does not
    import variables, so the only way to reach a model's parameters from a
    probe is `include` -- which also executes its top-level calls. The model
    then renders into the probe's own output, and since the thing being
    measured is a subset of the model sitting beside it in the same STL, every
    intersection probe becomes a no-op: four reads of 108800.749 mm3, the
    model's own volume, four times. Every model that wants to be measurable
    needs a render gate (`show_assembly = true; if (show_assembly) cutter();`).
    An MCP that exposes `parameters(file)` and `render(file, module)`
    separately removes the whole trap.

29. **`-D` string values have to survive two parsers.** `-D which=\"frame\"`
    inside a double-quoted shell string arrives as `which=frame`, an undefined
    variable, so no branch matches and the module renders nothing -- straight
    into note 27. Needs `-D 'which="frame"'`. A typed parameter API (strings as
    strings) makes this unrepresentable; with a CLI, every string override
    should be echoed back and asserted on.

30. **The overhang scan's PASS condition is identical to its "you measured
    nothing" condition.** An empty result writes no STL (note 26), and the
    scan declares that a pass -- correctly, because no unsupported region IS
    the pass. But it is also what you get when the geometry is not where you
    scanned. I scanned an old carrier through a file whose `explode` defaults
    to 80, so `floor_carrier()`'s own `translate([0,0,explode])` put the part
    at z 110..137 while the scan looked at 0..18. "PASS: every layer is
    supported." The part was 80 mm away. Any scan has to assert its target
    actually occupies the band before a pass means anything -- the cheap
    version is to report the target's own bounding box next to the verdict,
    which would have shown z=(110.0, 137.2) against a 0..18 scan immediately.
    MORE GENERALLY: a view parameter that moves geometry (`explode`) is a
    live input to every measurement taken through that file, and it does not
    look like one. Measurement entry points should pin it, not inherit it.

31. **Scanning an assembly is not scanning the parts.** The same run flagged
    2371 mm2 on one layer -- the loose puck's entire underside, floating in
    air because it is a separate printed part sitting on top of another one in
    the assembly view. Perfectly real as geometry, meaningless as
    printability. Per-part is the only unit in which "needs supports" has an
    answer, so a buildability API should take a part, and refuse (or at least
    partition) a multi-body input rather than averaging nonsense over it.
