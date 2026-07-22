// Extension for the binary_reader so I don't have to write the same code multiple times.
pub mod br_ext {
    use std::io::Error;

    use binary_reader::{BinaryReader, Endian};
    use encoding_rs::{UTF_16BE, UTF_16LE, SHIFT_JIS};

    pub struct BinaryReaderExtensions {

    }

    /// Length of the NUL-terminated prefix of a fixed-width field: the index of the
    /// first NUL byte, or the whole field when it has none.
    ///
    /// Both call sites previously open-coded this and both were off by one — see the
    /// `nul_prefix_len` tests. The BND4 header's 8-byte version field is the case that
    /// mattered: it holds `11611000` (regulation 11611000 = game 1.16.1) with no NUL at
    /// all, and the old rule decoded it as `1161100`.
    pub fn nul_prefix_len(bytes: &[u8]) -> usize {
        bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
    }

    /// The UTF-16 counterpart: length in BYTES of the prefix before the first NUL *code
    /// unit*, or the whole field (truncated to a whole number of code units) when it has
    /// none.
    ///
    /// A single zero byte is not a terminator here — every ASCII character in UTF-16
    /// carries one — so the scan has to step by pairs. Same trailing-byte trap as
    /// `nul_prefix_len`: a fully-used field ends without a terminator and the last code
    /// unit is still part of the string.
    pub fn nul16_prefix_len(bytes: &[u8]) -> usize {
        let usable = bytes.len() - bytes.len() % 2;
        (0..usable)
            .step_by(2)
            .find(|&i| bytes[i] == 0 && bytes[i + 1] == 0)
            .unwrap_or(usable)
    }

    #[allow(unused)]
    impl BinaryReaderExtensions {

        pub fn get_i8(br: &mut BinaryReader, offset: usize) -> Result<i8, Error> {
            let prev_pos = br.pos;
            br.jmp(offset);
            let byte = br.read_i8()?;
            br.jmp(prev_pos);
            Ok(byte)
        }

        pub fn get_i16(br: &mut BinaryReader, offset: usize) -> Result<i16, Error> {
            let prev_pos = br.pos;
            br.jmp(offset);
            let byte = br.read_i16()?;
            br.jmp(prev_pos);
            Ok(byte)
        }

        pub fn get_utf_16(br: &mut BinaryReader, offset: usize) -> Result<String, Error> {
            let prev_pos = br.pos;
            br.jmp(offset);
            let mut bytes: Vec<u8>= Vec::new();
                let mut pair = br.read_bytes(2)?;
                while pair[0] != 0 || pair[1] != 0 {
                    bytes.extend(pair);
                    pair = br.read_bytes(2)?;
                }
                
                let string = match br.endian {
                    binary_reader::Endian::Big => {
                        let (res, _enc, errors) = UTF_16BE.decode(&bytes);
                        if errors {
                            eprintln!("Failed");
                            String::new()
                        } else {
                            res.to_string()
                        }   
                    },
                    binary_reader::Endian::Little => {
                        let (res, _enc, errors) = UTF_16LE.decode(&bytes);
                        if errors {
                            eprintln!("Failed");
                            String::new()
                        } else {
                            res.to_string()
                        }   
                    },
                    binary_reader::Endian::Native => {panic!("Endian type is wrong!")},
                    binary_reader::Endian::Network => {panic!("Endian type is wrong!")},
                };

            br.jmp(prev_pos);
            Ok(string)
        }

        /// Fixed-width **UTF-16** string field. The byte-oriented sibling is
        /// `read_fix_str`; the `_w` suffix is the whole difference between them.
        pub fn read_fix_str_w(br: &mut BinaryReader, size: usize) -> Result<String, Error> {
            let big_endian = br.endian == Endian::Big;
            let bytes = br.read_bytes(size)?;
            let terminator = super::br_ext::nul16_prefix_len(bytes);

            let string = if big_endian {
                let (res, _enc, errors) = UTF_16BE.decode(&bytes[0..terminator]);
                if errors {
                    eprintln!("Failed");
                    String::new()
                } else {
                    res.to_string()
                }   
            }
            else {
                let (res, _enc, errors) = UTF_16LE.decode(&bytes[0..terminator]);
                if errors {
                    eprintln!("Failed");
                    String::new()
                } else {
                    res.to_string()
                }   
            };
            Ok(string)
        }

