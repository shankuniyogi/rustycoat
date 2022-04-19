macro_rules! set_lo_byte {
    ($obj:expr, $val:expr) => {
        *$obj = (*$obj & 0xFF00) | ($val as u16)
    };
}

macro_rules! set_hi_byte {
    ($obj:expr, $val:expr) => {
        *$obj = (*$obj & 0x00FF) | (($val as u16) << 8)
    };
}

macro_rules! lo_byte {
    ($obj:expr) => {
        ($obj & 0xFF) as u8
    };
}

macro_rules! hi_byte {
    ($obj:expr) => {
        ($obj >> 8) as u8
    };
}

macro_rules! bcd_add_digits {
    ($x:expr, $y:expr, $carry:expr) => {{
        let r = $x + $y + $carry;
        if r > 9 {
            r + 6
        } else {
            r
        }
    }};
}

#[macro_export]
macro_rules! assert_eq_hex {
    ($left:expr, $right:expr $(,)?) => {{
        assert_eq!(
            $left, $right,
            r#"
  left: `0x{:04x?}`,
 right: `0x{:04x?}`"#,
            $left, $right
        );
    }};
}
