#### I circle back to this project with enough infrequency that it's become evident that I need to do a better job of documenting the process and progress of the project.

# Reminders

## Current Task

Streamlining the exploration progress requires having a temporary file for newly found "candidate" expressions, and:
	1) periodically checking if they're included already in our persistent storage
	2) if they aren't, evaluating them
	3) if their evaluation is efficient, adding them to the evaluations `fst`.

To this end, we've been experimenting with creating fst files from txt and json files.

I think it's enough to compile a list of 5000 nullaries, then check inclusion against our fst. If not included, we should evaluate them, and check the values against the fst.

WE MUST AVOID EVALUATING NULLARIES THAT HAVE ALREADY BEEN EVALUATED.