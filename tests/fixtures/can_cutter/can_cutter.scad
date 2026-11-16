// ============================================================
//  CAN CUTTER
//  Holds a trapezoidal utility blade against a can so the can
//  can be turned by hand and scored around its circumference.
//
//  COORDINATE SYSTEM -- everything below depends on this:
//    Z   is the can axis. The can drops in from above, stands on
//        the adjustable floor, and is turned by hand. The cutter
//        is held still.
//    +X  is the radial direction through the blade.
//    The blade lies FLAT in the plane z = blade_z, cutting edge
//    tangent to the can, body pointing radially OUTWARD.
//
//  WHY THE BLADE IS MOSTLY OUTSIDE THE BODY. The edge is a
//  STRAIGHT 61 mm line and the can is a cylinder, so it can only
//  touch at the tangent point. The tips of that edge sit at
//  sqrt(r_edge^2 + (61/2)^2) -- 44.7 mm for a 66 mm can, against
//  a 37.6 mm outer wall. The blade MUST cross the wall; only its
//  holder can live outside.
//
//  WHY THE BODY IS A C. A closed ring needs a blade slot that
//  removes a 37-64 degree window at the cut plane, leaving the
//  layer above to bridge ~40 mm unsupported over a 1.3 mm gap.
//  Opening the window full height removes the bridge and puts
//  the mouth where the arm and the squeeze tab belong anyway.
//  ~292 degrees of bore still holds a can concentric easily.
//
//  WHY IT STANDS ON LEGS. Score height on the can is
//  blade_z - floor_z. A 36 mm ring with the blade at 20 can only
//  reach 14 mm up the can, which cuts nothing anybody wants. The
//  legs put the blade 80 mm up while staying open and light, so
//  the floor can travel for scores 14-72 mm above the can bottom.
// ============================================================

/* [Can] */
can_dia        = 66;    // body diameter of the can (12 oz = 66)
can_clearance  = 0.6;   // radial slop so the can turns freely

/* [Cut] */
blade_bite     = 0.30;  // how far inside the can wall the edge sits
blade_tilt     = 0;     // SLICE ANGLE. Degrees about the RADIAL axis (+X)
                        // through the contact point, so the cutting edge
                        // leaves the horizontal while staying TANGENT.
                        //
                        // WHY NOT ABOUT Z, which is the obvious reading of
                        // "turn the blade 45 degrees": tangency IS
                        // perpendicularity to the radius, so rotating a
                        // straight edge in the horizontal plane stops it
                        // being tangent and makes it a CHORD. Measured on
                        // this geometry: 10 deg puts the edge line 0.80 mm
                        // inside the wall, 30 deg puts it 4.68, and 45 deg
                        // puts it 9.88 -- against a 0.30 mm bite. An in-plane
                        // rotation does not score at a point, it drives a
                        // deep chord, which is the crush it was meant to
                        // avoid.
                        //
                        // WHY ABOUT X: the edge direction becomes
                        // (0, cos t, sin t) and the can's surface velocity at
                        // the contact is (0, v, 0), so the angle between them
                        // is exactly t. That is the slice angle -- and
                        // tangency, single-point contact and blade_bite are
                        // all untouched, because a line tangent to a cylinder
                        // and inclined still touches at one point.
                        //
                        // WHY 30 AND NOT 45: at layer_h = 0.15 and a 0.4 mm
                        // nozzle the self-supporting limit is
                        // atan(0.15/0.2) = 36.9 deg, not the 45 this file's
                        // comments assume. 45 would also take the low-side
                        // lip to 15 deg. See the LIP note in lip_void().
blade_rise     = 20;    // blade height above the bottom of the head

/* [Frame] */
wall           = 5.0;   // wall thickness. Was 4.0, which left only
                        // wall - thr_depth - thr_clear = 2.6 mm of material
                        // behind the thread groove, and the frame measurably
                        // flexed: the C can spread, and when it spreads the
                        // carrier's rib leaves its groove. Local bending
                        // stiffness goes as thickness cubed, so 2.6 -> 3.8 is
                        // 3.1x. Costs 4 mm of outside diameter.
mouth_deg      = 68;    // angular opening of the C, centred on +X
base_t         = 5;     // thickness of the base ring
leg_h          = 65;    // Sized from the JOB, and from cal's observation that
                        // the can can go in UPSIDE DOWN. A cut h mm from the
                        // can's bottom is (123-h) from its top, so a fixture
                        // reaching R covers 0..R upright and (123-R)..123
                        // inverted -- no gap once R >= 61.5. Reaching 65 mm
                        // therefore covers every cut on a 123 mm can, and
                        // saves 10 mm of frame over chasing 75 mm directly.
head_h         = 34;    // height of the cutting head
leg_deg        = 26;    // angular width of each leg
leg_at         = [60, 180, 300];   // leg angles -- all clear of the mouth
                        // These were briefly moved to [45, 110, 180, 250, 315]
                        // to put a leg under each dovetail boss, on the theory
                        // that the boss's ramp foot (z = 65.3, below
                        // head_z0 = 70) landed in a window. MEASURED: it does
                        // not. arched_window narrows at 45 degrees above its
                        // spring line, so by z = 62..68 the window between
                        // legs 300 and 60 has pinched to +/-8 mm of chord
                        // around 0 deg -- inside the mouth anyway -- and the
                        // wall at +/-45 deg is solid. A 90 mm3 probe under
                        // each boss foot read 179.910 of 180.000 with BOTH leg
                        // sets, identical. The change bought nothing.

/* [Compliant stage] */
leaf_t         = 1.2;   // leaf thickness, in the squeeze direction
leaf_h         = 15;    // leaf height
leaf_x         = 42;    // radial station of the leaves. Was 48, which is
                        // under the blade platform now -- the leaf's root
                        // has to land somewhere the platform actually is,
                        // and at x = 42 the platform reaches +/-25.9.
leaf_y_in      = 24;    // where a leaf meets the carrier. THIS NUMBER SETS
                        // THE CARRIER'S WIDTH, and the carrier's width is
                        // what decides whether the rails can reach the
                        // blade's slanted edges at all. At the old 8.5 the
                        // carrier was +/-9.5, a 45 deg ramp over the 3.9 mm
                        // between leaf_top and the blade reached +/-13.4,
                        // and the edges sit at 15.9 (spine) to 26.4 (front).
                        // Short everywhere. It is also why shelf_w was 26:
                        // +/-13 was all a ramp could buy, and the spine
                        // already overhung it by 2.9 mm.
leaf_y_out     = 46;    // where it meets the stand-off arm. Moved out with
                        // leaf_y_in so L stays 22 mm -- the spring rate is
                        // unchanged, the whole flexure just sits outboard.
arm_t          = 4.0;   // rigid stand-off arms, from horn to leaf
boss_out       = 3.5;   // how far the anchor boss stands proud of the body

/* [Stage mount -- the stage is a SEPARATE, REPLACEABLE part] */
dt_at          = 45;    // AZIMUTH OF THE TWO DOVETAILS, and the whole reason
                        // the stage would not stay on. This used to be
                        // mouth_deg / 2 = 34 deg -- dead on the mouth's lip --
                        // and pie(angle = mouth_deg) spans exactly +/-34, so
                        // the mouth cut sheared off each socket's inboard
                        // flank. Every socket was HALF a dovetail: one angled
                        // face, open air where the other belonged. Nothing
                        // captured the tenon; the only thing holding the stage
                        // on was the arms pressing sideways against that one
                        // surviving flank, and they do not press hard enough.
                        //
                        // 45 deg clears it. The socket's widest corner sits at
                        // dt_at - atan((dt_deep/2)/(socket_r - dt_depth)) =
                        // 38.3 deg, leaving 4.3 deg -- 2.9 mm of material at
                        // that radius -- between the flank and the mouth cut,
                        // and 6.3 mm on the outboard side. Both flanks solid.
