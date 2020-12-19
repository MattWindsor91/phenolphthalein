# phenolphthalein

_Phenolphthalein_ is an experiment to try and make a concurrency test runner that:

- can run fixed-input, fixed-threading C11 concurrency tests;
- is reasonably agnostic as to how those tests are implemented, requiring some common
  stub code but nothing more;
- is based on the model of a fairly heavyweight standalone test runner interfacing with
  separately compiled test code.

(**NOTE:** this is very much the kind of experiment that might result in failure, so
don't be getting any hopes up.)

In practice, it doesn't do anything yet, but watch this space.

Phenolphthalein is written in Rust (with some C interfacing code) and licenced under the
MIT licence.

## Why is it called phenolphthalein?

Similar purpose to [Litmus](https://github.com/herdtools/litmus7), but different execution.
Also, the incredibly long and difficult to spell name is a conscious effort to suggest to
the potential user that they should probably be using Litmus instead.
