# Mutation record: the reaper image pin

`tools/tests/t_reaper_pin.sh` holds .reaper.toml to a digest pin and checks the
toolchain it records against rust-toolchain.toml. Five mutants, five kills,
each driven with `REAPER_ROOT` pointing at a COPY.

| mutation | killed by |
| --- | --- |
| the pin edited back to a tag | the digest arm |
| the toolchain bumped, image not re-resolved | the recorded-version arm |
| the verified RUST_VERSION record removed | the recorded-version arm |
| a TODO reintroduced | the TODO arm |
| the manifest restructured so no `image =` line parses | the blind-parser refusal |

THE SECOND ARM IS THE ONE WITH TEETH. Pinning by digest fixes the IMAGE and
says nothing about what is inside it. Bumping rust-toolchain.toml without
re-resolving the image now fails here rather than in a container quietly
building with the wrong compiler.

THE TEST IS OFFLINE ON PURPOSE. Resolving the digest from the registry at test
time would need the network and, worse, would check the pin against the CURRENT
tag -- which is the exact thing pinning exists to stop mattering.

## Tiers

Touched: Tier 3, source-as-data, the manifest being the fixture. The pin itself
was additionally verified live: `reaper up` pulled the image by digest and the
registry served it. Skipped: every other tier.