dt_depth       = 3.0;   // radial depth of the dovetail
dt_open        = 6.0;   // its width at the mouth
dt_deep        = 9.0;   // and at the bottom -- wider, so it cannot pull out
dt_clear       = 0.35;  // slip fit between tenon and socket
dt_pad         = 2.0;   // socket floor, so the stage lands at a known height
shelf_t        = 2.4;   // thickness of the shelf the blade lies on
tab_w          = 16;    // width of the push tab
tab_l          = 14;    // height of the push tab

/* [Shelf] */
shelf_in       = 38;    // inner edge of the blade shelf (must clear the bore)
shelf_out      = 57;    // outer edge -- runs BEHIND the blade's spine
tab_in         = 51.9;  // the tab's front face IS the backstop: it sits
                        // 0.2 mm outboard of the spine and takes the cutting
                        // load, because the notches cannot (they open outward)

/* [Stage kinematics -- a LEVER, not a parallel flexure] */
// WHAT CHANGED AND WHY. The old stage was a PARALLEL flexure: two leaves of
// equal stiffness, so squeezing translated the blade straight in and the whole
// 61 mm edge arrived at the can together. That is the geometry that crushes --
// a tangent line meets a cylinder at one point, but a line driven bodily
// inward becomes a chord along its whole length at once, and the wall has
// nothing to push back with.
//
// This one is a LEVER. One side is a living HINGE; the other is free and
// carries the pad you press. Pressing swings the carrier about the hinge, so
// the blade ROTATES into the can: the edge enters at ONE point and the bite
// grows with the angle instead of arriving all at once. The hand force is
// multiplied by the lever ratio, and a hard STOP sets the deepest bite the
// mechanism can reach, so the crush is designed out rather than avoided by
// care.
//
// IT DROPS INTO THE EXISTING FRAME. Both dovetails are unchanged -- same
// stage_tenons(), same sockets at +/-45 deg, same stage_z0 -- so the frame
// does not get reprinted. All of the change is outboard of the tenons.
// EVERY NUMBER BELOW IS FROM THE CARRIER'S OWN PLAN, not chosen.
// Its back corners sit at (57.0, +/-18.42), radius 59.90, azimuth +/-17.9 --
// so the hinge goes on ONE of those corners, the thumb post on the other, and
// the yoke has to stand off far enough to leave the hinge somewhere to be.
// WHICH SIDE THE HINGE SITS ON IS SET BY WHICH WAY THE CAN TURNS.
// cal turns it COUNTER-CLOCKWISE seen from above, so at the contact the can's
// surface runs in +Y and drags the blade that way. The hinge is always
// radially OUTBOARD of the contact, so (C-H)_x is negative whichever corner it
// is on and the drag torque's SIGN NEVER CHANGES -- the only thing that
// chooses bite-from-release is H_y, because the bite rate is exactly H_y.
//   hinge at -17.9: bite needs +phi, drag drives -phi -> the cut RELEASES and
//                   you fight the drag for the whole turn.
//   hinge at +17.9: bite needs -phi, drag drives -phi -> the drag CARRIES the
//                   blade in and holds it against the stop.
// So it goes on the +Y corner, and the TAIL STOP stops being a safety net and
// becomes the thing that sets the depth: pinch to start, turn, and the cut
// holds itself at bite_stop instead of at whatever your hand is doing.
hinge_at       =  17.9; // the +Y corner: the drag feeds the cut
press_at       = -17.9; // the free corner, where the grip posts stand
hinge_t        = 1.4;   // hinge plate thickness: the ONE compliant direction
hinge_l        = 7.0;   // radial length, which IS the yoke stand-off gap
hinge_r        = 63.4;  // midway across that gap
yoke_t         = 5.0;   // the rigid yoke that ties the two tenons together
yoke_r         = 71.9;  // inner face 66.9, which clears the 59.90 corner by
                        // the 7 mm the hinge needs
pad_t          = 6.0;   // thickness of the push pad
bite_stop      = 0.80;  // THE DEEPEST BITE THE MECHANISM CAN REACH, and with
                        // the hinge on the feeding side it is no longer a
                        // safety net -- it is what SETS the depth, because the
                        // drag carries the blade in until the tail lands.
                        //
                        // WHERE 0.80 COMES FROM, which is the can and not a
                        // preference. A 12 oz aluminium wall is about 0.10 mm.
                        // Pinching pushes the can away until it bottoms on the
                        // far side of the bore, so its near surface RETREATS by
                        // can_clearance, from 33.00 to 32.40 -- and the edge at
                        // rest (32.70) is then 0.30 mm SHORT of even touching.
                        // So 0.30 of advance to touch, 0.40 to be through, and
                        // 0.80 leaves 0.40 mm of margin for a can that is not
                        // round, not centred, or not 66.00 across.
                        //
                        // AND WHY NOT MORE. At 0.80 the edge reaches radius
                        // 31.90: through the wall with margin, and if that can
                        // happens to be sitting centred instead, buried 1.10 mm
                        // -- survivable. At the 1.80 this started at, a centred
                        // can is buried 2.10 mm, which is the blade's wedge
                        // jammed through the slit, tearing rather than cutting.
                        // Measured: 0.80 mm of advance is 2.221 deg of lever.
press          = 0;     // PREVIEW ONLY: degrees of lever travel to draw. 0 is
                        // at rest with the edge just touching.

/* [Blade groove -- a WEDGE, not a clip] */
// The blade is a trapezoid: a 61.0 mm cutting edge, a 31.8 mm spine, and two
// slanted ends at 37.54 deg to the radial. It slides in from the FRONT (the
// can side) straight out along +X into a pocket of its own shape, and the two
// slanted walls pinch it. Pressure fit, nothing on top of it, no separate part.
//
// The load direction is the locking direction: cutting pushes the blade
// outward, which is further INTO the converging wedge. Same logic as the
// dovetail. Pulling it back out toward the open mouth is the only loose
// direction, and only friction holds there -- which is what you want, because
// that is how you change the blade.
layer_h        = 0.15;  // THE SLICER'S LAYER HEIGHT, declared here on purpose:
                        // every overhang rule below is layer_h / tan(45), so
                        // the geometry has to know it.
rail_t         = 2.0;   // wall thickness outboard of each slanted edge
lip_deg_min    = 40;    // the lip's dip must stay above the SELF-SUPPORTING
                        // limit after the tilt. At layer_h 0.15 and a 0.4 mm
                        // nozzle that limit is atan(0.15/0.2) = 36.9 deg, so
                        // 40 carries 3 deg of margin. See lip_deg.
rail_h         = 2.0;   // how far the rail stands above the blade's top face.
                        // At 45 deg this is ALSO how far the retaining lip
                        // reaches inward over the blade, because the lip's
                        // underside IS the 45 deg plane -- see lip_void().
blade_lift     = 0.15;  // Z clearance over the blade. One layer: enough to
                        // slide, not enough to rattle or to climb the lip.
pin_clear      = 0.20;  // clearance in the notch MOUTH, which is the narrow
                        // point of the path: the notch is a keyhole, 3.2 mm
                        // wide where it opens at the spine and 3.62 at its
                        // root, so the post has to be sized to the mouth it
                        // passes through, not the root it ends up in. That
                        // leaves 0.41 mm of radial slack in the root -- the
                        // posts are a GUIDE, and the wedge is the locator.
pin_stand      = 0.30;  // how far the post's cylindrical part clears the blade
                        // before the flare takes over
blade_press    = 0.05;  // NET interference per slanted wall, after the
                        // printer's own narrowing is taken out. This is the
                        // whole fit. 0 is a slip fit; go up in 0.02 steps.
slot_comp      = -0.09;  // how much narrower this printer lays an internal slot
                        // than the model says, measured on the clip-slot ladder
                        // (clip_root_144..156). It is a Klipper'd Kobra Neo and
                        // it runs slots TIGHT, which for a wedge is free press:
                        // half of it lands on each wall. blade_fit backs it out
                        // so blade_press means what it says.

