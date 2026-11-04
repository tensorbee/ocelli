# Key object and secondary capture

**F-IDs that contributed:** F-196
**Last updated:** 2026-11-04

`writeKeyObject` and `readKeyObject` live in `packages/core`. dcmjs 0.52.2 writes one Key Object Selection document. The SOP class is `1.2.840.10008.5.1.4.1.1.88.59`. The template identifier is 2010. PS3.16 TID 2010 row 1 is the coded title `(113000, DCM, "Of Interest")`. The caller title is row 7, `(113012, DCM, "Key Object Description")`. Row 10 is one COMPOSITE reference. The reader returns that title and the referenced SOP class and instance. A TID 1500 document returns nothing. Changing only the template identifier to 1500 returns nothing. An empty title, a title of only spaces, or an empty UID returns nothing. Study, series and the document instance must be synthetic `2.25` values. The referenced instance is any well-formed DICOM UID. There is no patient module and no Rust twin.

`writeSecondaryCapture` and `readSecondaryCapture` write one 8-bit `MONOCHROME2` Secondary Capture image. The SOP class is `1.2.840.10008.5.1.4.1.1.7`. PS3.3 C.7.6.3 stores the caller bytes when their length is columns times rows. Columns and rows are at most 65535. An odd count is padded to an even length on write, and the reader returns the caller bytes without that pad. C.8.6.2 conversion type is `WSD`. The file has no window and no patient module. A short buffer returns nothing. The same inputs produce the same bytes.

The example page shows the title Key image, the referenced CT SOP, and pixels 0, 1, 2, 3.
