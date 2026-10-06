# Third-party notices

This crate is a native Rust implementation of the behavior of Python PrettyTable.
The reference is PrettyTable 3.18.0, commit
`069405f00ea1075e28dc738b8c52fd3bff173520`.

PrettyTable is Copyright (c) 2009–2014 Luke Maurits, with contributions from
Chris Clark, Klein Stephane, John Filleau, Vladimir Vrzić, and the project's
contributors. Its BSD-3-Clause notice is retained in the root LICENSE file.
The upstream output fixtures and test-derived examples are distributed under
that notice. The excluded upstream checkout retains its original notices.

Word-wrapping behavior also follows CPython's `textwrap` algorithm. The Python
license and historical notices are retained in PYTHON_LICENSE.txt.

Unicode and ANSI wrapping behavior was studied against wcwidth 0.9.2. Its MIT
notice is retained in WCWIDTH_LICENSE.txt. Unicode segmentation and display
width tables are supplied by the unicode-segmentation and unicode-width Rust
crates; their licenses are distributed with those dependencies.

The developer-only fixture generator runs the upstream tests to capture an
independent reference. It does not run during normal Rust builds or tests.
