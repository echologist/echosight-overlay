# Third-party source

`src/components/text_input.rs` derives from Zed Industries' `crates/gpui/examples/input.rs`
at commit `4ed72e0135aaab74a43914491c915dd6cbd10c67` of `zed-industries/zed`.
The GPUI crate declares Apache-2.0 in its Cargo manifest and includes LICENSE-APACHE;
that license is reproduced here unchanged. The upstream example has no file-level
copyright notice and no applicable NOTICE file was present in that pinned checkout.
The adapted file remains Apache-2.0 (not the workspace's default MIT license).

Echosight modifications: configurable styles/placeholder, submit/cancel/change events,
UTF-16 length limit and single-line normalization, selection shortcuts and IME boundary
handling, horizontal caret visibility, headless interaction tests.

Other new Echosight source in this crate is MIT, under the repository LICENSE.
Redistributed spike binaries must be accompanied by this notice and LICENSE-APACHE;
the spike also embeds both texts for inspection with `--licenses`.
