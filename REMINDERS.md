#### I circle back to this project with enough infrequency that it's become evident that I need to do a better job of documenting the process and progress of the project.

# Reminders

## Current Task

Streamlining the exploration progress requires having a temporary file for newly found "candidate" expressions, and: 1) periodically checking if they're included already in our persistent storage 2) if they aren't, evaluating them 3) if their evaluation is efficient, adding them to the evaluations `fst`.

To this end, we've been experimenting with creating fst files from txt and json files.

I think it's enough to compile a list of 5000 nullaries, then check inclusion against our fst. If not included, we should evaluate them, and check the values against the fst.

WE MUST AVOID EVALUATING NULLARIES THAT HAVE ALREADY BEEN EVALUATED.

At this point it seems like we're able to add and explore appropriately. We still need to work on saving progress and restarting, and we need to work on the following additional challenge:

The evaluations need to be minimized. The evaluations.fst object contains all the evaluations, and that's well and good.
However, for the purpose of graphing, we will want a pipeline that processes evaluations.fst and extracts only the minimal evaluation for each numer, creating a hash in the process.
I think it will be expensive to run through the evaluations each time - ideally the minimal evalutions hash would be built alongside the evalutions.fst object during exploration.

Then, periodically, we could audit the hash against the evalutations to ensure that it remains faithful.

The only dependency for the minimal evaluations hash ought to be the evaluations fst object.
