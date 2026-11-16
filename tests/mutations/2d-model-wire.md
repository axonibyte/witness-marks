# `model.diff` wired -- six mutants, six kills

Issue #34 asked for this "via provider measure and sections".

## The provider publishes no hash

`measure(mode=model)` returns volume, surface area, bbox, triangle and vertex
counts, manifoldness -- and no content hash of any kind. `diff` REFUSES a
snapshot without one, because two un-hashed snapshots compare equal and report
themselves bit-identical, a PASS under `Intent::Refactor`.

The tempting shortcut was a FINGERPRINT over the measured properties. That is
a different claim: volume, area, bbox and counts can all agree between meshes
that are not the same mesh, and writing it into a field documented as a
content hash makes the documentation false exactly where someone relies on it.

So the identity comes from the bytes. `export_model` writes a real mesh file
and names its path; hashing those bytes is a genuine content hash and the
contract holds as written.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `mdlwire-no-path-hashes-empty` | an export with no path can be hashed empty | `an_export_naming_no_path_is_refused_rather_than_hashed_empty` |
| `mdlwire-format-not-in-identity` | the export format is not part of the identity | `the_same_bytes_under_two_formats_do_not_share_an_identity` |
| `mdlwire-digest-ignores-bytes` | the bytes need not enter the digest | `different_bytes_give_different_identities` |
| `mdlwire-empty-export-accepted` | a zero-byte export is an empty model | `a_zero_byte_export_is_refused` |
| `mdlwire-missing-volume-is-zero` | an unmeasured volume is zero | `a_measurement_without_a_volume_is_refused` |
| `mdlwire-empty-digest-accepted` | an empty digest can build a snapshot | `an_empty_digest_is_refused_at_the_snapshot` |

## The translation case, in measured data

The fixture is not constructed. The provider measures the moved part at
**3717.7106 mm3** -- exactly the volume of the original, to the last digit --
with a bbox shifted 3 mm and different exported bytes. That is the case a
volume-keyed diff reports as "no change", now pinned by a real reading rather
than a hand-written one.

## The survivor that was right

`mdlwire-format-not-in-identity` SURVIVED its first run. The digest already
carried the format as its string prefix, so also feeding it into the hash body
changed nothing any test could see. The mutant was correct: that was dead
work. It is removed rather than wrapped in a test asserting it happens, and
the mutant now targets the prefix, which is load-bearing.

A mutant that survives is sometimes telling you the code is redundant, not
that the test is missing.
