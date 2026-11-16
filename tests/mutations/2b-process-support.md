# `proc.support` -- eight mutants, eight kills

The criterion, from the spec:

```text
unsupported(z) = slice(z) - offset(slice(z - h), +d)      d = layer_h / tan(theta)
```

theta measured **from the horizontal**. The critical angle is
`atan(layer_h / (0.5 * width))` -- 45 degrees only because 0.2 mm at 0.4 mm
makes the ratio 1; at 0.1 mm the same nozzle holds 26.565.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `support-offset-inverts-the-convention` | `d = layer_h * tan(theta)` | `away_from_forty_five_the_offset_is_not_the_layer_height` |
| `support-one-anchor-bridges` | one anchor is enough to bridge | `the_same_face_with_one_anchor_is_a_cantilever_and_needs_support` |
| `support-span-limit-ignored` | any span bridges if it has two anchors | `a_span_past_the_bridging_limit_names_the_span_not_the_anchors` |
| `support-horizontal-gets-infinity` | a horizontal surface has an offset | `a_horizontal_surface_is_refused_rather_than_given_an_infinite_offset` |
| `support-past-vertical-allowed` | past vertical is still an overhang | `an_angle_at_or_past_vertical_is_refused` |
| `support-zero-area-region-passes` | a zero-area region needs no support | `a_zero_area_region_is_refused_rather_than_passed` |
| `support-zero-bridge-limit-allowed` | a zero bridging limit is usable | `a_non_positive_bridging_limit_is_refused` |
| `support-area-sums-every-region` | bridges cost support material | `the_support_area_sums_only_what_needs_support` |

## Measured: a 45-degree test cannot catch an inverted convention

`support-offset-inverts-the-convention` replaces `layer_h / tan(theta)` with
`layer_h * tan(theta)`. Run against the two angle tests separately, with the
mutation in place:

```
at_forty_five_degrees_the_offset_is_exactly_one_layer    ok.     1 passed
away_from_forty_five_the_offset_is_not_the_layer_height  FAILED. 1 failed
```

tan(45) is 1, so multiplying and dividing give the same answer, and the
45-degree test passes a build with the convention backwards. That is the whole
reason the convention has to be stated: 45 degrees is the one angle where both
readings agree, which makes it the one angle useless for testing them.

The project's own fixture had to move off 45 to have an opinion. So does
anybody else's.

## Face normals are the wrong instrument

A face-normal test asks which way a triangle points. It gets two classes wrong:

- **A bridge** is horizontal, so its normal points straight down -- the most
  unsupported reading available -- and anchored at both ends it needs no
  support at all.
- **A staircase** of small steps has every face vertical or horizontal, so no
  normal ever lands in the overhang band, and 0.3 mm steps on a 0.2 mm layer
  are unsupported at every one.

`support-one-anchor-bridges` is the first class stated as code: the bridge and
the cantilever in those two fixtures are the same area, the same span, the same
horizontal face, and the same normal. Only the anchor count separates them, and
a normal test does not have one.
