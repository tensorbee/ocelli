//! F-224, the content id of the bytes one frame was decoded from.
//!
//! Every expected value below is computed by hand from the FNV-1a 64
//! definition in the IETF FNV draft, not from the implementation. The
//! definition is the offset basis 0xcbf29ce484222325, the prime
//! 0x00000100000001b3, and one step per byte `(hash XOR byte) * prime`
//! wrapping at 64 bits. The input is the length-prefixed fields in order:
//! the SOP Instance UID length as a little-endian `u64`, the UID, the
//! transfer syntax UID length as a little-endian `u64`, the transfer syntax
//! UID, the encoded length as a little-endian `u64` and the encoded bytes.
//! The working is shown at each assertion as the fold state after each
//! segment.

use ocelli_cache::ContentId;

#[test]
fn fixture_content_id_values() {
    // uid "1.2.3" is 5 bytes, the transfer syntax "1.2.840.10008.1.2.1" is
    // 19 bytes and the encoded buffer is 4 bytes. The fold states are, from
    // the basis 0xcbf29ce484222325: after the eight bytes of 5
    // little-endian 0x0de21504f16dc720, after "1.2.3"
    // 0xa8974e07696603d0, after the eight bytes of 19 little-endian
    // 0x23094caa3449da23, after "1.2.840.10008.1.2.1"
    // 0x8c0a80dcd87f1661, after the eight bytes of 4 little-endian
    // 0x20bfd51f7f261505, and after 00 01 02 03 the value below.
    assert_eq!(
        ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", &[0, 1, 2, 3]).0,
        0x49e8_b650_ff1c_d3f1,
    );

    // The same buffer with its last byte 4 instead of 3 changes one fold
    // step, and the state cannot return to a previous value because the
    // prime is odd. 0x49e8b750ff1cd5a4, hand-computed the same way.
    assert_eq!(
        ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", &[0, 1, 2, 4]).0,
        0x49e8_b750_ff1c_d5a4,
    );

    // A different SOP Instance UID, same transfer syntax and buffer. The
    // fold state after "1.2.4" differs from the one after "1.2.3" and stays
    // different. 0xaa81c9a83be590b6.
    assert_eq!(
        ContentId::of(b"1.2.4", b"1.2.840.10008.1.2.1", &[0, 1, 2, 3]).0,
        0xaa81_c9a8_3be5_90b6,
    );

    // Three empty fields are three zero lengths, 24 zero bytes folded from
    // the basis. After the first eight zeros 0xa8c7f832281a39c5, after the
    // second 0x88201fb960ff6465, after the third 0x81d23fd7003c2305.
    assert_eq!(ContentId::of(b"", b"", b"").0, 0x81d2_3fd7_003c_2305,);
}

#[test]
fn fixture_field_boundaries() {
    // Decision 2's length prefixes make the field boundaries unambiguous.
    // Without them both rows below would fold "ab" and "c" or "a" and "bc"
    // into the same 24 bytes and both ids would be FNV-1a of "abc",
    // 0xe71fa2190541574b. With them the ids are the distinct values below.
    //
    // For uid "ab" and transfer syntax "c", from the basis: after the eight
    // bytes of 2 little-endian 0xe6bd86443df8ce07, after "ab"
    // 0x9c6001d328a94690, after the eight bytes of 1 little-endian
    // 0xc8090adcb79702b1, after "c" 0x7e60470bf599cad6, and after the eight
    // bytes of 0 the value below.
    assert_eq!(ContentId::of(b"ab", b"c", b"").0, 0x42c5_658f_d73f_0196,);

    // For uid "a" and transfer syntax "bc": after the eight bytes of 1
    // little-endian 0x89cd31291d2aefa4, after "a" 0x529a4ddc8ff56bbf, after
    // the eight bytes of 2 little-endian 0xc1e27c05773a855d, after "bc"
    // 0xba1e1f0e0704d8ea, and after the eight bytes of 0 the value below.
    assert_eq!(ContentId::of(b"a", b"bc", b"").0, 0x8188_530f_f4c9_9a2a,);
}

#[test]
fn property_one_byte_change_changes_the_id() {
    // Decision 2's one-byte property, over an 8-byte encoded buffer. Every
    // single-byte change is all 8 positions times the 255 other byte values,
    // 2040 inputs, and each must differ from the base id. This rejects a
    // digest of the length alone and a digest of a prefix of the bytes,
    // because both leave later positions unread.
    let encoded = [0_u8, 1, 2, 3, 4, 5, 6, 7];
    let base = ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", &encoded);
    for (position, original) in encoded.into_iter().enumerate() {
        for value in 0..=255_u8 {
            if value == original {
                continue;
            }
            let changed: Vec<u8> = encoded
                .iter()
                .enumerate()
                .map(|(index, byte)| if index == position { value } else { *byte })
                .collect();
            assert_ne!(
                ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", &changed).0,
                base.0,
                "position {position} held {value} and kept the base id",
            );
        }
    }
}
