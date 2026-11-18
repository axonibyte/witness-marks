# The falsification plan

Spec §10 lists six ways this project could be wrong. Issue 46 asks for a plan
for tests 1, 2, 4 and 5, plus the scope-versus-measurement accounting that
test 6 raises. The results are their own issues; this is the plan.

Each test below states its **current evidence** before its experiment, because
two of the four have moved since the spec was written and one of those moved
against the spec's own guess.

## Citations

Every path and figure this document names, so a citation that stops resolving
is caught rather than believed. `tools/tests/t_falsification_plan.sh` reads
this table.

| claim | cited |
| --- | --- |
| the corpus schema has no cost field | tests/corpus/build.toml |
| 108 incidents, 10 with no class | tests/corpus/README.md |
| the rediscovery matrix says 1 of 108 | tests/corpus/rediscovery.toml |
| dependency discovery is by perturbation | crates/wm-claims/src/claim.rs |
| perturbation moves one input at a time | crates/wm-claims/src/claim.rs |
| the index is the single account | crates/wm-claims/src/index.rs |
| trace is lexical, file-scope only | crates/wm-claims/src/wire.rs |
| the pinning axes enumerated | docs/pinning-axes.md |

---

## Test 1 — a second harness produces classes outside A–G

**Current evidence: partly fired, without a second harness.** The corpus went
60 → 106 → 108 entries on the *same* part and produced two new classes on the
way. And 10 of 108 entries carry `class = ""`, which `tests/corpus/README.md`
calls an answer rather than an omission: *"none of the seven genuinely fits"*.
So the seven may be the shape of a hundred and eight entries rather than the
shape of the problem — which is the test's own hypothesis, already half
confirmed by growth alone.

**The experiment.** Instrument a second harness part, log to the same schema,
classify every incident, and count the blank-class entries separately from the
A–G ones.

**What falsifies.** A class H that none of A–G can absorb. The weaker signal,
and the one to watch for first: the blank-class *fraction* rising above the
current 10/108 on the new part. A rising blank fraction says the classes are
running out before the problem does.

**The choice of part is not mine.** Spec §10 names fasteners, seals and
anything thermal as likely sources of H. Each tests a different axis:

- **a fastener** — tolerance stack-up and assembly order; closest to the
  existing corpus, so the weakest test of the classes
- **a seal** — compression set and material behavior over time; the only one
  of the three where a *claim* would need a time axis the claims layer does
  not have
- **anything thermal** — a physics the whole stack currently ignores, and
  therefore the strongest test and the most expensive

Recommendation: the **seal**, because it stresses the claims layer and the
class set at once, and because a missing time axis would be a finding about
Layer 5 rather than only about the seven classes. This is a decision about
cal's design work and is recorded here as a recommendation, not a choice.

## Test 2 — a second agent's failures cluster differently

**Current evidence: untested, and it cannot be tested by me.** Every logged
entry in both corpus files is one agent's. A second run by the same agent is
not a second agent; it is a second sample of one.

**The experiment.** Another actor takes a harness part, logs to the same
schema with no sight of the existing corpus, and the two class distributions
are compared.

**What falsifies.** A materially different distribution. If the classes track
the agent rather than the problem, then A–G describes how one agent fails and
the spec's generality claim goes.

**Who.** Not me, by construction. The blind condition matters more than the
choice of actor: an actor who has read this corpus will classify into it.

## Test 4 — general use is mostly Layer E, and Layer E is mostly unevidenced

**Current evidence.** Every `[general]` badge is reasoned rather than logged,
and the spec says so. What *has* been measured since is the surface those
badges describe: the project calls 7 of the provider's 12 tools and about five
modes — a dozen behaviours, not thirty-eight thousand lines.

**The experiment.** A week on `.scad` files the author did not write, logged
the same way. The spec's own framing, unchanged.

**What falsifies.** The badges confirmed, or half of them replaced. Either is a
result; the failure mode is running the week and not logging it, which is how
every `[general]` badge got to be a guess in the first place.

## Test 5 — the provider boundary is in the wrong place

**Current evidence: has not fired, and there is now a mechanism explaining
why.** The test says staleness propagation needs to know which geometric facts
a claim consumed, and that if this requires the provider's *internals* —
which faces, which CSG nodes — then Witness Marks cannot sit above an MCP
surface and must vendor the kernel.

Dependency discovery turned out to be **observational, not introspective**.
`derived_inputs` moves an input and watches whether the claim's value moves;
that needs the provider's *answers* and nothing behind them. The provider's
own lexical `trace` is a cross-check rather than the source, and it is the
weaker of the two: by its own statement it models *"lexical, file-scope
constants only… module-local variables, parameters and conditional
redefinition are not modelled"*, and perturbation sees what it cannot.

So the boundary looks correctly placed, for a reason the spec did not state:
the question "what did this claim consume" is answerable by experiment on a
black box.

**What would still fire it, and this is new.** `derived_inputs` perturbs **one
input at a time**. A claim sensitive only to a *combination* — two inputs that
move the value together and neither alone — reads as reading nothing, and
lands in `Index::inert()` beside the genuine constants. The first real claim
that appears in `inert()` while demonstrably depending on its inputs is the
signal, and it would mean dependency discovery needs either joint perturbation
or something the provider does not expose.

**The cheapest experiment** is therefore not a new harness at all: take the
claims that exist, construct one with a deliberate interaction term, and
confirm it lands in `inert()`. That is a unit test, and it should be written
before any of the above.

## The scope-versus-measurement accounting

Issue 46 asks what fraction of lost time is scope rather than measurement.
**It is not computable from the corpus as it stands.** The schema carries
`id, source, n, title, class, class_why, state, measurement, fix` — the
`measurement` field says how the defect was found, not what it cost, and
nothing records whether the time went on measuring the wrong thing or on
building the wrong thing.

Spec §10 test 6 is blunt about the stake: *"Zero logged entries record building
the wrong thing, and the project's most expensive turn was exactly that."* A
corpus that cannot express the more expensive half cannot be used to argue the
project addresses it.

**The first step is a schema change, not an experiment**: a `cost` field per
incident, one of `measurement` or `scope`, with a `why`. Retrofitting it across
108 existing entries is a judgment call per entry and should be its own issue;
adding it for *new* entries costs nothing and starts the count.

---

## What this plan does not do

It does not run anything. Tests 1 and 4 need a design cycle and a working week;
test 2 needs an actor who is not me and has not read the corpus; the scope
accounting needs a schema change and then time to accumulate. The one piece
that can be done immediately is test 5's interaction-term unit test, which is
why it is named as the cheapest and recommended first.
