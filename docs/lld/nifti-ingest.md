# NIfTI ingest

**F-IDs that contributed:** F-022
**Last updated:** 2026-09-09

`ocelli-dicom::parse_nifti` is a direct, safe, in-memory parser for one bounded
NIfTI-1.1 profile. The wire format and its RAS coordinates remain separate
from DICOM parsing and DICOM LPS world coordinates.

The format source is the official
[`nifti1.h`](https://github.com/NIFTI-Imaging/nifti_clib/blob/master/niftilib/nifti1.h).
Distinct NIfTI-2 recognition uses the official
[`nifti2.h`](https://github.com/NIFTI-Imaging/nifti_clib/blob/master/nifti2/nifti2.h).

## Accepted profile

The parser accepts uncompressed little-endian single-file `n+1` input. It
refuses gzip, NIfTI-2, paired NIfTI-1 `ni1`, header extensions, and big-endian
NIfTI-1 without retrying the input under another interpretation.

The complete NIfTI-1 header is 348 bytes. A single-file input also contains
the four-byte extender, so the minimum input and payload offset are 352 bytes.
The parser reads these fields at their specified byte offsets:

| Bytes | Field | Accepted contract |
|-------|-------|-------------------|
| 0..4 | `sizeof_hdr` | little-endian i32 equal to 348 |
| 40..56 | `dim[8]` | rank 3 through 7, three positive spatial dimensions, declared higher dimensions equal to 1 |
| 70..72 | `datatype` | 2, 4, 16, or 512 |
| 72..74 | `bitpix` | exact width paired with the datatype |
| 76..108 | `pixdim[8]` | finite positive spatial widths and raw qfac |
| 108..112 | `vox_offset` | finite exact integer at least 352 and representable as `usize` |
| 112..120 | `scl_slope`, `scl_inter` | retained exactly without application |
| 123 | `xyzt_units` | spatial bits select millimetres |
| 252..280 | qform | signed code, quaternion, and RAS translation |
| 280..328 | sform | three general RAS affine rows |
| 344..348 | `magic` | exact `n+1\0` |
| 348..352 | extender | first byte zero |

Only dimensions 4 through `dim[0]` are declared higher dimensions. Slots
above the declared rank are ignored. Temporal unit bits are independent from
the low three spatial unit bits. The low bits must equal
`NIFTI_UNITS_MM`, value 2.

## Datatypes and payload

| Public kind | NIfTI datatype | `bitpix` | Bytes per voxel |
|-------------|----------------|----------|-----------------|
| `U8` | 2 | 8 | 1 |
| `I16` | 4 | 16 | 2 |
| `U16` | 512 | 16 | 2 |
| `F32` | 16 | 32 | 4 |

The payload is x fastest, then y, then z. Its length is the checked product of
the three spatial dimensions and bytes per voxel. `ParsedNifti` retains the
exact `vox_offset..payload_end` range without copying or interpreting payload
values. Input ending before that range is refused. Bytes after the range are
allowed.

`vox_offset` is decoded from its IEEE-754 representation into an exact integer
without an `as` cast. Values below 352 are an Ocelli product-scope refusal.
NIfTI-1 itself treats them as 352. A 16-byte-aligned offset is recommended by
the format but is not required here. An offset greater than 352 is valid when
the extension flag is zero.

The raw `scl_slope` and `scl_inter` bit patterns are retained as f32 values.
This story does not apply scaling and does not reject non-finite scaling
fields.

## Affine declarations and selection

`NiftiHeader` retains the raw signed codes and parameters for both qform and
sform. Positive codes activate a form. A negative code is invalid and zero is
inactive. An active sform takes precedence over an active qform. Differing
active forms remain observable through their raw declarations and
`NiftiAffineSource` records which one was selected.

Only the selected declaration is constructed and checked as an affine. An
inactive or unselected declaration may therefore retain non-finite raw fields
without changing the selected geometry.

The sform rows map `(i,j,k,1)` directly to NIfTI RAS `(x,y,z,1)`. The qform
uses the official unit-quaternion matrix. `pixdim[1..=3]` scale its three
columns and `pixdim[0]` supplies qfac for the third. A raw qfac of zero means
effective positive one while remaining zero in the retained declaration.
Only raw qfac values `-1`, `0`, and `1` are accepted. A quaternion with
`b*b + c*c + d*d > 1` is refused without normalisation or a tolerance.

Every component of the selected affine and its determinant must be finite.
An exactly zero determinant is singular. No epsilon is introduced. Validation
happens before calling the unchecked `Transform::from_mat4`, and
`Transform::inverse` is not used as a validation mechanism.

## RAS to LPS conversion

NIfTI subject coordinates use Right, Anterior, Superior axes. Ocelli `World`
uses DICOM Left, Posterior, Superior axes in millimetres. With column vectors,
the conversion is:

```text
M_index_to_lps = diag(-1, -1, 1, 1) * M_index_to_ras
```

The conversion is left multiplication. It negates the first two complete
rows, including x and y translation. The public geometry is only a
`Transform<Index, World>`. The raw RAS matrix is not exposed as a world-space
transform.

## Refusal boundary

`NiftiError` distinguishes unsupported compression, version, container,
extensions, and byte order from truncated headers. It separately distinguishes
header size and magic, datatype and bit-width mismatch, rank, spatial and
higher dimensions, units, affine codes, absent affine, quaternion, qfac,
non-finite and singular selected affine, voxel offset, arithmetic overflow,
and truncated payload.

Errors contain only their structural class. They retain no source bytes,
header text, paths, identifiers, or payload values.

## Targets and evidence

The module has no I/O, compression, target-specific branch, `unsafe`, or
`wasm-bindgen`. It compiles as part of the same `ocelli-dicom` library on
native and `wasm32-unknown-unknown`.

Synthetic fixtures cover all supported datatypes and every refusal class. A
non-symmetric sform and a non-identity qform verify the origin and three basis
points against hand-computed LPS millimetres. A property test checks complete
and one-byte-short payload products. The private checked-length helper is used
as `usize` in production and `u32` with maximum NIfTI-1 spatial dimensions in
its overflow proof.
