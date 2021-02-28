# phenolphthalein

_Phenolphthalein_ is an experimental concurrency test runner that:

- can run fixed-input, fixed-threading C11 concurrency tests
- is reasonably agnostic as to how those tests are implemented, requiring
  some common stub code but nothing more
- is based on the model of a fairly heavyweight standalone test runner
  interfacing with separately compiled test code, rather than integrated test
  harnesses

While still quite new, inefficient, and rough against the edges, it has
the following features:

- handles `SIGTERM` by returning partial results
- run tests indefinitely (or until the test fails or passes)
- output in machine-readable JSON as well as traditional histograms

Phenolphthalein is written in Rust (with some C interfacing code) and
licenced under the MIT licence.

## Why would I want to use this?

You probably don't yet - it's early beta-grade software.  But the intention is
that it'll be useful when
you want something like [Litmus](https://github.com/herdtools/litmus7), but:

- you need to support things that aren't, or don't fit well in, Litmus tests
- you'd prefer most of the test running infrastructure concentrated in one
  program, rather than being duplicated into test binaries
- you can somehow compile/run Rust binaries but not OCaml ones
- you need some of the above exotic features phenolphthalein has
- you don't need native support for assembly, cross-compilation, Litmus test
  ingestion, etc.

## How do I use this?

Using either the example test given in `main.c` or your own variant thereof, do
something like:

```shell
$ clang -dynamiclib -std=c11 -pedantic -O3 -o test.dylib test.c
$ cargo run --release [OPTIONS] test.dylib
```

### Options

`phph` accepts several arguments:

#### Test parameters

These can also be set globally using a TOML config file: pass
`--dump-config-path` instead of a test file to see where `phph` is looking for
one, and `--dump-config` to get the current config in the right format.

- `--iterations=N`: run `N` many iterations in total (set to `0` to disable
  iteration cap)
- `--period=N`: join and re-create threads every `N` iterations
  (set to `0` to disable thread rotation)
- `--check=TYPE`: control how phenolphthalein checks states against the test's
  postcondition: `disable` checks entirely; `report` the check outcomes per
  state; or `exit-on-pass`, `exit-on-fail`, or `exit-on-unknown` to abort the
  test when a particular outcome arrives
- `--permute=TYPE`: control the order in which phenolphthalein launches threads:
  either `static` or `random`
- `--sync=TYPE`: synchronise threads with a spinlock (`spinner`, default) or
  a full Rust barrier (`barrier`); `spinner` is faster and tends to show more
  weak behaviour, but `barrier` is perhaps 'safer'

#### Output control

- `--output-type=TYPE`: control the output format, with possibilities being a
  litmus7-style `histogram`, or a semi-machine-readable `json` serialisation

## How can I help?

All contributions are welcome!  Check the GitHub issues page for specific
things that need work.  General areas of development include:

- feature parity with litmus7 (though we don't intend to support
  every litmus7 feature, given the different focus for phenolphthalein);
- speed of test turnaround (likely taking hints from litmus7);
- ability to discover weak behaviours (likely also taking hints from litmus7); 
- reducing test boilerplate, to make it straightforward to write hand-written
  tests against phenolphthalein;
- supporting boilerplate and ABIs for more languages (C++, Rust, Go?)

## Why is it called phenolphthalein?

Similar purpose to [Litmus](https://github.com/herdtools/litmus7), but different execution.
Also, the incredibly long and difficult to spell name is a conscious effort to suggest to
the potential user that they should probably be using Litmus instead.
