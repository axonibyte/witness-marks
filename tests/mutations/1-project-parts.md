# Mutation record: project parts and staleness

| mutation | killed by |
| --- | --- |
| a part never exported reads as Current | a_part_never_exported_is_not_current |
| `is_trustworthy` admits everything except Behind | a_part_never_exported_is_not_current |
| the staleness report omits never-exported parts | a_model_with_no_exports_at_all_reports_every_part |
| every `module` in the file is taken for a part | a_module_is_a_part_only_when_the_caller_names_it |
| `show_` flags are not read at all | the_harnesss_parts_are_found_from_its_show_flags |

## Unknown is not current, twice over

The first two mutants are the same rule from two directions. An absent export
record and a matching one both produce "nothing to do", and only one of them
means the artifact can be trusted. The second mutant is the subtle one: an
`is_trustworthy` written as "not Behind" looks equivalent and quietly blesses
every part nobody has ever exported.

The third is the same rule at the report level: a model nobody has exported
would produce an EMPTY staleness report, which reads as fully up to date.

## A misfire, and my own mutant file was the defect

`project-guesses-every-module` first reported MISFIRED. The mutation text was
written through a shell `printf` with escaped quotes and did not survive as
valid Rust, so the build failed and the named test never ran. mutate.sh
refused to count it, which is the third time that refusal has caught a
build-breaking mutant. The mutant files are written with Python now, which
does not mangle quotes.

## A test arm of mine that was simply wrong

`an_export_from_this_source_is_current` first asserted an empty staleness
report while passing the whole fixture, which pulls in five `show_` parts that
were never recorded. The report was rightly non-empty. The arm was wrong, not
the code, and it is scoped to the one part now.

## Tiers

Touched: Tier 1, over the real harness file vendored in Foundations. Skipped:
every other tier.
