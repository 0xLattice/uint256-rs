use lattice_uint256::{Error, UInt256 as U};
fn hex(s: &str) -> U {
    U::from_str_radix(s, 16).unwrap()
}
#[test]
fn actual_swift_arithmetic_and_shift_vectors() {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("swift-vectors.json")).unwrap();
    assert_eq!(rows.len(), 188);
    for row in rows {
        let a = hex(row["a"].as_str().unwrap());
        let b = hex(row["b"].as_str().unwrap());
        for (name, (value, overflow)) in [
            ("add", a.overflowing_add(b)),
            ("sub", a.overflowing_sub(b)),
            ("mul", a.overflowing_mul(b)),
            ("div", a.overflowing_div(b)),
            ("rem", a.overflowing_rem(b)),
        ] {
            assert_eq!(value.to_hex_string(), row[name], "{name} {a:?} {b:?}");
            assert_eq!(overflow, row[format!("{name}Overflow")].as_bool().unwrap());
        }
        assert_eq!((a / b).to_hex_string(), row["plainDiv"]);
        assert_eq!((a % b).to_hex_string(), row["plainRem"]);
        assert_eq!(a.div_rem(b), (a / b, a % b));
        let (high, low) = a.full_mul(b);
        for (name, value) in [
            ("high", high),
            ("low", low),
            ("and", a & b),
            ("or", a | b),
            ("xor", a ^ b),
            ("swapped", a.swap_bytes()),
        ] {
            assert_eq!(value.to_hex_string(), row[name], "{name}");
        }
        assert_eq!(a.count_ones() as u64, row["ones"].as_u64().unwrap());
        assert_eq!(a.leading_zeros() as u64, row["leading"].as_u64().unwrap());
        assert_eq!(a.trailing_zeros() as u64, row["trailing"].as_u64().unwrap());
        for shift in row["shifts"].as_array().unwrap() {
            let n = shift["n"].as_i64().unwrap();
            assert_eq!(a.shift_left(n).to_hex_string(), shift["left"]);
            assert_eq!(a.shift_right(n).to_hex_string(), shift["right"]);
        }
        if let Some(high) = row["divHigh"].as_str() {
            let (q, r) = a.full_div(hex(high), b).unwrap();
            assert_eq!(q.to_hex_string(), row["fullQuotient"]);
            assert_eq!(r.to_hex_string(), row["fullRemainder"]);
        }
    }
}
#[test]
fn bounds_and_conversion_contracts() {
    assert_eq!(U::MAX.checked_add(1.into()), None);
    assert_eq!(U::ZERO.checked_sub(1.into()), None);
    assert_eq!(U::MAX + U::from(1), U::ZERO);
    assert_eq!(U::ZERO - U::from(1), U::MAX);
    assert_eq!(U::from_i64_wrapping(-1), U::MAX);
    assert_eq!(
        U::from_swift_words(&[1, 2, 3, 4, 5]).words_be(),
        [1, 2, 3, 4]
    );
    assert_eq!(U::from_swift_words(&[1, 2]).words_be(), [0, 0, 1, 2]);
    assert_eq!(
        U::from(2).full_div(U::from(2), U::ZERO),
        Err(Error::Overflow)
    );
    assert_eq!(
        U::ZERO.full_div(U::ZERO, U::ZERO),
        Err(Error::DivisionByZero)
    );
    assert_eq!(U::from_str_radix(&"f".repeat(65), 16), Err(Error::Overflow));
    for radix in [0, 1, 37, u32::MAX] {
        assert_eq!(U::from_str_radix("1", radix), Err(Error::InvalidInteger));
    }
    let n = hex("010203040506070809");
    assert_eq!(U::from_le_bytes(n.to_le_bytes()), n);
    assert_eq!(n.swap_bytes().swap_bytes(), n);
    assert_eq!(n.to_string(), n.to_hex_string());
}
