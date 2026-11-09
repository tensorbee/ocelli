# dicom-object 0.10.0 provenance

This directory contains the published `dicom-object` 0.10.0 package from
crates.io. The downloaded `.crate` archive has SHA-256 digest
`0504b87987f0e7624cf7ee063fe7ffdb438c66c092620817fb9bb0f130686b85`.
Its `.cargo_vcs_info.json` identifies upstream Git commit
`39603cef65da125e47daffbb32310c6826f2f62b` and path `object`. That commit
is not reachable on the upstream repository. The upstream `v0.10.0` tag,
commit `2941c54cae434dd63a54d0a5546b3b0df42e9f6c`, carries `object/src`
files byte-identical to the package's `src/collector.rs`, `src/lib.rs` and
`src/mem.rs`.

`PACKAGE-INVENTORY.sha256` records every file in the unmodified published
package. No published file was removed. Ocelli changes one file, under F-244
and deviation D-18.

## The patch

`src/collector.rs`, function `build_encapsulated_data`. One line is added as
the last statement of the `LazyDataToken::ItemEnd` arm:

```rust
first = false;
```

DICOM PS3.5 A.4 says the first Item of encapsulated Pixel Data is the Basic
Offset Table, and that "When the Item Value is not present, the Item Length
shall be zero". The published function clears its first-item flag only when
an Item value is read, and a zero-length Item has none, so the first Fragment
after an empty table was read as the table. The added line clears the flag at
the end of every Item, which is the end of the table for the first one. A
populated table has already cleared it, so its behaviour is unchanged.

The patched `src/collector.rs` has SHA-256 digest
`69f08d0ebf59e3d4fe377912d8e0729d79f980d266500d378d49558413620656`.

dicom-object 0.10.1 and upstream `master` carry the same function, measured
on 2026-11-08. A release carrying the fix replaces this directory with the
registry dependency.

## Licence

The `LICENSE-MIT` and `LICENSE-APACHE` files are the upstream text at the
`v0.10.0` tag, commit `2941c54cae434dd63a54d0a5546b3b0df42e9f6c`, because the
packaged commit is not reachable. The package declares
`license = "MIT OR Apache-2.0"`. Ocelli selects the MIT alternative for
redistribution. Their SHA-256 digests are:

- `LICENSE-MIT`: `77fbf925c55b05c210868fa639de038b4f01d9a19b9309f7db46f32ce0d5661f`
- `LICENSE-APACHE`: `ae8a8e2f408f02b92bbaa5a4b482209288ec1d9dac85b5722b001040f1f13a9d`
