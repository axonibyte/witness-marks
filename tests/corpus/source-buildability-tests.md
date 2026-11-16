# Buildability checks for a CAD model
Generic tests I'd run on any model before believing it can be made.
Kept at cal's request 2026-11-07. Each one here is a mistake I actually
made on the can cutter, or one I checked for and nearly made.

## Topology / "is it one thing"
1. **Is every part a single connected solid?** OpenSCAD's `--render` prints
   `Genus:`; a NEGATIVE genus means more components than you think you have.
   `Genus: -1` caught a finger tab floating 1.2 mm from its shelf, before any
   camera angle showed it.
2. **Are two parts touching at only a point or a line?** Tangent cylinders,
   a box whose corner just grazes a curved wall, a feature placed exactly ON
   a boundary plane. These look joined and are not. Always overlap mating
   features by a real amount (0.5-1 mm), never abut them.
3. **Is the solid manifold / watertight?** `Status: NoError` vs a self-
   intersection warning. A non-manifold mesh may slice into nonsense.
4. **Does the genus match the number of through-holes you intended?** Genus 2
   on a blade with exactly 2 mounting holes is a free confirmation. An
   unexplained jump means an unintended void or a fused part.

## Printability
5. **Overhang angle of every downward-facing surface.** Anything past ~45
   degrees from vertical needs support. Check the ones you created implicitly
   (a helical thread's flanks, the underside of a shelf, a chamfer).
6. **Unsupported bridge spans.** Measure them. A closed ring with an internal
   slot asked for a ~40 mm bridge over a 1.3 mm gap here -- it would have
   printed as drool. Re-architect rather than add support.
7. **Minimum feature size vs nozzle.** A 0.7 mm slot is under two extrusion
   widths. Thin walls, small pins, fine threads: compare to 0.4 mm nozzle.
8. **Is there a print orientation where this works at all?** If every
   orientation needs support in a functional surface, the design is wrong,
   not the slicer.
9. **Trapped volumes / unprintable internal cavities** with no drain or no
   way to remove support.

## Fit and motion
10. **Clearance on every sliding or threaded fit.** Nominal-to-nominal is a
    press fit at best. Here: 0.35 mm on the thread, 0.6 mm radial on the can.
11. **Sweep every moving part through its FULL travel and check interference**,
    not just its rest position. The flexure's blade needed radial room at
    both ends of its stroke.
12. **Does a moving part collide with a part that is only there sometimes?**
    The floor carrier travels 54 mm up a bore that also contains the flexure
    arm at the top. Cap the travel below it.
13. **Will the mechanism back-drive?** A thread's helix angle vs the material's
    friction angle decides whether it holds position under load. 1.4 vs ~11
    degrees here, so it self-locks.
14. **Does friction reach the wrong part?** The can spinning on the floor
    would have unscrewed the floor. A free-spinning puck isolates it.

## Does it actually do the job
15. **Does the mechanism have the RANGE the job needs?** The first frame could
    only score 14 mm up a 123 mm can. The geometry was all correct and the
    machine was useless. Check the working envelope against real use before
    detailing anything.
16. **Can a human reach the controls?** The adjustment knob was buried inside
    the frame where no thumb fits. Model the hand, or at least ask where the
    fingers go.
17. **Can consumable parts be replaced?** The blade is the part that wears
    out; if it cannot be changed without destroying something, the design has
    failed regardless of geometry.
18. **Is the reference/ghost geometry actually the real thing?** A `%` ghost
    silently disappears in render mode, so a clearance you "checked" may never
    have been drawn.

## Loads
19. **Stress concentration at flexure roots** -- a sharp inside corner on a
    living hinge is where it cracks, especially across print layers.
20. **Is the compliant element in the right stiffness range?** Compute it:
    deflection per newton. Too stiff is unusable, too soft has no control.
21. **Which way do the print layers run relative to the load?** A flexure that
    bends across layer lines delaminates.

## Modelling hygiene that masquerades as a design fault
22. **A cutter must never share a surface with the solid it cuts.** The floor
    groove's inner cylinder sat at exactly bore_r, coincident with the bore.
    CGAL turned that into 174 fragments. Push every subtracting solid into
    the void by a real amount. This cost the most time of anything here and
    looked, at every stage, like a design problem.
23. **Long swept/twisted features degrade with TOTAL sweep, not step size.**
    One linear_extrude carrying 4104 degrees of twist shattered the boolean
    at every slice count and every depth. Build long helices as stacked
    single turns.
24. **Grip, knurl and cosmetic features must not cut functional surfaces.**
    Flutes for a fingertip, cut into the rim that carries the thread, chopped
    the thread into fragments. Separate the surfaces: a plain skirt for grip,
    the thread above it.
25. **Mating helices must be phase-aligned in the MODEL.** A real screw finds
    its own thread; a model does not. Two helices generated from different
    starting heights interpenetrate unless one is rotated by the difference.
26. **Clearance is measured from a moving part's LEADING FACE, not its
    origin.** The floor's travel limit was set from the carrier's base, so a
    16 mm tall carrier drove 10 mm into the flexure at full travel.

## Checking the checks
27. **Genus alone cannot tell "two parts" from "one fused part".** The whole
    complex satisfies g = 1 - N + sum(g_i), so a solid carrier (0) plus a
    washer-shaped puck (1) reads 0 -- and so does the pair fused together.
    Render each part ALONE and confirm its individual genus.
28. **Re-derive any check that reports a dramatic failure before acting on
    it.** A clearance test here said the blade fouled the mouth by 28
    degrees. It was comparing a half-width against a radius, which asks a
    different question; the real answer was 4.5 degrees of clearance. I was
    one step from redesigning around a bug in my own arithmetic.
29. **Size the mechanism from the JOB first, then detail it.** Both times the
    working envelope was wrong -- 14 mm of cut height, then 55 -- every
    dimension was internally consistent and the machine was useless.

## Checking a FIT (added after the thread fiasco)
30. **Count components on the exported MESH, not from genus.** Genus cannot
    distinguish one part from two (item 27). Export STL and union-find the
    triangles. That is what finally found the floating lug -- 8 vertices,
    bbox 4.6 x 4.8 x 10, which genus had reported as a perfectly ordinary 2.
31. **A resting contact reads as FUSED in a component count.** The puck sits
    on the carrier, so they share a face and count as one. Add an `explode`
    parameter and re-count with the parts separated; only then does the count
    mean what you think.
32. **Verify a mating fit by computing the INTERSECTION VOLUME of the two
    parts.** `intersection() { partA(); partB(); }` must be empty. This is
    the only honest test of a clearance; it found 7 interference blobs in a
    thread whose radial and axial clearances were both correct on paper.
33. **Interference blob POSITIONS name the cause.** Printing each blob's
    angle and radius showed all seven at exactly 60/180/300 degrees -- the
    leg angles -- which pointed straight at the real fault.
34. **When two features must register, derive one from the other's
    coordinate frame; never compute the offset by hand.** My phase formula
    agreed with itself at every height I tested and was still wrong, because
    I had assumed the wrong sign convention for the extrude's twist. Building
    the carrier's rib from the groove's own origin and trimming it makes the
    error impossible rather than merely absent.
35. **When analysis and a render disagree, the render is the evidence.** The
    arithmetic said the groove sat at 151 degrees; the section showed it at
    300. Two independent instruments disagreeing means stop computing and go
    look.

## Approximated curves (added after the thread fit)
36. **Two approximated curves that must nest do NOT nest at their nominal
    clearance.** The groove's outer arc and the rib's outer arc were 0.35 mm
    apart on paper, but they are sampled at different densities ($fn on one,
    a 5-degree polygon on the other), so locally the containing curve dipped
    inside the contained one. Give explicit clearance BEYOND the facet error,
    or sample both boundaries identically.
37. **Sweep a moving fit across its whole travel, not one position.** The
    carrier was checked clear at floor_z = 10, 25, 40 and 55, not just where
    it happened to be parked.
38. **"No output file" is not "no interference".** An empty boolean writes no
    STL, and a script that tests for the file reports success for a run that
    crashed, too. Read the tool's own words -- OpenSCAD says "Current top
    level object is empty" -- and treat a missing file as unknown. And pair the
    emptiness claim with a CONTROL that must not be empty -- the same probe run
    against a deliberate overlap. A probe that cannot produce a non-empty answer
    reports "clear" for every input, including the ones that are not.

## Travel means ASSEMBLY too (added after the thread was eaten twice)
39. **A moving part's travel includes how it GETS IN, not just where it
    works.** I filled the thread groove at the horns to give the flexure a
    root, and argued it was safe because the floor carrier never works that
    high. It does not -- but it has to be screwed down THROUGH that band to
    reach its working range. Measured: it jammed at floor_z 60-74. Sweep from
    the insertion point to the far limit, every time.
40. **Fixing one constraint can break another you already fixed.** Running
    the thread full height solved assembly; it then ate the flexure anchor.
    Filling the wall to restore the anchor re-broke assembly. When two fixes
    touch the same material, re-run BOTH checks, not the one you just
    changed.
41. **Make a repeated check a script, not a spot check.** The first travel
    test I ran by hand sampled four heights and all four happened to be in
    the clear. check_travel.sh now walks fourteen stations from the rim down
    and fails loudly. A check you perform by choosing samples will choose
    them kindly.
42. **Keep a visual confirmation alongside the numeric one.** A section
    through the disputed band shows at a glance whether the groove is
    continuous. The number says pass/fail; the picture says why, and catches
    the case where the test itself is asking the wrong question.

## MECHANISM: sweeping a motion for collisions with --animate
Written up 2026-11-07 after the floor carrier jammed on its way in.

**The model.** Drive the moving parameter from `$t`, which is 0 for an
ordinary render, so the same file still works normally:

    floor_z = $t == 0 ? floor_park : anim_lo + $t * (anim_hi - anim_lo);

**The test geometry** is the intersection of the moving part with everything
it must clear:

    intersection() { union(){ frame(); blade_stage(); } floor_carrier(); }

**The trap, and the way round it.** `--animate N` with an STL output emits
one file per frame -- but it ABORTS THE WHOLE EXPORT at the first empty
frame, and "empty" is exactly the pass condition for an interference test.
Driven: a sweep that overlaps and then clears wrote 2 of 6 files and stopped.
So add a SENTINEL -- a tiny cube parked far from the work -- so no frame is
ever empty:

    union() {
        translate([1000,0,0]) cube(0.2, center=true);   // sentinel
        intersection() { ...the real test... }
    }

Then every frame emits, and a frame is CLEAR when it contains only the
sentinel (1 component) and a COLLISION when it contains more. Driven on a
known case: 8 of 8 frames emitted, collisions flagged exactly where the two
boxes actually overlapped.

**One call, N stations** instead of N invocations of a shell loop.

**Is the STL itself useful?** For DETECTION, no -- "did the result come back
empty" is the whole answer, and OpenSCAD says so in words. For DIAGNOSIS it
is the most useful artifact there is, and three numbers off the mesh have
named the cause every time:
  * VOLUME (divergence theorem over the triangles) -- the one scalar for
    "how bad". Validated against a known 2x3x4 overlap: 24.0000 mm^3.
  * BLOB COUNT -- how many separate places it is wrong.
  * EACH BLOB'S ANGLE, RADIUS AND Z -- which named the cause every time:
    blobs at exactly 60/180/300 deg were the three legs; a blob at r=26 with
    8 vertices was a lug floating inside the bore; blobs that did not move
    when clearance changed meant it was not a clearance problem at all.

**Caveat on the blob count.** Components are found by union-find over
vertices matched on rounded coordinates. That assumes an exporter that emits
shared vertices identically; T-junctions or jitter would over-report. It has
agreed with the visible truth every time here, but it is an inference, not a
reading -- confirm a surprising count with a section render.

## Stale derived files (third instance of this class)
43. **Regenerate every derived test file from the source, every run.** A
    scratch .scad built by rewriting the model's top-level call is a SNAPSHOT.
    I edited the model, re-ran the sweep against the snapshot, and read the
    old geometry's answer -- concluding a fix had not worked when it had. Same
    class as executing a stale .pyc, and as treating a missing output file as
    a pass: in all three the artifact under test was not the thing I changed.
44. **A BIT-IDENTICAL result after a change is a tell, not a finding.** The
    interference came back as 0.6447 mm^3 before and after coning the pegs.
    Identical to four decimals means the change did not reach the geometry
    being measured -- check the pipeline before concluding the change failed.
45. **Match the test's degrees of freedom to the real motion.** Insertion was
    modelled as pure rotation about one pivot, which a hand never does. The
    intermediate grazes that produced were an artifact of the constraint, not
    of the part. Either give the search the freedoms the real motion has, or
    only trust the endpoints.

46. **An overhang's printability is three numbers, not one.** The face's tilt,
    the depth of air beneath it, and the unsupported reach. A 6-degree face
    over 0.37 mm of solid bridges fine; a 0-degree face over 15 mm of air does
    not. Tilt alone condemned the first and I nearly built supports for it.

47. **A modelled interference that encodes a preload competes with the printable
    gap.** Press part A into part B of thickness t by delta, and the air left
    under A is t - delta. That remainder must clear about two layer heights or
    A fuses to whatever is under it and stops being a spring. The preload is
    capped by the printer, not by the material's strength -- so derive it from
    the layer height and check the force that falls out, not the reverse.

48. **A retainer modelled with a clearance retains nothing.** Check the sign.
    A hold-down that sits above its part with a gap has specified free play in
    exactly the axis it was meant to constrain. A clamp is modelled
    INTERFERING; the overlap volume is the proof it will clamp.

49. **A threshold test at exactly the design limit mis-sorts.** A face built to
    be 45 degrees arrives from acos() at 44.99999 and counts as an overhang.
    Compare against the limit minus a tolerance, and histogram the distribution
    rather than thresholding it -- the histogram showed 223 mm2 sitting in the
    40-45 bucket and told me instantly that the chamfer had worked.

50. **A near-flat face can hide in an angle test as easily as a steep one.**
    Raking the leaf 6.4 degrees changed nothing in the angle column, and I
    wrote here that it had moved the part from unprintable to printable. THAT
    WAS WRONG, and #53 is why: the rake fixed the clearance and made the print
    worse. What the entry got right is that the angle column was not where the
    answer lived.

51. **A parameter's NAME is not its definition -- read the line, not the label.**
    `blade_rise = 20` sits in a section headed "Cut", so I read it as the height
    of the cut above the can's base and reported a conflict with the floor's
    travel limit, asking the owner to resolve it. Its own comment says "blade
    height above the bottom of the head": it positions the blade in the FRAME
    and says nothing about the can. There was no conflict. The real cut height
    was a quantity no parameter stated at all, which is why the name was free to
    be misread -- so when a number matters, DERIVE it and echo it, rather than
    leaving the reader to infer it from a name that resembles it.

52. **A limit and a target are different kinds of thing; don't compare them
    until you know which is which.** A travel stop (where a part may not go) and
    a construction dimension (where a part is built) can carry similar numbers
    and mean nothing to each other. Ask what moves and what is fixed at print
    time before treating a gap between them as a defect.

53. **A one-ended overhang is a cantilever, not a bridge, and it prints from
    its FREE end.** A bridge is anchored twice and spans in one layer; a
    cantilever is anchored once and grows outward layer by layer, starting at
    whichever end is LOWEST. Rake one 6.4 degrees and the free tip becomes the
    starting point, so the part builds backward into air. The number that
    decides it is not the slope but the per-layer step, layer_height / tan
    (slope): 1.33 mm at a 0.15 mm layer here, against the ~0.2 mm (half an
    extrusion width) that is actually self-supporting. Compute the step.

54. **When no shape and no orientation works, the PART is wrong, not the
    parameters.** Three constraints closed on each other: to self-support, the
    leaf had to slope past 37 degrees; to slope, it had to be short; to stay
    soft at 3 mm it had to be 0.3 mm thick, which does not print. That is not a
    tuning problem, and more parameter sweeps would never have found an answer.
    Splitting the feature into its own part dissolved all three at once -- and
    paid twice more, because a separately printed flexure can be laid down with
    its layer lines ALONG the bend instead of across it, and because the piece
    most likely to break became the piece you can reprint alone.

55. **Check that a part can be INSERTED, not just that it fits.** Seated
    clearance is one configuration out of the whole stroke. Sweep the assembly
    path and test every station -- the clip here passes between two locating
    pins with 0.60 mm a side for its entire 3.5 mm travel, and that is the
    number that matters, not the one at rest.

56. **A clearance in a joint is multiplied by the joint's lever before it
    reaches the working point.** The blade clip's root sits 3.5 mm deep and its
    leaf reaches 8.9 mm past it, so the joint amplifies by (reach + root)/root =
    3.54. Its whole working deflection is 0.35 mm, which means 0.05 mm of slot
    slack costs half the clamp and 0.15 mm -- an ordinary print tolerance --
    costs all of it. Compute amplified_slack = clearance x lever and compare it
    to the working deflection. When it is the same order or larger, the joint
    cannot be a clearance fit: make it an interference, where there is no slack
    to multiply and the uncertainty moves into insertion depth, which the
    mechanism does not care about.

57. **A load that is not along the joint reacts as a COUPLE, so check which
    face each half pushes on.** I assumed the clip pressed into its slot. It
    does not: solving it gives roof pushing DOWN at the mouth with 11.2 N and
    floor pushing UP at the back with 8.1 N, against a 3.2 N load. Both reaction
    faces must exist and both must be tight -- and the one at the mouth was the
    one my printability "fix" had sloped away.

58. **Check what a face sits on, not just that it is flat.** "Flat" lumps
    together a 10.4 mm slot roof anchored on both side walls -- a bridge, which
    prints -- and a pad with 15.1 mm of open air beneath it, which does not.
    Stab a ray down from each face and report what it lands on. And write the
    probe so material that ENCLOSES the height reads as supported: mine only
    accepted spans ending below the target, so solid surrounding the point
    reported as "nothing there" and I nearly chased a defect that was my own
    filter.

59. **Test overhangs the way the SLICER does, not with face normals.** A slicer
    slices to 2D layers and takes a boolean difference between each layer and the
    one below it, grown by d = layer_height / tan(threshold). Face normals are
    only a proxy for the local slope, and they get two things wrong that matter:
    they cannot see a BRIDGE (anchored twice, prints fine) and they cannot see
    what lies beneath. `overhang-scan.scad` in this directory implements the real
    test and is validated against hand-computed shapes -- 198.5 vs ~197 for a flat
    cantilever, 0.0 for 45 and 60 degrees, 131.1 vs 132.8 for a 30 degree cone.

60. **"45 degrees" is not a property of the model.** It is atan(layer_height /
    (0.5 * extrusion_width)), and it only equals 45 because 0.2 mm layers at
    0.4 mm width happen to make that ratio 1. At 0.1 mm layers the same nozzle
    holds a 26.6 degree overhang. Always state the layer height with the angle.

61. **An unsupported region can be a bridge, a droop, or an ISLAND, and only the
    third is fatal.** Area and angle cannot tell them apart. A blob on a layer
    that neither overlaps the layer below nor touches anything else on its own
    layer is molten plastic extruded into free space -- it does not sag, it
    fails. Test it directly: per layer, find connected components and check each
    one overlaps the previous layer. I turned a printable 10.5 mm bridge into a
    7.5 mm2 floating island while believing I was improving it, and every
    instrument I had -- area, angle, genus, component count on the finished
    solid -- said the part was fine. It is fine, as a SOLID. It is unbuildable
    as a SEQUENCE.

62. **A self-supporting gable must grow from the walls inward, never from the
    middle outward.** Both have 45-degree faces and both look identical in a
    section view. Only one has every layer attached to something already
    printed. When adding a "self-supporting" feature, name which existing
    material each new layer lands on; if the answer is "the layer below, which
    is also new", it is an island.

63. **Shift the grid to tell an artifact from a feature.** A rasterized check
    flagged a 0.09 mm2 island; re-running with the sampling origin moved made it
    vanish, and halving the cell kept it at exactly one cell. A real feature
    holds its area under both. One extra run turns an ambiguous positive into a
    decided one -- cheaper than either believing it or dismissing it.

64. **An unmeasured input does not fail where you wrote it.** `razor_notch_
    distance = 12.7` was a plausible-looking number nobody had measured. The
    real blade reads 6.52. That one value sets the pin positions, which set the
    clearance between the pins, which set the maximum width of the clip, which
    sets its spring rate, which sets the clamp force -- so a wrong dimension at
    the blade surfaced four steps away as a 9.83 mm3 collision between two parts
    that had both passed every check. Tag every input with where it came from,
    and treat `assumed` as a defect waiting for a trigger, not as a value.

65. **Fits must be split from functions.** The clip's thickness was doing two
    unrelated jobs: setting a spring rate (must be a whole number of layers,
    changes the mechanics) and filling a slot (must match the printer, changes
    nothing mechanical). One number cannot be tuned for both, and tuning it for
    either silently breaks the other. Split them, and put the tunable half on
    whichever part is cheapest to reprint -- here the clip, at 40 facets against
    an hour for the stage.

66. **A joint centered on the edge of a cut is half a joint.** The stage's two
    dovetail sockets sat at `horn_deg = mouth_deg / 2` = 34 deg, and the mouth
    was removed by `pie(angle = mouth_deg)`, which spans exactly +/-34. The cut
    sheared the inboard flank off both sockets. Each was one angled face with
    open air where the other belonged, so nothing captured the tenon and the
    only thing holding the stage on was its arms pressing sideways against the
    surviving flank. A 52.8 mm3 probe outboard of each flank measured it: the
    mouth-side flanks read ZERO, the outboard ones read 52.800. The defect was
    not in either feature -- both were correct -- it was in the coincidence of
    two parameters that nothing related. Derived positions that happen to
    equal a boundary are the ones to probe.

67. **A 45-degree ramp's reach is a budget, and it is usually already spent.**
    The blade's slanted edges needed material at half-width 15.9 (spine) to
    26.4 (front). What was available was the carrier's +/-9.5 plus the 3.9 mm
    of rise between the leaf tops and the blade -- +/-13.4, short everywhere.
    No amount of cleverness in the ramp fixes that: reach equals base plus
    rise, both fixed by other things. `shelf_w = 26` was the same budget being
    spent years earlier, and the blade's spine already overhung the shelf by
    2.9 mm as a result. The fix was to widen the BASE (move `leaf_y_in` from
    8.5 to 24, carrying `leaf_y_out` out with it so the leaf length and
    therefore the spring rate never changed), after which the walls are simply
    vertical and the ramp is not needed at all. When a 45-degree taper cannot
    reach, check whether you are solving the wrong end of `base + rise`.

68. **A retaining lip should BE the 45-degree plane, not sit on one.** A lip
    that reaches `rail_h` over a part to hold it down is, printed upright, a
    cantilever: 2.5 mm of flat roof at 0.15 mm layers, which droops into the
    very gap whose size is the fit. Make the lip's underside the 45-degree
    plane itself -- touching the retained part at the wall and climbing away
    inward -- and it retains by line contact while every layer oversteps the
    one below by exactly `layer_h`. Probed across the slant at 1 mm steps it
    reads 0.322 / 1.539 / 3.058 / 3.904 mm3 of a 4.000 mm3 box, the ramp
    exactly. Line contact is enough whenever the part cannot travel sideways,
    and in a wedge pocket it cannot: leaving one lip drives it into the other.

69. **"There is a window at that azimuth" is a claim about a height.**
    `arched_window` keeps full width only to its spring line, then narrows at
    45 degrees to a point. I read `leg_at = [60, 180, 300]`, worked out that
    the window between 300 and 60 spanned 313..47 deg, concluded the dovetail
    boss's ramp foot at z = 65.3 was standing on air, and added two legs to
    carry it. Measured afterward: 179.910 mm3 of 180.000 under both feet with
    the ORIGINAL legs, identical, because by z = 62..68 that window has
    pinched to +/-8 mm of chord around 0 deg -- inside the mouth anyway. The
    legs bought nothing and I reverted them. A swept or tapered cut's extent
    is a function of z; reading its parameters tells you the extent at one
    height, not at yours.

70. **Entry 65 again, in a different part: one clearance doing three jobs.**
    `thr_clear` set the thread's running clearance, the rib's height
    (`bore_r + thr_depth - thr_clear`), AND the carrier's centring in the bore
    (`r_core = bore_r - thr_clear`). The carrier slid past its own thread, and
    the reason was not the thread at all:
        engagement = thr_depth - thr_clear = 0.40 mm
        lateral slop = thr_clear           = 0.50 mm
    Slop exceeded engagement, so the carrier shifted sideways and the rib
    cleared the crest with no force and no frame flex. The retention was
    NEGATIVE by 0.10 mm and the part still assembled, screwed and looked
    right. Splitting the centring out (`core_clear = 0.25`) and cutting
    `thr_clear` to 0.3 gives 0.60 against 0.25, 2.4:1. The check to carry
    forward is an INEQUALITY, not a value: any snap, rib or key must engage
    deeper than the part can wander perpendicular to it.

71. **A rectangle laid against a cylinder overhangs at its corners.** The
    dovetail boss's ramp foot was a 14 mm wide slab reaching to `body_r`. The
    wall is round, so the slab's two corners stood up to 0.65 mm outside it --
    four thin crescents of flat unsupported bottom, 7.56 mm2 on one layer,
    which no amount of looking at the ramp's 45 degrees would have found
    because the ramp was correct. It was the footprint that was the wrong
    shape. The fix has a trap in it too: pulling the foot in to
    `sqrt(body_r^2 - halfwidth^2)` alone lengthens the ramp's run without
    changing its rise and flattens it from 45 to 39.8 deg, so the foot has to
    drop by its own radial run as well. Fixing an overhang by moving material
    inward usually makes some other face shallower; check the angle after.

72. **Deleting a part deletes the reason it existed.** The free puck was not a
    spacer. Turning the can drags on whatever it stands on, and the puck took
    that drag at a 5-11 mm thrust land -- mean radius 8.4 mm, out-resisted by
    the thread 3.7:1, a margin that is purely geometric because the load and
    the friction coefficient cancel. Fused to the carrier, the can bears on
    the full top face and feeds its drag in at roughly 26 mm against the
    thread's 34 mm: about 1.3:1, which is the regime that same analysis
    already called thin enough to walk the floor while cutting. Fusing was
    the right call for assembly and it is the owner's to make, but the number
    that changed is a different number from the one anybody was looking at.

73. **A post standing in a notch has nothing underneath it.** Asked for two
    posts in the blade's notches to back up the retaining lip, and they cannot
    do that job however they are shaped -- a notch is a VOID, so anything
    standing in one has no workpiece directly beneath it to press on. The
    nearest material is `pin_clear` plus the root slack away laterally, so a
    45-degree flare on top of the post only reaches blade after 0.41 mm of
    rise and catches it at 0.56 mm of lift, against the lip's 0.15. The posts
    are excellent at the OTHER job they were asked for -- guiding, and stopping
    travel along the blade's length -- and that distinction is the point:
    locating and clamping are different functions and a feature that is
    perfect for one can be structurally incapable of the other. Ask which one
    a feature is for before sizing it.

    The dimension that sizes it is also not the obvious one. The notch is a
    KEYHOLE: 3.2 mm where it opens at the spine, 3.62 at its root. The post
    has to clear the MOUTH IT PASSES THROUGH, not the root it finishes in, so
    pin_d keys to top_width and the leftover 0.41 mm of root slack is simply
    accepted. Size a pin to the tightest section of its insertion path, not
    to its destination.

74. **The travel stop was a part nobody was checking against.** The flexure
    was designed around its own rate (0.059 mm/N, 15 MPa leaf stress) and the
    blade was positioned against the CAN (blade_bite = 0.30 mm inside the
    wall). Both correct, and the blade still would not cut, because the thing
    it actually hit was the FRAME: the cutting edge sweeps outward in angle as
    it advances, reaching +/-32.10 deg at the wall's outer surface at rest,
    +/-33.74 at 0.6 mm and +/-35.31 at 1.2, against a +/-34 deg mouth. About
    0.65 mm of usable travel, after which the jambs take the cutting load.
    frame X blade measured 0.009 mm3 at 1 mm of advance and 0.158 at 1.5, in
    two pieces, one per jamb, inside the blade's own thickness band.
    Interference at rest was ZERO -- every static check passed. A moving part
    has to be checked against every neighbour THROUGH ITS TRAVEL, and the
    sweep is the test, not the assembled position.

    Note what made it invisible: the blade's ends are at radius 44.5, outside
    the wall's 38.6, so the obvious reading is that they clear. They do. What
    fouls is the stretch of edge that crosses the wall's radial BAND, and the
    angle it reaches there is a function of how far in the blade has come.
    Clearance between two parts is a question about the radius at which they
    coexist, not about their extents.

75. **Bound a cut with an existing feature, not a number.** Opening the mouth
    needed a new limit, and the choice that needs no justification later is
    the outermost corner of each dovetail socket's outboard flank -- a face
    that is already there for another reason, and above which the wall has no
    remaining job because the tenons end at that height. +/-51.57 deg rather
    than a figure somebody picked, and it is generous by construction: the
    edge could advance to x = 24.0 before reaching it, against a can at 33.0.
    The cut also reduced the frame's own unsupported area in that band
    (212.49 -> 188.83 mm2 over 130 layers, worst layer unchanged at 2.116),
    because removing material from above can only leave flat tops and
    vertical jambs -- a widening never costs supports, which is worth
    remembering when weighing one against a redesign.

## A part's own features can destroy each other (added after the stage's dovetails)

76. **Intersect a keyed feature with the REST OF ITS OWN PART, not only with
    the part it mates.** #32 says a fit is proven by `intersection()` of the
    two parts, and it is, but it only watches the gap between them. Within one
    part every solid is unioned, so an overlap there is normally the thing you
    want -- and that is exactly why a load-carrying stub can swallow a
    functional feature in silence. A bridge was added to tie each male dovetail
    out to a yoke; it began at the dovetail's own DEEP end and was as wide as
    the socket with no clearance, so it was wider than the tenon at every
    radius. Measured `intersection() { stage_tenons(); stage_yoke(); }` =
    744.6 mm3, which was 100.0% of both tenons. The male dovetail did not
    exist: it was a plain 9.0 mm bar, and a 9.0 mm bar cannot pass a 6.0 mm
    socket mouth, so the part could not be assembled at all.

    What made it invisible is worth more than the instance. `stage_tenons()`
    rendered alone was perfect, and `intersection() { frame(); stage_tenons(); }`
    was EMPTY -- the designed fit, passing. Every check aimed at the dovetail
    said the dovetail was right, because each one looked at the module that
    draws it rather than at the assembled part. The test that bites is
    `intersection() { frame(); whole_part(); }`, which came back 101.2 mm3, and
    then one intersection per module of the part to name which module did it.
    Add it to the checks for any feature whose JOB is its shape: a key, a
    dovetail, a snap, a locating flat, a clearance notch.

77. **A flat face stopping against a concave cylinder touches at its CORNERS,
    so the gap at the face center is not the gap.** #26 fixed which face a
    clearance is measured from; this is which POINT on that face. The press
    stop was a cube reaching out to the yoke's inner bore, its outer face set
    tail_gap = 0.515 mm short of the wall. Its corners, grip_w/2 = 6 mm off
    axis at radius 66.9, were only 0.244 mm off it -- the sagitta,
    r - sqrt(r^2 - (w/2)^2) -- so the stop fired at roughly half the intended
    travel and then wedged on two points, measured 0.50 mm INSIDE the wall at
    full press. The sign is what to remember: a convex face into a concave
    bore loses clearance at the edges, the other way round gains it.

    The fix is geometric rather than numerical: trim the stop's outer face with
    a cylinder concentric with the bore (`intersection()` of the block and
    `cylinder(r = bore - gap)`), and the gap is then identical at every point
    across the width. Re-derive the number afterward from a sweep -- the arc
    tail advanced 0.326 mm of radius per degree of press, giving 0.723 mm for
    the stroke, and it then engaged at 2.209 deg against a 2.221 target.
    Do not keep a clearance that was computed against the wrong contact point
    just because the new face is in the right place.
