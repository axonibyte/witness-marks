# push-event fixtures, for `check-mutation-record.sh --from-event`

A CI job cannot know which commits a push carried except from the event
payload, so resolving that range is code, and code gets driven. These are the
shapes it must refuse. `ok.json` carries `PLACEHOLDER` where a sha goes: the
test substitutes a real one from the hermetic repo it builds, because a sha
written into a fixture would be a sha that resolves in no repository.

| file | what it is | required outcome |
| --- | --- | --- |
| `ok.json` | a normal push | the range resolves |
| `branch-created.json` | all-zero before: a new branch | REFUSE, there is no before-state |
| `no-before.json` | an event with no `before` key | REFUSE |
| `null-before.json` | `before` present and null | REFUSE |
| `garbage.json` | not JSON | REFUSE |
| `not-an-object.json` | valid JSON, wrong shape | REFUSE |

Every refusal is the SAME exit (66) on purpose: the caller's job is to stop,
and splitting "no key" from "bad JSON" would invite a caller to treat one of
them as recoverable. What must never happen is any of them passing.
