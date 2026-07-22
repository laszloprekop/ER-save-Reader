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

        pub fn read_fix_str_w(br: &mut BinaryReader, size: usize) -> Result<String, Error> {
            let big_endian = br.endian == Endian::Big;
            let bytes = br.read_bytes(size)?;
            let mut terminator = 0;
            for mut i in (0..size).step_by(2) {
                terminator = i;
                if i == size -1 {
                    i -= 1;
                }
                else if bytes[i] == 0 && bytes[i + 1] == 0 {
                    break;
                }
            }

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

        pub fn read_fix_str(br: &mut BinaryReader, size: usize) -> Result<String, Error> {
            let big_endian = br.endian == Endian::Big;
            let bytes = br.read_bytes(size)?;
            let terminator = super::br_ext::nul_prefix_len(bytes);

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
    use super::br_ext::nul_prefix_len;
    use encoding_rs::SHIFT_JIS;

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
}