        pub fn get_ascii(br: &mut BinaryReader, offset: usize) -> Result<String, Error>{
            let prev_pos = br.pos;
            br.jmp(offset);
            let string = Self::read_ascii(br)?;
            br.jmp(prev_pos);
            Ok(string)
        }

        pub fn read_ascii(br: &mut BinaryReader) -> Result<String, Error> {
            let bytes = Self::read_chars_terminated(br)?;
            let res = String::from_utf8(bytes);

            match res {
                Ok(str) => Ok(str),
                Err(err) => panic!("Failed to read string: {err}"),
            }
        }

        /// Fixed-width **single-byte** (Shift-JIS) string field, NUL-padded.
        ///
        /// The single-zero-byte terminator scan and Shift-JIS go together; they are the
        /// same assumption stated twice. This used to scan for one zero byte and then
        /// decode the result as UTF-16, which is broken for every input:
        /// a real single-byte field ("ACTION_BUTTON_PARAM_ST") came back as the
        /// mojibake "䍁䥔乏䉟呕佔彎䅐䅒彍呓", and a genuine UTF-16 field came back empty
        /// because ASCII in UTF-16 has a zero high byte on the first character, so the
        /// scan stopped at index 1 and the odd single byte failed to decode.
        /// Endianness does not enter into a byte encoding, so there is no branch on it.
        /// UTF-16 fields belong in `read_fix_str_w`.
        pub fn read_fix_str(br: &mut BinaryReader, size: usize) -> Result<String, Error> {
            let bytes = br.read_bytes(size)?;
            let terminator = super::br_ext::nul_prefix_len(bytes);

            let (res, _enc, errors) = SHIFT_JIS.decode(&bytes[0..terminator]);
            if errors {
                eprintln!("Failed");
                Ok(String::new())
            } else {
                Ok(res.to_string())
            }
        }


        pub fn get_shift_jis(br: &mut BinaryReader, offset: usize) -> Result<String, Error> {
            let prev_pos = br.pos;
            br.jmp(offset);
            let string = Self::read_shift_jis(br)?;
            br.jmp(prev_pos);
            Ok(string)
        }

        pub fn read_shift_jis(br: &mut BinaryReader) -> Result<String, Error> {
            let bytes = Self::read_chars_terminated(br)?;
            let (res, _enc, errors) = SHIFT_JIS.decode(&bytes);
            if errors {
                panic!("Failed to read string!");
            } else {
                Ok(res.to_string())
            }  
        }

        fn read_chars_terminated(br: &mut BinaryReader) -> Result<Vec<u8>, Error> {
            let mut bytes: Vec<u8> = Vec::new();
            let mut b = br.read_u8()?;
            while b != 0 {
                bytes.push(b);
                b = br.read_u8()?;
            }
            Ok(bytes) 
        }

