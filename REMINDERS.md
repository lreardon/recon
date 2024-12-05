#### I circle back to this project with enough infrequency that it's become evident that I need to do a better job of documenting the process and progress of the project.

# Reminders

## Current Task

The task at hand is that of efficiently pruning and storing forms, in order to optimize the exploration process. To that end, we're making use of `fst`, a project for using finite state automata to store and efficiently search enormous collections of strings. For our purposes, we're most interested in a transducer, which ought to store the discovered strings along with their numerical values.

The ultimate goal is actually dual to the construction of this transducer - we want a hash-like object pairing numerical quantities to the minimal length of their representations.