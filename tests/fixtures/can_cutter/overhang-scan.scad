// overhang-scan.scad -- the test a SLICER actually performs, run on the model.
//
// A slicer does not look at face normals. It slices to 2D layers and asks, for
// each layer, which part of it has nothing under it:
//
//     unsupported(z) = slice(z)  MINUS  offset(slice(z - h), +d)
//     d = h / tan(threshold)
//
// d is the furthest a perimeter may step outward in one layer. That makes the
// threshold a property of the PRINT, not of the model: a wall is self-supporting
// when its per-layer step stays under about half an extrusion width, so
//
//     critical angle = atan(layer_height / (0.5 * extrusion_width))
//
// 0.20 mm layers at 0.40 mm width -> 45 deg, which is where the folk rule comes
// from. 0.10 mm layers at the same width -> 26.6 deg: finer layers tolerate
// SHALLOWER overhangs. Quoting 45 without the layer height is quoting half a fact.
//
// USAGE
//   include <overhang-scan.scad>
//   module target() { translate([0,0,-ZMIN]) import("part.stl"); }
//   overhang_scan(0.15, 45, 0, NLAYERS);
// then render to STL. Each layer's unsupported region is extruded 0.5 mm and
// parked at z = layer_index, so ONE render carries every layer:
//   area(layer i) = (volume in the z-band [i, i+1)) / 0.5
// Drop layer 1: it is on the bed. An EMPTY result writes no STL -- that is the
// pass condition, so check the exit status, never the file's existence.
//
// VALIDATED against hand-computed answers:
//   flat 20x10 cantilever off a post   198.5 measured   ~197 by hand
//   45 deg cone                          0.0             0
//   60 deg cone                          0.0             0
//   30 deg cone                        131.1           132.8
//
// LIMIT: this does not exempt BRIDGES. A region anchored on two opposite sides
// within bridging range prints fine unsupported, and this reports it anyway.
// Check flagged regions by hand for anchors before adding support -- though note
// OrcaSlicer's own default is bridge_no_support = 0, i.e. it supports them too.
module overhang_scan(h, thr, z0, n) {
    d = h / tan(thr);
    for (i = [1 : n]) {
        z = z0 + i*h;
        translate([0, 0, i*1.0])
            linear_extrude(height = 0.5)
                difference() {
                    projection(cut=true) translate([0,0,-z]) target();
                    offset(delta = d)
                        projection(cut=true) translate([0,0,-(z-h)]) target();
                }
    }
}