        pub fn get_bytes(br: &mut BinaryReader, offset: usize, size: usize) -> Result<Vec<u8>, Error> {
            let prev_pos = br.pos;
            br.jmp(offset);
            let bytes = br.read_bytes(size)?.to_vec();
            br.jmp(prev_pos);
            Ok(bytes)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::br_ext::{nul16_prefix_len, nul_prefix_len, BinaryReaderExtensions};
    use binary_reader::{BinaryReader, Endian};
    use encoding_rs::{SHIFT_JIS, UTF_16LE};

    /// The case that was actually wrong in the field. regulation.bin's BND4 header
    /// carries its version in 8 bytes with no room left for a terminator; the old rule
    /// reserved one anyway and dropped the last digit, turning 11611000 (game 1.16.1)
    /// into 1161100 — wrong, and wrong in a way that still looks like a version.
    #[test]
    fn full_width_field_keeps_its_last_byte() {
        let field = b"11611000";
        assert_eq!(nul_prefix_len(field), 8);
        let (decoded, _, had_errors) = SHIFT_JIS.decode(&field[..nul_prefix_len(field)]);
        assert!(!had_errors);
        assert_eq!(decoded, "11611000");
    }

    #[test]
    fn stops_at_the_first_nul_and_keeps_the_byte_before_it() {
        assert_eq!(nul_prefix_len(b"BND4\0\0\0\0"), 4);
        let field = b"BND4\0\0\0\0";
        let (decoded, _, _) = SHIFT_JIS.decode(&field[..nul_prefix_len(field)]);
        assert_eq!(decoded, "BND4");
    }

    #[test]
    fn a_field_opening_with_nul_is_empty() {
        assert_eq!(nul_prefix_len(b"\0abcdefg"), 0);
        assert_eq!(nul_prefix_len(b"\0\0\0\0\0\0\0\0"), 0);
    }

    #[test]
    fn single_byte_and_empty_fields_do_not_underflow() {
        assert_eq!(nul_prefix_len(b""), 0);
        assert_eq!(nul_prefix_len(b"x"), 1);
        assert_eq!(nul_prefix_len(b"\0"), 0);
    }

    /// Trailing content after the terminator is not part of the string, even when the
    /// field is reused and holds stale bytes.
    #[test]
    fn ignores_bytes_after_the_terminator() {
        assert_eq!(nul_prefix_len(b"ab\0stale"), 2);
    }

    /// The trap that made `read_fix_str` wrong for UTF-16: one zero byte is not a
    /// terminator, it is the high half of an ASCII character.
    #[test]
    fn a_lone_zero_byte_does_not_terminate_utf16() {
        // "AB" in UTF-16LE, then the terminator.
        assert_eq!(nul16_prefix_len(b"A\0B\0\0\0"), 4);
    }

    /// Same off-by-one as `full_width_field_keeps_its_last_byte`, one field wider: a
    /// fully-used UTF-16 field has no terminator, so the last code unit is still string.
    /// The old loop returned `size - 2` here and dropped it.
    #[test]
    fn full_width_utf16_field_keeps_its_last_code_unit() {
        assert_eq!(nul16_prefix_len(b"A\0B\0C\0D\0"), 8);
        let (decoded, _, had_errors) = UTF_16LE.decode(b"A\0B\0C\0D\0");
        assert!(!had_errors);
        assert_eq!(decoded, "ABCD");
    }

    #[test]
    fn utf16_empty_odd_and_leading_terminator_fields() {
        assert_eq!(nul16_prefix_len(b""), 0);
        assert_eq!(nul16_prefix_len(b"\0\0A\0"), 0);
        // An odd-length field cannot end mid-code-unit; the stray byte is not decodable.
        assert_eq!(nul16_prefix_len(b"A\0B"), 2);
        assert_eq!(nul16_prefix_len(b"A"), 0);
    }

    /// The real shape of a PARAM `param_type` field when the format stores it inline:
    /// single-byte, NUL-padded to 0x20. Every param in regulation 1.16.1 takes the
    /// `OffsetParamType` branch instead, so this path is unreached — but the strings it
    /// would read are the same single-byte ASCII the offset branch points at.
    #[test]
    fn a_fixed_param_type_field_decodes_as_bytes_not_utf16() {
        let mut field = [0u8; 0x20];
        field[..22].copy_from_slice(b"ACTION_BUTTON_PARAM_ST");
        let mut br = BinaryReader::from_u8(&field);
        br.set_endian(Endian::Little);

        let got = BinaryReaderExtensions::read_fix_str(&mut br, 0x20).expect("read");
        assert_eq!(got, "ACTION_BUTTON_PARAM_ST");
    }

    #[test]
    fn read_fix_str_w_keeps_the_last_character_of_a_full_field() {
        let mut field = [0u8; 8];
        for (i, c) in b"ABCD".iter().enumerate() {
            field[i * 2] = *c;
        }
        let mut br = BinaryReader::from_u8(&field);
        br.set_endian(Endian::Little);

        let got = BinaryReaderExtensions::read_fix_str_w(&mut br, 8).expect("read");
        assert_eq!(got, "ABCD");
    }
}