/* [Floor] *//* [Floor] */
thr_pitch      = 5;     // one turn = 5 mm of floor travel
thr_depth      = 0.9;   // radial depth. SHALLOWER ON PURPOSE: the groove's
                        // ceiling is an unsupported ledge when printed, and
                        // its depth IS the unsupported dimension.
thr_clear      = 0.3;   // radial clearance, rib crest to groove root. ONLY
                        // that. It used to also set the rib's height and the
                        // carrier's centring in the bore -- three jobs, one
                        // number, and tuning it for any one broke the others.
                        // Cutting it 0.5 -> 0.3 buys engagement directly,
                        // because engagement = thr_depth - thr_clear, and it
                        // also STEEPENS the groove's flank (the taper spans
                        // thr_depth + thr_clear, so a shorter span is a
                        // steeper ramp): 30.7 deg -> 34.5 deg.
core_clear     = 0.25;  // radial gap, carrier core to bore crest. THIS is what
                        // was letting the carrier slide past the grooves, and
                        // it is not a thread dimension at all. At 0.5 the
                        // carrier could shift sideways further than the rib
                        // was deep (0.40 mm), so the rib cleared the crest on
                        // one side for free -- no frame flex required, which
                        // is exactly how it felt. Now:
                        //    engagement 0.60 mm  vs  slop 0.25 mm  = 2.4 : 1
                        // and escaping means spreading the frame 0.35 mm
                        // through a 3.8 mm wall instead of a 2.6 mm one.
carrier_h      = 17.6;  // total height of the floor carrier. 16 + the 1.6 the
                        // puck used to add on top (3 of puck, less the 2 of
                        // recess it sat in, plus the 0.6 land) -- so the can
                        // still stands at exactly the same height and
                        // cut_height does not move.
skirt_h        = 6;     // fluted grip section at its base
grip_n         = 3;     // FINGER GRIP for the top of travel. The fluted skirt is
grip_r         = 22;    // reachable through a window only while the window is
grip_d         = 10;    // still wide; above floor_z 58 the gable has closed to
grip_depth     = 4;     // under 15 mm and a fingertip no longer fits. These go
                        // in from ABOVE instead, through the open bore, and work
                        // at every height. The PUCK gets three holes and the
                        // carrier gets a concentric GROOVE rather than matching
                        // holes -- the puck spins free, so holes would have to be
                        // hunted into alignment; a groove is aligned by symmetry
                        // whatever angle the puck happens to be sitting at.
                        // Full-face, the mean radius was 20.9 mm and the thread
                        // only out-resisted it 1.5 to 1 -- thin enough that
                        // cutting could walk the floor. On a 5-11 mm ring the
                        // mean radius is 8.4 mm and the margin is 3.7 to 1.
                        // The margin is geometric: load and friction coefficient
                        // cancel, so it does not depend on how hard you press.
                        //
                        // The top is no use with a can standing on it, so the
                        // same three positions bore straight THROUGH to the
                        // carrier's underside, which a finger reaches through a
                        // window at every height the top is not: the window is
                        // always wider at the carrier's bottom face than at its
                        // top. Bored through, not blind -- a blind divot has a
                        // flat roof, and three of them would have put 236 mm2
                        // of overhang in the middle of the part. Through, every
                        // wall is vertical and there is no roof to angle.
seam_ov        = 0.25;  // overlap between stacked turns of the helix
nut_deg_mouth  = 200;   // groove width at the bore face
nut_deg_root   = 80;    // and at its deepest -- the taper between them
                        // puts the flank at 42.8 deg from horizontal,
                        // instead of the 1.4 deg of a square groove
thr_play_deg   = 43;    // how much narrower the carrier's rib is than the
                        // groove: 43/360*5 = 0.60 mm of axial give
floor_z        = 30;    // CURRENT floor height -- turn the knob to change

/* [View] */
show_can       = true;
show_blade     = true;
show_frame     = true;  // the printed frame
show_stage     = true;  // the blade stage -- its own printed part
show_floor     = true;  // the carrier (the puck is fused into it)
explode        = 80;     // PULL THE LOOSE PARTS CLEAR for slicing. Set to 80
                        // and the carrier and puck rise above the body while
                        // the blade stage withdraws RADIALLY -- along the axis
                        // its dovetails actually slide on. Lifting the stage
                        // was not enough: it never intersected the frame, but
                        // it stayed nested inside the frame's sockets, so a
                        // slicer could not pick it out to lay it flat.
stage_out      = 0.80;  // stage's radial withdrawal per unit of explode
stage_up       = 0.25;  // and its lift, so the view still reads as exploded

// RESOLUTION. $fn = 64 was a flat facet count: 64 facets on the 37.6 mm body
// (a 3.7 mm chord) and the same 64 on a 1.5 mm pin, which is both too coarse
// where it shows and wasteful where it does not. $fa/$fs drive facets from
// curvature instead -- 120 on the body, 8 on the pin. Drop $fa to 1.5 for a
// final export; it roughly doubles render time.
$fa = 3;      // max degrees per facet
$fs = 0.6;    // max facet length, mm

/* [Blade] the trapezoidal utility blade this holds */
// --- Default Physical Parameters ---
razor_blade_length        = 61.0;  // Bottom cutting edge length (mm)
razor_spine_length        = 31.8;  // Top spine length (mm)
razor_height              = 19.0;  // Total vertical height (mm)
razor_thickness           = 0.615; // Flat body thickness (mm)

razor_hole_count          = 0;     // REAL BLADES HERE HAVE NOTCHES ONLY, no holes.
                                   // The stage locates on the notches, which is what
                                   // the blade was designed for anyway.
razor_hole_dia            = 5.0;   // Mounting hole diameter (mm)
razor_hole_distance       = 12.7;  // Center-to-center hole spacing (mm)

razor_notch_count         = 2;     // Number of top notches (0 = smooth spine, 2 = standard, 3 = universal)
razor_notch_distance      = 6.52;  // Center-to-center notch spacing (mm). MEASURED
                                   // on a real SK2 blade: inner edges 2.90 apart,
                                   // notch dia 3.62, so c-c = 2.90 + 3.62 = 6.52.
                                   // The 12.7 that was here was never measured.
razor_notch_depth         = 2.5;   // Depth from top spine (mm)
razor_notch_top_width     = 3.2;   // Opening width at spine (mm)
razor_notch_root_dia      = 3.62;  // MEASURED. Was 2.0, also never measured.

razor_bevel_primary_angle = 10.0;  // Primary grind angle per side (deg)
razor_bevel_micro_angle   = 14.5;  // Micro-bevel angle per side (deg)
razor_bevel_micro_height  = 0.2;   // Micro-bevel vertical height (mm)


// ---- derived ------------------------------------------------
can_r     = can_dia / 2;
bore_r    = can_r + can_clearance;
body_r    = bore_r + wall;
r_edge    = can_r - blade_bite;
spine_r   = r_edge + razor_height;

// Where the NOTCH ROOTS sit, in the frame's coordinates. spine_notch() puts
// its root circle at blade-local y = height - depth + root_dia/2, and
// blade_in_place() maps blade-local +Y onto global +X from r_edge.
notch_r   = r_edge + razor_height - razor_notch_depth + razor_notch_root_dia / 2;
notch_y   = razor_notch_distance / 2;
pin_d     = razor_notch_top_width - 2 * pin_clear;   // sized to the MOUTH

// ---- the wedge ----
// The blade's slanted end, as a slope and an angle. Everything about the
// groove is one of these two numbers.
slant_tan = ((razor_blade_length - razor_spine_length) / 2) / razor_height;
slant_deg = atan(slant_tan);                 // 37.54 deg
// Modelled clearance per slanted wall. Printed clearance comes out at
// modelled + slot_comp/2, and we want that to equal -blade_press, so:
blade_fit = -blade_press - slot_comp / 2;
// Half-width of the blade at radial station x, and of a wall offset
// perpendicularly from it by g (negative g = into the blade = interference).
function hw_blade(x)   = razor_blade_length / 2 - (x - r_edge) * slant_tan;
function hw_off(x, g)  = hw_blade(x) + g / cos(slant_deg);
// How far short of the pocket's back wall the blade actually comes to rest:
// an offset g on the walls costs g/cos(slant) of half-width, and the blade
// recovers that by sitting (g/cos)/tan further in.
seat_short = -blade_fit / cos(slant_deg) / slant_tan;
horn_deg  = dt_at;              // see dt_at: NOT mouth_deg/2 any more


