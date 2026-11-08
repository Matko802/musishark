# stacksafe-macro

A local copy of `stacksafe-macro` 0.1.4 from https://github.com/fast/stacksafe
(Apache-2.0), with the unused `proc-macro-error2` dependency removed.

Musishark does not invoke the macro in an error state, so the dependency only
existed for the crate's own compile diagnostics.
