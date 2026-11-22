# fanal-core

Grey is closed, yellow is open and idle, green is a task executing, and red is a confirmation waiting on a person. Red outranks green, then yellow, then grey. A closed program is never red.

`judge` does not expire a reading while its process lives. A file last written before that process started belongs to an earlier run.

A catalog id missing from a saved settings file stays off. Do not default a new id to on.