head_z0   = base_t + leg_h;                 // bottom of the cutting head
head_top  = head_z0 + head_h;
blade_z   = head_z0 + blade_rise;
shelf_z   = blade_z - razor_thickness / 2;

// THE STAGE SITS UNDER THE BLADE and is a PARALLEL flexure, not a
// cantilever. A cantilever tip rotates as it deflects -- 3*delta/(2L),
// which at full squeeze was 2.98 degrees and swung the blade's 61 mm edge
// by +/-1.59 mm. The edge then stops being tangent to the can and the bite
// no longer matches the parameter. Two MIRROR-SYMMETRIC leaves give pure
// radial translation instead: by symmetry the carrier cannot rotate.
//
// AND THE LEAVES MUST BE SQUARE TO THE SQUEEZE. A straight beam set at an
// angle carries the load along its own axis, where it is 383x stiffer than
// in bending (2683 N/mm vs 7.0) -- a truss, not a spring. So rigid arms
// stand the anchors off the body first, and only then do the leaves run
// tangentially, perpendicular to the motion.
leaf_top  = shelf_z - shelf_t - 1.5;
stage_z0  = leaf_top - leaf_h;
groove_out = bore_r + thr_depth + thr_clear;   // outer reach of the floor thread
socket_r  = body_r + boss_out;                // outer face of the boss
horn_x    = socket_r * cos(horn_deg);
horn_y    = socket_r * sin(horn_deg);
// Where the dovetail boss's 45-degree ramp has to start, and how far it drops.
// The boss's foot is a RECTANGLE laid against a CYLINDER, so if it reaches all
// the way to body_r its two corners at +/-(dt_deep+5)/2 stand 0.65 mm outside
// the wall -- four thin crescents of flat, unsupported bottom, measured at
// 7.56 mm2 on one layer. Pull the foot in to where the corners are still
// inside the wall, and drop it by its own radial run so the ramp stays at 45
// degrees. Narrowing the foot WITHOUT dropping it further would flatten the
// ramp to 39.8 deg, which is the wrong trade.
boss_foot_r = sqrt(body_r * body_r - pow((dt_deep + 5) / 2, 2)) - 0.05;
boss_drop   = socket_r - boss_foot_r;

// ---- THE MOUTH OPENS OUT ABOVE THE DOVETAILS ----
// The blade was being stopped by the FRAME instead of by the can. Measured:
// the cutting edge reaches +/-32.10 deg at the wall's outer surface at rest,
// +/-33.74 at 0.6 mm of advance and +/-35.31 at 1.2, against a +/-34 deg
// mouth -- so about 0.65 mm of usable travel, after which the jambs take the
// cutting load. frame X blade reads 0.009 mm3 at 1 mm of advance and 0.158 at
// 1.5, in two pieces, one at each jamb, in the blade's own thickness band.
//
// Above the top of the dovetail engagement the wall has no job left: the
// flanks are what the tenons need and the tenons end at mouth_hi_z. So
// everything inboard of the OUTERMOST corner of each socket's outboard flank
// comes out above that height. The bound is an existing feature rather than a
// number somebody picked, and it is far more than the blade needs -- the edge
// could advance to x = 24.0 before reaching it, against a can at 33.0 -- so
// the frame can never be the limit again.
//
// Nothing to support: material is only REMOVED from above, which leaves flat
// top faces and vertical jambs.
mouth_hi_z   = stage_z0 + leaf_h;
dt_out_deg   = dt_at + atan((dt_deep / 2) / (socket_r - dt_depth));
mouth_hi_deg = 2 * dt_out_deg;

floor_lo  = base_t + 3;
// The carrier is carrier_h TALL -- its top, not its base, is what meets
// the arm. floor_hi = arm_z0 - 6 left the carrier crashing 10 mm into the
// flexure at full travel. Clearance must be measured from the moving
// part's leading face, not its origin.
floor_hi  = stage_z0 - carrier_h - 4;
can_base_z = floor_z + carrier_h;           // the real datum: the can stands
                                            // on the carrier's own top face now
// The number you actually care about, which no parameter was stating. blade_rise
// is NOT this -- it is where the blade sits in the FRAME, measured up from the
// bottom of the head, and it says nothing about the can. How far up the can the
// cut lands is the gap between the blade and whatever height the floor is at.
// THE LIP'S ANGLE, and why it is no longer 45.
//
// The lip's underside is generated by shrinking the pocket outline as z
// rises; shrinking by rail_h over a rise of rail_h is the 45 deg this file
// used to assume. TILTING THE POCKET TIPS THAT PLANE: measured on this
// geometry, a 45 deg lip at blade_tilt = 30 comes out at 15.9 deg on the low
// side -- 254 mm2 of it, against a 36.9 deg limit. So the lip is cut STEEPER
// by the tilt it has to survive, and reaches inward less per unit of rise.
lip_deg   = max(45, blade_tilt + lip_deg_min);
lip_reach = rail_h / tan(lip_deg);          // inward reach over the rail_h rise
// How far the blade's high wing climbs above blade_z at the platform's FRONT
// face, which is the highest blade material the platform has to contain.
tilt_high = hw_blade(shelf_in) * sin(blade_tilt);
blade_top = shelf_z + razor_thickness;      // the blade's upper face
plat_top  = blade_top + tilt_high + blade_lift + rail_h;  // the rails' crest,
                        // raised to clear the high wing: a tilted blade is
                        // tilt_high taller on one side than a flat one, and a
                        // platform sized for flat lets the slot exit through
                        // its own roof -- measured at 10.8 mm of blade above
                        // plat_top at 30 deg before this term existed.
cut_height = blade_z - can_base_z;
echo(str("cut lands ", cut_height, " mm above the can's base",
         "   (range ", blade_z - (floor_hi + carrier_h),
         " to ", blade_z - (8 + carrier_h), " mm)"));

// ============================================================
//  ASSEMBLY
// ============================================================
// Gated so a probe file can `include` this one for its PARAMETERS without
// also getting the whole assembly rendered into its output. `use` does not
// import variables, so include is the only way in -- and an unguarded
// top-level call makes every intersection probe a no-op, because the thing
// you are measuring is a subset of the model sitting next to it in the same
// STL. Four flank probes all read 108800.749 mm3 that way: the model's own
// volume, four times, reported as a measurement.
show_assembly = true;
if (show_assembly) cutter();

module cutter() {
    if (show_can)   can_ghost();
    // The blade travels WITH the stage: the stage is what holds it, so an
    // exploded view that leaves it hanging in the frame is lying about the
    // assembly.
    if (show_blade) translate([explode * stage_out, 0, explode * stage_up])
        blade_in_place();
    if (show_frame) frame();
    if (show_stage) translate([explode * stage_out, 0, explode * stage_up]) blade_stage();
    if (show_floor) floor_carrier();
}

module can_ghost() {
    // The can stands on the CARRIER's own top face now that the puck is fused
    // into it, so the datum is simply floor_z + carrier_h. When the puck was
    // loose this was floor_z + carrier_h - recess_d + land_h + puck_t, and an
    // earlier version omitted the tail of that and drew the ghost 14 mm low --
    // an instrument that lied about the one datum every height here is measured
    // from. carrier_h absorbed the 1.6 mm difference, so the number is the same.
    %translate([0, 0, can_base_z]) cylinder(h = 123, r = can_r);
}

