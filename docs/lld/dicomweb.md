# DICOMweb

**F-IDs that contributed:** F-021, F-101, F-198, F-230, F-224, F-245
**Last updated:** 2026-11-11

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
retrieval, rendered WADO-URI output, response caching of its own, and timeout
policy are outside this contract.

## Transport validation and errors

TypeScript requires HTTP success before reading the response body. It then
validates the outer content type before performing a bulk write. The stable
transport error classes are `InvalidRequest`, `Network`, `Aborted`,
`HttpStatus`, and `ContentType`. Only `HttpStatus` retains the numeric status.
Errors never retain a URL, UID, header, query, credential, or response body.

`DicomwebError` extends `OcelliError`, so `instanceof OcelliError` catches it.
Its `name` is `DicomwebError`, its `code` is one of the five transport classes,
its message is fixed per code, and it carries no `cause`.

An unknown own key on `DicomwebClientOptions` or `QidoSearchOptions` is not a
transport class. It is `OcelliError` code `UnknownOption`, raised before any
fetch, and `error.option` names the refused key and nothing else. The
constructor throws it. `searchStudies`, `searchSeries` and `searchInstances`
reject their promise with it. `postWadoInstance` refuses an unknown key on
`WadoInstanceRequest` the same way. A value that is not an options object is
`InvalidOptions`. `docs/lld/public-api.md`, "Options and refusals", holds the
rule.

The injected fetch receives the caller's `AbortSignal` unchanged. Ocelli does
not retry a rejected fetch.

## The fetch cache mode

Every request is sent with a fetch cache mode. `DicomwebClientOptions` takes
an optional `cache`, and `postWadoInstance` takes the same field on
`WadoInstanceRequest` and passes it through. The accepted values are
`"no-cache"`, `"no-store"`, `"reload"` and `"default"`, and the default is
`"no-cache"`. `"no-cache"` uses a stored response only after a conditional
request to the server, which closes the RFC 9111 heuristic freshness path a
server sending no `Cache-Control` would otherwise open, and a server with
validators answers 304 so an unchanged object is not downloaded twice.
`"default"` is an explicit opt-in to the browser's heuristics.
`"force-cache"` and `"only-if-cached"` return a stored response whether or
not it is stale, and are refused at construction with `DicomwebError` code
`InvalidRequest` before any fetch, as is any other string. A proxy or a
service worker between the browser and the server is outside what a cache
mode controls.

## Bulk boundary

`writeDicomwebResponse` lives in `bulk.ts`, an existing allowed linear-memory
view site. It allocates first, creates one fresh view after allocation, copies
the complete response once, and commits once with a `DicomwebResponseKind`.
The kinds are QIDO JSON, WADO-URI Part 10, WADO-RS instances with a boundary,
and WADO-RS frames with a boundary and media type.

The sink is an interface exercised by fixtures. F-101 built the live Session
command channel, its ticketed allocation and its frame commit. It did not
build `commit_dicomweb_response`, and F-021 still exposes no wasm function and
invents no Part 10 ingress. That response commit is an open boundary gap with
no scheduled F-ID, recorded here rather than left as an implied promise.

The example page fetches one WADO-RS instance through `DicomwebClient` and
posts that response body to its worker. The body is the multipart envelope.
The worker also keeps the boundary from that response. The worker copies the
bytes into the session bulk slot with `holdSessionBytes`. That ticket is the
session's only bulk slot, so a later frame or command allocation refuses
until the ticket is released. It does not call `commit_dicomweb_response`,
and that wasm entry is still absent. The worker binds one offscreen canvas,
commits one monochrome stored sample through the existing frame commit, and
publishes with `publishCpuCanvas`. The page shows the tier string from
`resolvedTier` and whether that publish returned. A failed load rejects
`loadCore` with `CoreLoadFailed` and leaves `coreAvailable()` false.

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

## The viewer provider and its native routes

F-245 adds `DicomwebProvider` in `packages/core/src/dicomweb-viewer.ts`,
exported from `@ocelli/core`. It composes on top of the unchanged
`DicomwebClient`: the client keeps fetch, status handling, abort and the
bounded delivery branch, and the provider adds typed QIDO envelopes and one
staged WADO-RS instance. Its constructor takes `studiesServiceUrl`,
`wadoUriUrl` and `fetch`. The methods are `searchStudies`,
`searchSeries(studyUid)`, `searchInstances(studyUid, seriesUid)`,
`fetchSelectedInstance(identity)`, `readStagedInfo`, `takeStagedInstance`,
`releaseStaged`, `cancelPending` and `dispose`.

A QIDO envelope is frozen and carries `level`, `capturedScope`, `rows`,
`sourceNonce` as 32 lowercase hex, `generation` as a safe decimal no greater
than 9007199254740991, and `rawBodySha256`. Study scope has zero keys, series
scope exactly `studyUid`, and instance scope exactly `studyUid` and
`seriesUid`. Study rows carry only `studyUid`, series rows `seriesUid` plus an
optional `returnedStudyUid`, and instance rows `instanceUid` plus optional
`returnedStudyUid` and `returnedSeriesUid`. The Rust decoder validates the
shapes and the parent matching, and no browser JavaScript parses upstream
DICOM JSON.

`fetchSelectedInstance` stages one instance. The staged receipt is deeply
frozen and carries the nested `Readonly` `InstanceIdentity`, `rawBodySha256`,
`sourceNonce`, `generation`, `byteLength` and a representation of field
`wado-rs-instances` and the validated boundary. `takeStagedInstance(expected)`
deep-compares every field against the caller's expected receipt and returns
the owned body once, and stale, missing or already taken state returns null.
A new intent or a cancel releases the staged body only. The fixed product
status is that the instance is fetched and validated, remote image display is
not available yet, and the local image is unchanged. F-202 owns remote
display, and no decode or render runs on this path.

The native viewer serves `GET /dicweb/config` with the two full URLs when the
pair of `OCELLI_VIEWER_STUDIES_SERVICE_URL` and `OCELLI_VIEWER_WADO_URI_URL`
startup values is present, and answers the probe with a JSON null body when
the pair is absent, so an unconfigured page load logs no missing route. A URL
pair on port 0 is refused at startup, because a browser cannot fetch it. The
native `POST /dicweb/qido` and `POST /dicweb/instance` routes exist only when
the pair is present, and answer with the `X-Qido-` and `X-Wado-` prefixed
metadata headers then. The reply serialization cap belongs to the viewer
layer, at a fixed
16 MiB reply with a 2 MiB query body cap, a 32 MiB encoded instance cap and a
128 MiB decoded cap. The parser limits live in `crates/ocelli-dicom`, and
`docs/lld/dicom-ingest.md` records their signatures.

## Targets

The Rust implementation has no browser API, `wasm-bindgen`, or target-specific
code. It uses the workspace `serde` and `serde_json` dependencies and compiles
as part of `ocelli-dicom` for native and `wasm32-unknown-unknown`. Fetch
remains in the TypeScript shell.
