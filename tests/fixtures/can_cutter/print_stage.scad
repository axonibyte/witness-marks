// The lever stage alone, dropped to the bed for printing.
show_assembly = false;
include <can_cutter.scad>;
translate([0, 0, -stage_z0]) blade_stage();