// local +X (edge) -> global -Y ; local +Y (body) -> global +X
module blade_in_place() {
    // The blade rides the CARRIER, so it takes the lever's rotation too --
    // otherwise the preview shows a blade that never moves and the whole
    // mechanism cannot be checked by looking at it.
    translate([hinge_cx, hinge_cy, 0]) rotate([0, 0, press_dir * press])
        translate([-hinge_cx, -hinge_cy, 0])
    tilt_about_edge()
    translate([r_edge, 0, blade_z])
        rotate([0, 0, -90])
            trapezoidal_razor_complete();
}

// ------------------------------------------------------------
//  FRAME: one tall annulus, with the mouth cut full height and
//  windows cut between the legs. Every surface is vertical, so
//  it prints upright with no bridging anywhere.
// ------------------------------------------------------------
module frame() {
    difference() {
        union() {
            tube(r_out = body_r, r_in = bore_r, h = head_top);
            base_ring();
            stage_bosses();
        }
        // The floor thread is a helical GROOVE cut outward into the wall,
        // so the bore's crests stay at bore_r and the can still clears them
        // by can_clearance. Cutting inward instead would put lands at
        // 32.1 mm, inside the 33.0 mm can, and the can would bind.
        // r_in reaches 1 mm INSIDE the bore on purpose. At exactly bore_r
        // the groove's inner cylinder is coincident with the bore surface
        // it is cutting, and CGAL turned that into 174 fragments. A cutter
        // must never share a face with the solid it cuts; push it into the
        // void, which here costs nothing because the bore is already empty.
        stage_sockets();
        translate([0, 0, base_t])
            helix_rib(r_in = bore_r - 1 - thr_clear,
                      r_out = bore_r + thr_depth + thr_clear,
                      shift = 0, play = 0, pitch = thr_pitch,
                      h = head_top - base_t);
        pie(angle = mouth_deg, r = body_r + 10, h = head_top + 4, z = -2);
        // and wider still above the dovetails -- see mouth_hi_deg
        pie(angle = mouth_hi_deg, r = body_r + 10,
            h = head_top + 4 - mouth_hi_z, z = mouth_hi_z);
        for (i = [0 : len(leg_at) - 1]) {
            a0 = leg_at[i] + leg_deg / 2;
            a1 = leg_at[(i + 1) % len(leg_at)] - leg_deg / 2;
            span = (a1 - a0 + 360) % 360;
            // pie() is CENTRED on +X, so the window must be rotated to
            // its MIDDLE, not its start. Rotating to a0 put every window
            // half a width off, straddling the legs it was meant to miss.
            //
            // ARCHED, not square-topped. A flat window ceiling is a 55 mm
            // horizontal span between legs -- 3071 mm2 of unprintable
            // overhang, measured. The window keeps full width up to
            // arch_z, then its angular width shrinks at 45 degrees to a
            // point, so every layer is carried by the one below it.
            rotate([0, 0, a0 + span / 2])
                arched_window(span, leg_h);
        }
    }
}

// Base: a ring whose inner lip is smaller than the bore, so the
// floor carrier cannot fall out of the bottom.
module base_ring() {
    tube(r_out = body_r, r_in = bore_r - 3, h = base_t);
}

// ------------------------------------------------------------
//  THE STAGE MOUNT.
//
//  The stage is the part most likely to break -- it is the only thing
//  here that flexes -- so it is a SEPARATE printed part that slides into
//  two dovetails on the body. Break one, print one, slide it in.
//
//  The dovetail opens OUTWARD and widens inward, so the cutting reaction
//  (which pushes the stage radially out) is exactly what the joint
//  resists. It slides down from above, and every face of it is vertical,
//  so it needs no support either.
// ------------------------------------------------------------
module stage_bosses() {
    for (s = [-1, 1])
        rotate([0, 0, s * horn_deg])
            // The boss protrudes boss_out past the wall, and that protrusion
            // was a flat 3.5 x 14 mm ceiling hanging in free air. Hulling it
            // down to a slab that stops at the wall grows the protrusion from
            // nothing at 45 degrees, so every layer lands on the one below.
            // The ramp's foot is at the body's own surface, where there is
            // solid wall at this azimuth all the way down.
            hull() {
                translate([groove_out + 0.2, -(dt_deep + 5) / 2, stage_z0 - dt_pad])
                    cube([socket_r - groove_out - 0.2, dt_deep + 5, leaf_h + dt_pad]);
                translate([groove_out + 0.2, -(dt_deep + 5) / 2,
                           stage_z0 - dt_pad - boss_drop])
                    cube([boss_foot_r - groove_out - 0.2, dt_deep + 5, 0.01]);
            }
}

module stage_sockets() {
    for (s = [-1, 1])
        rotate([0, 0, s * horn_deg])
            translate([0, 0, stage_z0])
                linear_extrude(height = leaf_h + 10)
                    dovetail_2d(0);
}

module stage_tenons() {
    for (s = [-1, 1])
        rotate([0, 0, s * horn_deg])
            translate([0, 0, stage_z0])
                linear_extrude(height = leaf_h)
                    dovetail_2d(dt_clear);
}

// Mouth at socket_r, widening inward. `clear` shrinks it all round for
// the tenon.
module dovetail_2d(clear) {
    x0 = socket_r - dt_depth + clear;      // deep end
    x1 = socket_r + 1;                     // past the mouth, so it opens out
    polygon([[x0,  dt_deep/2 - clear], [x1,  dt_open/2 - clear],
             [x1, -dt_open/2 + clear], [x0, -dt_deep/2 + clear]]);
}

// ------------------------------------------------------------
//  THE BLADE STAGE: a parallel flexure.
//
//  Two leaves, mirror images of each other, run TANGENTIALLY between
//  rigid stand-off arms and a central carrier that sits behind the
//  blade. Push the tab and the carrier translates straight in: no
//  rotation, so the edge stays tangent and the bite stays what the
//  parameter says.
//
//  0.059 mm/N measured from 24EI/L^3 for the pair -- 5 N gives 0.29 mm,
//  10 N gives 0.59 mm, leaf stress 15 MPa. The anchors sit on bosses
//  OUTSIDE the body, because the floor thread's groove now runs the full
//  height and would otherwise eat 1.85 mm of any root sunk into the wall.
// ------------------------------------------------------------
// ------------------------------------------------------------
//  STAGE: a LEVER, redesigned 2026-11-15.
//
//  Reads as three pieces and is one solid:
//    YOKE     rigid, tied to BOTH dovetails, spanning the mouth
//             outboard of everything. It never moves.
//    HINGE    one thin plate at hinge_at. Thin in the tangential
//             direction, tall in Z and long in X, so it is soft
//             about the vertical axis and stiff against the
//             blade's own cutting load.
//    CARRIER  the blade, its wedge pocket and the pad. Joined to
//             the yoke ONLY through the hinge, so pressing the
//             pad rotates it and the edge swings in at a point.
//
//  THE STOP IS PART OF THE MECHANISM, not a warning in a comment:
//  the carrier's tail reaches back past the hinge, and the yoke
//  catches it after bite_stop of advance at the contact. Press
//  harder and nothing happens.
// ------------------------------------------------------------
r_corner  = 59.90;              // the carrier's back corner, radius
grip_w    = 12;                 // width of the two grip posts
post_top  = stage_z0 + leaf_h + 16;   // how high they stand, clear of the band
tail_gap       =  0.723; // MEASURED: the arc tail advances 0.326 mm of radius
                        // per degree of press, so this is where it meets the
                        // yoke at bite_stop. The cube-faced version used
                        // 0.515, the gap at a flat face center, while its
                        // corners sat 0.244 off the wall and fired first.
                                // lands on the yoke's inner face. The radial
                                // rate along a ray is H_y*cos(az) - H_x*sin(az)
                                // and so does NOT depend on how far out the
                                // tail reaches -- 13.28 mm/rad at +30 deg,
                                // which is 0.515 mm at the 2.221 deg that
                                // bite_stop measures to.
