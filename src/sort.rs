use std::cmp::Ordering;

/// Natural order without allocation or integer parsing. ASCII case folds for
/// familiar source names; other UTF-8 and invalid bytes retain byte order.
pub fn name(a: &[u8], b: &[u8]) -> Ordering {
    let (mut x, mut y) = (0, 0);
    while x < a.len() && y < b.len() {
        if a[x].is_ascii_digit() && b[y].is_ascii_digit() {
            let (start_x, start_y) = (x, y);
            while x < a.len() && a[x].is_ascii_digit() {
                x += 1;
            }
            while y < b.len() && b[y].is_ascii_digit() {
                y += 1;
            }
            let mut ax = start_x;
            let mut by = start_y;
            while ax < x && a[ax] == b'0' {
                ax += 1;
            }
            while by < y && b[by] == b'0' {
                by += 1;
            }
            let order = (x - ax)
                .cmp(&(y - by))
                .then_with(|| a[ax..x].cmp(&b[by..y]));
            if !order.is_eq() {
                return order;
            }
        } else {
            let order = a[x].to_ascii_lowercase().cmp(&b[y].to_ascii_lowercase());
            if !order.is_eq() {
                return order;
            }
            x += 1;
            y += 1;
        }
    }
    (a.len() - x).cmp(&(b.len() - y)).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbers_case_and_arbitrarily_long_runs() {
        let mut names = [
            "photo10",
            "photo2",
            "photo1",
            "Photo3",
            "photo02",
            "photo0002",
        ];
        names.sort_by(|a, b| name(a.as_bytes(), b.as_bytes()));
        assert_eq!(
            names,
            [
                "photo1",
                "photo0002",
                "photo02",
                "photo2",
                "Photo3",
                "photo10"
            ]
        );
        assert!(
            name(
                b"a99999999999999999999999999999",
                b"a100000000000000000000000000000"
            )
            .is_lt()
        );
        assert!(name(b"a\xff", b"b").is_lt());
    }
    #[test]
    fn comparison_is_a_total_order_with_zero_runs_and_suffixes() {
        let names: &[&[u8]] = &[
            b"", b"0", b"00", b"a0", b"a00", b"A0", b"a00x", b"a0y", b"a1", b"a01", b"a2", b"a11",
            b"\xff",
        ];
        for a in names {
            for b in names {
                for c in names {
                    assert_eq!(name(a, b), name(b, a).reverse());
                    if name(a, b).is_le() && name(b, c).is_le() {
                        assert!(name(a, c).is_le());
                    }
                }
            }
        }
    }
}
