# DICOMweb

**F-IDs that contributed:** F-021
**Last updated:** 2026-09-09

The DICOMweb path has two explicit owners. `@ocelli/core` owns HTTP fetch,
authentication injection, status handling, content negotiation, and
cancellation. `ocelli-dicom` owns a transport-neutral response contract,
DICOM JSON projection, multipart parsing, and Part 10 parsing. No Rust source
type performs I/O and no TypeScript code interprets DICOM metadata.

## Endpoint scope

`DicomwebClient` accepts separate Studies Service and WADO-URI URLs plus an
injected `fetch` function. The caller can add authentication inside that
function without exposing credentials to Ocelli.

The client supports these bounded GET operations:

- QIDO-RS study, series, and instance searches
- WADO-RS retrieval of one instance
- WADO-RS retrieval of an explicit nonempty frame list
- WADO-URI retrieval of one DICOM instance

Each QIDO matching attribute appears once. UID List matching uses one
comma-separated value. Only `includefield` repeats. `limit`, `offset`, and
`fuzzymatching` remain explicit options. The unsupported
`emptyvaluematching` and `multiplevaluematching` controls are reserved and
cannot escape through generic filters. WADO-URI always supplies
`requestType=WADO`, `studyUID`, `seriesUID`, `objectUID`, and
`contentType=application/dicom`.

Every study, series, instance, and WADO-URI object UID is validated before URL
construction or fetch. PS3.5 section 9.1 permits nonempty decimal components
separated by dots, forbids a leading zero except for the component `0`, and
limits the complete UID to 64 characters. Empty values, dot segments,
non-decimal components, leading-zero forms, and longer values are
`InvalidRequest`.

Automatic retry, authentication refresh, whole-study retrieval, whole-series
retrieval, rendered WADO-URI output, caching, and timeout policy are outside
this contract.

## Transport validation and errors

TypeScript requires HTTP success before reading the response body. It then
validates the outer content type before performing a bulk write. The stable
transport error classes are `InvalidRequest`, `Network`, `Aborted`,
`HttpStatus`, and `ContentType`. Only `HttpStatus` retains the numeric status.
Errors never retain a URL, UID, header, query, credential, or response body.

The injected fetch receives the caller's `AbortSignal` unchanged. Ocelli does
not retry a rejected fetch.

## Bulk boundary

`writeDicomwebResponse` lives in `bulk.ts`, an existing allowed linear-memory
view site. It allocates first, creates one fresh view after allocation, copies
the complete response once, and commits once with a `DicomwebResponseKind`.
The kinds are QIDO JSON, WADO-URI Part 10, WADO-RS instances with a boundary,
and WADO-RS frames with a boundary and media type.

The sink is an interface exercised by fixtures. F-101 owns the live Session
command and `commit_dicomweb_response` implementation. F-021 does not expose a
new wasm function or invent Part 10 ingress ahead of that story.

## Rust source contract

`SeriesSource::consume` accepts a `SourceResponse` containing owned bytes and
an explicit representation kind. `DicomwebSource` returns one of three
`SourceBatch` variants:

- `Query(Vec<MetadataSet>)`
- `Instances(Vec<ParsedDicom>)`
- `Frames(EncodedFrames)`

WADO-URI parses one Part 10 object directly. WADO-RS instance parts each pass
once through `parse_part10`. Encoded frames retain one owned multipart body
and ordered payload ranges into it. `part_bytes` borrows each range without a
second payload allocation or decode.

`SourceError` reports only a structural class plus a safe attribute or part
index and the patient-safe `ParseError` class. It stores no response values,
identifiers, header text, payload bytes, or upstream parser message.

## Multipart handling

The parser requires a declared boundary from the validated outer content
type. The boundary must be 1 to 70 legal MIME boundary characters and cannot
end in a space. Delimiters are recognized only as exact CRLF-framed boundary
lines with either a next-part or closing suffix. A boundary-like substring in
frame or Part 10 bytes is payload unless it has that complete framing.

Each part must begin with one nonempty `Content-Type`, one nonempty
`Content-Location`, and exactly one of `Content-Length` or `Transfer-Encoding`
in that order. A decimal `Content-Length` must equal the delimited payload
length. Duplicate or conflicting required headers are refused. Instance parts
require `application/dicom`. Frame parts require the caller-selected media
type including an order-independent comparison of its parameters. The full
per-part spelling and transfer-syntax parameter remain observable.

Required header names use the exact field-name grammar, so whitespace before
the colon is not normalized away. Content-Location is parsed as an RFC 3986
URI reference through `uriparse`. Absolute URIs and legal relative references
are accepted. Its spelling is discarded after validation and never enters a
`SourceError`.

## DICOM JSON projection

QIDO responses must be one top-level array of data-set objects. Attribute keys
must be uppercase eight-digit hexadecimal tags and every attribute must carry
a valid explicit `vr`. No carrier means a present empty element. Exactly one
of `Value`, `BulkDataURI`, or `InlineBinary` may be present. A
duplicate-detecting deserialization visitor rejects repeated JSON object
members before `serde_json::Value` can collapse them, including equal Tag keys
inside sequence item data sets. The resulting public error is structural and
retains no object name or response value.

Projection reuses F-017 `MetadataSet` and `MetadataElement` constructors. It
preserves array order and multiplicity, text source spelling, typed null slots,
signed and unsigned numeric widths, exact SV and UV strings, PN component
groups, AT values, ordered SQ items,
binary carrier identity, and absence versus present empty. Binary carrier VR
and canonical base64 validation stay centralized in the F-017 constructors.

`MetadataValue::WithNullSlots` stores the exact total multiplicity, strictly
increasing empty positions, and a compact existing typed value. Its checked
constructor rejects inconsistent counts, out-of-range or duplicate positions,
nesting, SQ, and binary carriers. Accessors expose both null positions and the
mapping from an original position to the compact typed index. An all-null
Value array retains its multiplicity. Empty SQ items remain empty JSON objects,
and SQ null slots are refused.

## Targets

The Rust implementation has no browser API, `wasm-bindgen`, or target-specific
code. It uses the workspace `serde` and `serde_json` dependencies and compiles
as part of `ocelli-dicom` for native and `wasm32-unknown-unknown`. Fetch
remains in the TypeScript shell.