// Positive `press` always means DEEPER, whichever corner the hinge is on:
// the bite rate is H_y, so the sense of the rotation flips with the side.
press_dir = (hinge_r * sin(hinge_at)) < 0 ? 1 : -1;
hinge_cx  = hinge_r * cos(hinge_at);
hinge_cy  = hinge_r * sin(hinge_at);
// Bite per degree of lever travel, from the kinematics rather than a guess:
// the contact starts at (r_edge, 0) and swings about the hinge, so its radial
// rate is the tangential component of that swing.
bite_per_deg = (r_edge - hinge_cx == 0 ? 1 : -(0 - hinge_cy)) * PI / 180;
press_deg    = bite_stop / (hinge_cy == 0 ? 1 : abs(hinge_cy) * PI / 180);
tail_at   =  30;                // the tail reaches back past the hinge. NOT
                                // -40: the yoke's stub to the -45 dovetail is
                                // 9 mm wide and the two bars crossed, 1058 mm3
                                // of overlap measured -- a stop welded to the
                                // thing it is supposed to stop against.
yoke_in   = yoke_r - yoke_t;

// THE TENON-TO-YOKE BRIDGE. stub_x0 sits stub_lap inboard of the tenon's mouth
// face so the two solids actually overlap rather than meeting at two coincident
// faces. MEASURED on the frame: material along this ray runs 32.86 .. 42.10 mm
// and stops, 42.10 being socket_r exactly -- the socket is cut 1 mm past the
// mouth, but there is no material out there to remove. So the lap region
// 42.60 .. 43.10 is already outside the frame, and the narrowest place the
// tenon has to pass is the socket at socket_r, 6.75 mm wide against the
// stage's 6.12 mm there.
stub_lap       =  0.5;
stub_x0        = socket_r + 1 - stub_lap;
stub_x1        = socket_r + 5;              // clear of the frame; it may widen
stub_w0        = dt_open - 2 * dt_clear;    // exactly the tenon's mouth width
stub_w1        = dt_deep + 3;               // the bridge's working width

module blade_stage() {
    stage_tenons();
    stage_yoke();
    translate([hinge_cx, hinge_cy, 0]) rotate([0, 0, press_dir * press])
        translate([-hinge_cx, -hinge_cy, 0]) {
            stage_hinge();
            stage_carrier();
        }
}

// The yoke: a band from dovetail to dovetail, standing off far enough to leave
// the hinge a gap, with a stub out to each tenon. Vertical faces throughout,
// standing on stage_z0, so nothing in it overhangs.
module stage_yoke() {
    translate([0, 0, stage_z0]) {
        difference() {
            pie(angle = 2 * horn_deg, r = yoke_r, h = leaf_h, z = 0);
            pie(angle = 2 * horn_deg + 2, r = yoke_r - yoke_t, h = leaf_h + 2, z = -1);
        }
        // THE BRIDGE FROM EACH TENON OUT TO THE YOKE ARC.
        //
        // The first cut of this was a plain cube starting at
        // socket_r - dt_depth -- the dovetail's own DEEP end -- and dt_deep
        // wide, which is the width of the socket's deep end with no clearance
        // at all. It was therefore wider than the tenon at every radius and
        // began 0.35 mm inboard of it. Measured on the assembled stage: it
        // swallowed 744.6 mm3 of tenon, which is 100.0% of both of them, so
        // the male dovetail did not exist -- it was a plain 9.0 mm bar, and a
        // 9.0 mm bar cannot pass a 6.0 mm socket mouth. It also put
        // 101.2 mm3 of itself inside the frame.
        //
        // The frame's last material along this ray is at socket_r, 42.10 mm
        // (measured: material 32.86 .. 42.10, nothing beyond). So the bridge
        // starts at the tenon's own MOUTH width, laps back into the tenon far
        // enough to make one solid, and flares to its working width only once
        // it is past the frame. The flare's edge crosses the tenon's slant at
        // x 42.778, y 2.782 -- outboard of the frame -- so the dovetail keeps
        // its full undercut over every radius the socket actually grips.
        for (sgn = [-1, 1])
            rotate([0, 0, sgn * horn_deg])
                linear_extrude(height = leaf_h)
                    polygon([[stub_x0,  stub_w0 / 2], [stub_x1,  stub_w1 / 2],
                             [yoke_r,   stub_w1 / 2], [yoke_r,  -stub_w1 / 2],
                             [stub_x1, -stub_w1 / 2], [stub_x0, -stub_w0 / 2]]);
    }
    // THE FIXED HALF OF THE THUMB GRIP: a post beside the moving one, so the
    // press is a PINCH between two posts and the frame takes no side load.
    rotate([0, 0, press_at])
        translate([yoke_r - yoke_t, -grip_w / 2, stage_z0])
            cube([yoke_t, grip_w, post_top - stage_z0]);
}

// The hinge plate: thin tangentially, tall in Z, spanning the yoke gap.
module stage_hinge() {
    rotate([0, 0, hinge_at])
        translate([hinge_r - hinge_l / 2, -hinge_t / 2, stage_z0])
            cube([hinge_l, hinge_t, plat_top - stage_z0]);
}

module stage_carrier() {
    difference() {
        union() {
            translate([0, 0, stage_z0])
                linear_extrude(height = plat_top - stage_z0) plat_plan();
            // the ear that reaches the hinge
            rotate([0, 0, hinge_at])
                translate([r_corner - 2, -hinge_t * 2, stage_z0])
                    cube([hinge_r - r_corner + 2, hinge_t * 4, plat_top - stage_z0]);
            // THE THUMB POST on the free corner, rising clear of the yoke band
            rotate([0, 0, press_at])
                translate([r_corner - 4, -grip_w / 2, stage_z0])
                    cube([6, grip_w, post_top - stage_z0]);
            // THE TAIL, reaching back past the hinge to meet the yoke, and
            // the thing that stops the press at bite_stop.
            //
            // Its outer face is an ARC CONCENTRIC WITH THE YOKE's inner face,
            // not a flat cube face. A flat face meeting a concave cylinder
            // touches at its CORNERS first: the cube version's face center was
            // tail_gap from the wall, but its corners, grip_w/2 off axis, were
            // only 0.244 mm off it, and measurement found them 0.50 mm INSIDE
            // the yoke wall at full press -- a wedge on two points, firing at
            // roughly half the intended travel. An arc gives the same gap at
            // every point across the width, so the stop lands as a face and
            // engages where the number says it does.
            rotate([0, 0, tail_at])
                intersection() {
                    translate([r_corner - 8, -grip_w / 2, stage_z0])
                        cube([yoke_in - r_corner + 8, grip_w, leaf_h]);
                    translate([0, 0, stage_z0])
                        cylinder(h = leaf_h, r = yoke_in - tail_gap, $fn = 360);
                }
        }
        tilt_about_edge() blade_pocket();
        tilt_about_edge() lip_void();
    }

    for (sgn = [-1, 1])
        tilt_about_edge()
        translate([notch_r, sgn * notch_y, shelf_z]) {
            cylinder(h = razor_thickness + pin_stand, d = pin_d, $fn = 24);
            translate([0, 0, razor_thickness + blade_lift])
                cylinder(h = rail_h, r1 = pin_d / 2,
                         r2 = pin_d / 2 + lip_reach, $fn = 24);
        }
}

// Plan outline of the platform: the blade's trapezoid grown by rail_t on the
// slanted edges, squared off at the front face and carried back past the
// spine at constant width so the tab has something to stand on.
// THE TILT. Rotates its children about the RADIAL line through the contact
// point -- x unchanged, (y,z) rotated about (0, blade_z). The contact point
// (r_edge, 0, blade_z) lies ON that axis, so it does not move: the bite stays
// blade_bite and the edge stays tangent, which is the whole point of choosing
// this axis over Z.
module tilt_about_edge() {
    translate([0, 0, blade_z])
        rotate([blade_tilt, 0, 0])
            translate([0, 0, -blade_z])
                children();
}

