pub trait ParseNum {
    /// Parse string to number
    ///
    /// This trait is default implemented for all [str](std::str) and
    /// [String](std::string::String)
    ///
    /// Unlike [from_str_radix](num::Num::from_str_radix) where user must manually
    /// determine the radix, this method support auto hex, dec, oct, bin detection
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::ParseNum;
    ///
    /// assert_eq!("0".parse_num::<i32>().unwrap(), 0i32);
    /// assert_eq!("10".parse_num::<f32>().unwrap(), 10f32);
    ///
    /// assert_eq!("0x01".parse_num::<u16>().unwrap(), 1u16);
    /// assert_eq!("0xFF".parse_num::<f64>().unwrap(), 255f64);
    /// assert_eq!("0b1111".parse_num::<u8>().unwrap(), 0b1111u8);
    /// assert_eq!("0o1463".parse_num::<u16>().unwrap(), 0o1463u16);
    ///
    /// assert_eq!("0XfF".parse_num::<f64>().unwrap(), 255f64);
    /// assert_eq!("0B1111".parse_num::<u8>().unwrap(), 0b1111u8);
    /// assert_eq!("0O1463".parse_num::<u16>().unwrap(), 0o1463u16);
    /// ```
    fn parse_num<T: num::Num>(&self) -> Result<T, T::FromStrRadixErr>;
}

impl ParseNum for str {
    fn parse_num<T: num::Num>(&self) -> Result<T, T::FromStrRadixErr> {
        let (radix, trimmed_str) =
            if self.starts_with("0x") || self.starts_with("0X") {
                (16, &self[2..])
            } else if self.starts_with("0b") || self.starts_with("0B") {
                (2, &self[2..])
            } else if self.starts_with("0o") || self.starts_with("0O") {
                (8, &self[2..])
            } else {
                (10, self)
            };

        T::from_str_radix(trimmed_str, radix)
    }
}

#[cfg(test)]
mod test {
    use super::ParseNum;

    #[test]
    fn conversion() {
        assert_eq!("0".parse_num::<i32>().unwrap(), 0i32);
        assert_eq!("10".parse_num::<f32>().unwrap(), 10f32);

        assert_eq!("0x01".parse_num::<u16>().unwrap(), 1u16);
        assert_eq!("0xFF".parse_num::<f64>().unwrap(), 255f64);
        assert_eq!("0b1111".parse_num::<u8>().unwrap(), 0b1111u8);
        assert_eq!("0o1463".parse_num::<u16>().unwrap(), 0o1463u16);

        assert_eq!("0XfF".parse_num::<f64>().unwrap(), 255f64);
        assert_eq!("0B1111".parse_num::<u8>().unwrap(), 0b1111u8);
        assert_eq!("0O1463".parse_num::<u16>().unwrap(), 0o1463u16);
    }

    #[test]
    fn fails() {
        assert!("".parse_num::<i8>().is_err());
        assert!("0b".parse_num::<u8>().is_err());
        assert!("0o".parse_num::<i16>().is_err());
        assert!("0x".parse_num::<u16>().is_err());

        assert!("a".parse_num::<i8>().is_err());
        assert!("0b2".parse_num::<u8>().is_err());
        assert!("0o8".parse_num::<i16>().is_err());
        assert!("0xg".parse_num::<u16>().is_err());
    }
}
