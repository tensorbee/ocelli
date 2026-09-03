# Security policy

## Reporting a vulnerability

Report privately through GitHub's **Report a vulnerability** button on the
Security tab of this repository. Please do not open a public issue for a
vulnerability.

**Never include patient data in a report.** Not in a description, not in a
stack trace, not as an attachment. If a defect can only be demonstrated with a
real study, say so and describe the shape of the data rather than sending it.
We will work out a way to reproduce it that does not move a patient record.

## What counts as a security issue here

This is a medical imaging library, so the category is wider than usual.
Alongside the ordinary classes, we treat these as security issues:

- **A wrong pixel value.** Anything causing the rendered result to differ from
  the DICOM standard, particularly in the LUT chain, rescale, or the VOI
  functions. A plausible image with incorrect values is the defect class this
  project is built to prevent.
- **Wrong geometry.** Incorrect spacing, slice ordering, or coordinate
  transforms, which make a measurement wrong while looking correct.
- **A feature that degrades silently.** A capability that cannot run on the
  resolved hardware tier must report itself unavailable. One that quietly
  produces a different answer instead is a safety issue, not a papercut.
- **Data escaping the browser.** Pixel data is designed never to leave the
  client. A path that sends it anywhere is a defect regardless of intent.

## Supported versions

Pre-1.0 and under active development. Only the latest release is supported.
Version 1.0 arrives with feature parity and the semantic-versioning and
deprecation policy that goes with it.

## This library is not a medical device

Ocelli is a component. It carries no regulatory clearance and makes no clinical
claim. An integrator building a cleared product is responsible for that
product's validation. What this project provides toward it is measured
evidence: a differential corpus, a written tolerance policy, and a published
divergence bound rather than a claim of exactness.