module plat_plan() {
    wf = hw_off(shelf_in, rail_t);
    ws = hw_off(spine_r,  rail_t);
    polygon([[shelf_in, wf], [spine_r, ws], [shelf_out, ws],
             [shelf_out, -ws], [spine_r, -ws], [shelf_in, -wf]]);
}

// The pocket the blade slides into, offset by blade_fit on the slanted walls.
// It is open at the front because it starts at r_edge = 32.7, which is 5.3 mm
// inboard of the platform's own front face at shelf_in = 38 -- so the mouth
// is a hole in that face, not a feature that needs a lead-in.
//
// The back wall is tab_in, 0.2 mm beyond the spine. The blade never reaches
// it: the walls converge at 37.54 deg, so it jams seat_short mm early. The
// tab's face is a hard limit on over-driving, not the seat.
module blade_pocket(g = undef) {
    gg = is_undef(g) ? blade_fit : g;
    translate([0, 0, shelf_z])
        linear_extrude(height = razor_thickness + blade_lift)
            pocket_plan(gg);
}

module pocket_plan(g) {
    polygon([[r_edge,  hw_off(r_edge, g)], [tab_in,  hw_off(tab_in, g)],
             [tab_in, -hw_off(tab_in, g)], [r_edge, -hw_off(r_edge, g)]]);
}

// Everything above the blade that is NOT the retaining lip.
//
// The lip's underside is a 45-DEGREE PLANE, not a flat roof. It meets the
// blade's top face right at the slanted wall and climbs away inward, so the
// first lip layer oversteps the pocket wall by exactly layer_h and every one
// after it oversteps the one below by the same. Nothing to support, and no
// bridge: a flat lip reaching rail_h over the blade would be a 2 mm
// cantilever at 0.15 mm layers, which droops into the slot and eats the fit.
//
// It retains by line contact at the wall, which is all that is needed -- the
// blade cannot lift more than blade_lift before its corner is under the
// apex, and it cannot slide sideways out from under one lip without driving
// into the other, because the pocket is a wedge.
module lip_void() {
    zb = blade_top + blade_lift;
    hull() {
        translate([0, 0, zb]) linear_extrude(height = 0.01)
            pocket_plan(blade_fit);
        translate([0, 0, zb + rail_h]) linear_extrude(height = 0.01)
            pocket_plan(blade_fit - lip_reach);
    }
    // AND THE CHANNEL ABOVE IT, OPEN TO THE SKY.
    //
    // While the platform was sized for a flat blade, plat_top WAS zb + rail_h,
    // so the ledge's top face was the part's own top surface and there was
    // never any material above it. Raising plat_top to clear the tilt put
    // tilt_high of solid over that face and turned it into a ROOF -- measured
    // at 507 mm2 of 15.9 deg overhang, which is the whole of what the first
    // attempt got wrong. The slot has to stay a channel: everything above the
    // ledge is removed, so the only downward faces left are the two ledges
    // themselves, cut at lip_deg.
    translate([0, 0, zb + rail_h])
        linear_extrude(height = plat_top - zb + 2 * tilt_high + 5)
            pocket_plan(blade_fit - lip_reach);
}

// ------------------------------------------------------------
//  FLOOR: a threaded carrier that screws up and down the bore,
//  with a loose puck sitting on it that the can stands on.
//
//  WHY A LOOSE PUCK. Turning the can drags on whatever it stands
//  on. If that were the carrier, the can would unscrew the floor
//  as you cut. The puck spins freely on a centre pip instead, so
//  the can's friction goes into spinning a 3 mm disc and never
//  reaches the thread.
//
//  WHY NO KNOB AND NO CENTRAL SCREW. A central screw long enough
//  for 54 mm of travel protrudes 57 mm below the bench at its
//  low setting. Threading the bore puts the adjustment where you
//  can already reach it: the carrier's fluted rim is exposed in
//  the windows between the legs, so a fingertip rolls it.
// ------------------------------------------------------------
module floor_carrier() {
    r_core = bore_r - core_clear;
    translate([0, 0, explode])
    // The grip flutes and the thread cannot share a surface -- cut into the
    // rim where the rib is and you chop the thread into fragments (Genus 85).
    // So: a plain fluted SKIRT you can reach through a window and roll with
    // a fingertip, and the THREAD above it, untouched.
    //
    // PHASE. The bore groove and this rib are independent helices whose
    // generators start at different heights, so the carrier must be rotated
    // by the difference or the rib ploughs through the frame.
    translate([0, 0, floor_z]) {
        difference() {
            union() {
                difference() {
                    cylinder(h = skirt_h, r = r_core);
                    for (a = [0 : 12 : 359])
                        rotate([0, 0, a]) translate([r_core, 0, -1])
                            cylinder(h = skirt_h + 2, d = 2.6);
                }
                translate([0, 0, skirt_h])
                    cylinder(h = carrier_h - skirt_h, r = r_core);
                translate([0, 0, -floor_z]) carrier_rib();
            }
            // THE PUCK IS PART OF THIS NOW, so the recess it sat in, the pip
            // it spun on and the narrow thrust land it bore against are all
            // gone -- they existed only to keep the can's rotation out of the
            // thread, and a fused top hands that rotation straight to it.
            // The top is simply solid to carrier_h and the can stands on it.
            //
            // The three finger bores go straight through the whole part at one
            // radius, so they align by construction. The concentric groove the
            // free puck needed is gone with it: a puck that could sit at any
            // angle had to meet a feature that looked the same from every
            // angle, and a fused one does not.
            for (i = [0 : grip_n - 1])
                rotate([0, 0, i * 360 / grip_n])
                    translate([grip_r, 0, -1])
                        cylinder(h = carrier_h + 2, d = grip_d);
        }
    }
}

// PHASE BY CONSTRUCTION, not by arithmetic. The groove and the rib are
// generated from the SAME origin (z = base_t) with the same pitch, then the
// rib is trimmed to the carrier's band. A hand-computed phase offset looked
// right on paper -- both helices agreed at every height I checked -- and the
// parts still interfered at exactly the three leg angles, because the
// convention I assumed for linear_extrude's twist sign was wrong. Deriving
// the rib from the groove's own frame makes the question impossible to get
// wrong.
module carrier_rib() {
    r_core = bore_r - core_clear;
    intersection() {
        translate([0, 0, base_t])
            helix_rib(r_in = r_core - 1, r_out = bore_r + thr_depth - thr_clear,
                      shift = thr_clear, play = thr_play_deg, pitch = thr_pitch,
                      h = head_top - base_t);
        translate([0, 0, floor_z + skirt_h])
            cylinder(h = carrier_h - skirt_h, r = bore_r + 5);
    }
}

// A loose disc. The can spins THIS, not the carrier, so the can's friction
// never reaches the thread and cannot unscrew the floor as you cut.

module helix_rib(r_in, r_out, shift, play, pitch, h) {
    turns = ceil(h / pitch);
    // SEGMENTS OVERLAP, THEY DO NOT ABUT. Stacked turns that merely touch
    // share a face at every seam, and CGAL left slivers of uncut material
    // there -- which showed up as interference blobs straddling z=45, a seam
    // height, and which widening the groove did nothing to fix. Each turn is
    // exactly 360 degrees, so extending a segment upward continues the same
    // helix and simply overlaps the next one.
    for (i = [0 : turns - 1]) {
        seg = min(pitch, h - i * pitch) + (i < turns - 1 ? seam_ov : 0);
        if (seg > 0.01)
            translate([0, 0, i * pitch])
                linear_extrude(height = seg, twist = -360 * seg / pitch,
                               slices = max(8, ceil(120 * seg / pitch)))
                    rib_section(r_in, r_out, shift, play);
    }
}

// Angular width of the thread at radius r. Constant inboard of the bore
// face (that stretch only exists to keep the cutter off the bore surface),
// then tapering outward so the groove's CEILING slopes instead of hanging
// flat. A square groove's ceiling sits at the helix angle, 1.4 degrees --
// effectively horizontal, and it droops into the clearance we depend on.
function tdeg(r) =
    r <= bore_r ? nut_deg_mouth
                : nut_deg_mouth - (nut_deg_mouth - nut_deg_root)
                  * (r - bore_r) / (thr_depth + thr_clear);

module rib_section(r_in, r_out, shift, play) {
    n   = 18;
    din = tdeg(r_in + shift) - play;
    dot = tdeg(r_out + shift) - play;
    polygon(concat(
        [for (i = [0:n]) let (a = -din/2 + i*din/n) [r_in*cos(a), r_in*sin(a)]],
        [for (i = [1:n]) let (f = i/n, r = r_in + (r_out-r_in)*f,
                              a = (tdeg(r + shift) - play)/2) [r*cos(a), r*sin(a)]],
        [for (i = [1:n]) let (a = dot/2 - i*dot/n) [r_out*cos(a), r_out*sin(a)]],
        [for (i = [1:n-1]) let (f = 1 - i/n, r = r_in + (r_out-r_in)*f,
                              a = -(tdeg(r + shift) - play)/2) [r*cos(a), r*sin(a)]]
    ));
}

// A window with a 45-degree gabled top, approximated in slabs.
module arched_window(span, h) {
    half0 = body_r * sin(span / 2);
    arch  = half0;                       // 45 deg: rise equals half-chord
    flat  = max(1, h - arch);
    rr = body_r + 10;
    // The arch used to be sixteen stacked sectors, and sixteen 1.7 mm risers
    // read as a staircase however fine $fa gets -- the steps were in the
    // model, not in the tessellation. This is the same shape with no steps:
    // the exact angular sector, intersected with a chord-bounded 45-degree
    // gable. Below the spring line the sector is the narrower of the two, so
    // the window keeps its true radial sides; above it the gable governs and
    // the ceiling is one smooth plane each side. The half-chord law is
    // identical to the stepped version's -- half0*(1-f) -- because a 45-degree
    // gable sheds exactly its own rise, and arch == half0.
    intersection() {
        pie(angle = span, r = rr, h = flat + arch, z = base_t);
        rotate([90, 0, 90])
            linear_extrude(height = rr)
                polygon([[-half0, base_t],
                         [ half0, base_t],
                         [ half0, base_t + flat],
                         [     0, base_t + flat + arch],
                         [-half0, base_t + flat]]);
    }
}

module pie(angle, r, h, z = 0) {
    translate([0, 0, z])
        rotate([0, 0, -angle / 2])
            rotate_extrude(angle = angle)
                square([r, h]);
}

module tube(r_out, r_in, h) {
    difference() {
        cylinder(h = h, r = r_out);
        translate([0, 0, -1]) cylinder(h = h + 2, r = r_in);
    }
}

// ============================================================
//  THE BLADE  (unchanged -- the part you already had)
// ============================================================
// --- Parameterized Modules ---

module trapezoidal_razor_complete(
    blade_length        = razor_blade_length,
    spine_length        = razor_spine_length,
    height              = razor_height,
    thickness           = razor_thickness,
    holes               = razor_hole_count,
    hole_dia            = razor_hole_dia,
    hole_distance       = razor_hole_distance,
    notches             = razor_notch_count,
    notch_distance      = razor_notch_distance,
    notch_depth         = razor_notch_depth,
    notch_top_width     = razor_notch_top_width,
    notch_root_dia      = razor_notch_root_dia,
    bevel_primary_angle = razor_bevel_primary_angle,
    bevel_micro_angle   = razor_bevel_micro_angle,
    bevel_micro_height  = razor_bevel_micro_height
) {
    difference() {
        // 1. Solid beveled blade body
        trapezoidal_razor_body(
            blade_length        = blade_length,
            spine_length        = spine_length,
            height              = height,
            thickness           = thickness,
            bevel_primary_angle = bevel_primary_angle,
            bevel_micro_angle   = bevel_micro_angle,
            bevel_micro_height  = bevel_micro_height
        );
        
        // 2. Subtract central mounting holes (automatically skipped if holes == 0)
        if (holes > 0) {
            for (i = [0 : holes - 1]) {
                x_pos = (i - (holes - 1) / 2) * hole_distance;
                translate([x_pos, height / 2, 0])
                    cylinder(h = thickness * 3, d = hole_dia, center = true);
            }
        }
        
        // 3. Subtract top locking notches (automatically skipped if notches == 0)
        if (notches > 0) {
            for (i = [0 : notches - 1]) {
                x_pos = (i - (notches - 1) / 2) * notch_distance;
                translate([x_pos, height, 0])
                    spine_notch(
                        depth     = notch_depth,
                        top_width = notch_top_width,
                        root_dia  = notch_root_dia,
                        thickness = thickness
                    );
            }
        }
    }
}

module spine_notch(depth, top_width, root_dia, thickness) {
    root_r = root_dia / 2;
    hull() {
        translate([0, -depth + root_r, 0])
            cylinder(h = thickness * 3, r = root_r, center = true);
        
        translate([0, 0.5, 0])
            cube([top_width, 1.0, thickness * 3], center = true);
    }
}

module trapezoidal_razor_body(
    blade_length        = razor_blade_length,
    spine_length        = razor_spine_length,
    height              = razor_height,
    thickness           = razor_thickness,
    bevel_primary_angle = razor_bevel_primary_angle,
    bevel_micro_angle   = razor_bevel_micro_angle,
    bevel_micro_height  = razor_bevel_micro_height
) {
    // Derived Geometric Values (Exact)
    z_body  = thickness / 2;
    z_micro = bevel_micro_height * tan(bevel_micro_angle);
    
    bevel_primary_height = (z_body - z_micro) / tan(bevel_primary_angle);
    
    y_micro = bevel_micro_height;
    y_prim  = bevel_micro_height + bevel_primary_height;

    function half_width(y) = 
        (blade_length / 2) - (y / height) * ((blade_length - spine_length) / 2);

    x0 = half_width(0);       // Cutting edge
    x1 = half_width(y_micro); // Micro-bevel ridge
    x2 = half_width(y_prim);  // Primary bevel shoulder
    x3 = half_width(height);  // Top spine

    points = [
        // Left slanted edge (-X): indices 0 to 6
        [-x0, 0,       0],        // 0: Cutting apex
        [-x1, y_micro, z_micro],  // 1: Top micro-bevel ridge
        [-x2, y_prim,  z_body],   // 2: Top primary shoulder
        [-x3, height,  z_body],   // 3: Top spine corner
        [-x3, height,  -z_body],  // 4: Bottom spine corner
        [-x2, y_prim,  -z_body],  // 5: Bottom primary shoulder
        [-x1, y_micro, -z_micro], // 6: Bottom micro-bevel ridge

        // Right slanted edge (+X): indices 7 to 13
        [ x0, 0,       0],        // 7: Cutting apex
        [ x1, y_micro, z_micro],  // 8: Top micro-bevel ridge
        [ x2, y_prim,  z_body],   // 9: Top primary shoulder
        [ x3, height,  z_body],   // 10: Top spine corner
        [ x3, height,  -z_body],  // 11: Bottom spine corner
        [ x2, y_prim,  -z_body],  // 12: Bottom primary shoulder
        [ x1, y_micro, -z_micro]  // 13: Bottom micro-bevel ridge
    ];

    faces = [
        [0, 1, 8, 7],             // Top micro-bevel facet
        [1, 2, 9, 8],             // Top primary bevel facet
        [2, 3, 10, 9],            // Top flat blade body
        [3, 4, 11, 10],           // Flat top spine
        [4, 5, 12, 11],           // Bottom flat blade body
        [5, 6, 13, 12],           // Bottom primary bevel facet
        [6, 0, 7, 13],            // Bottom micro-bevel facet
        [0, 6, 5, 4, 3, 2, 1],    // Left side face (-X)
        [7, 8, 9, 10, 11, 12, 13] // Right side face (+X)
    ];

    polyhedron(points = points, faces = faces, convexity = 4);
}