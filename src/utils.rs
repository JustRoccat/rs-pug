pub fn natural_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();
    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (Some(&a_char), Some(&b_char)) => {
                if a_char.is_ascii_digit() && b_char.is_ascii_digit() {
                    let mut a_num = 0u64;
                    while let Some(&c) = a_chars.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        a_num = a_num
                            .saturating_mul(10)
                            .saturating_add(c.to_digit(10).unwrap_or(0) as u64);
                        a_chars.next();
                    }
                    let mut b_num = 0u64;
                    while let Some(&c) = b_chars.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        b_num = b_num
                            .saturating_mul(10)
                            .saturating_add(c.to_digit(10).unwrap_or(0) as u64);
                        b_chars.next();
                    }
                    if a_num != b_num {
                        return a_num.cmp(&b_num);
                    }
                } else {
                    if a_char != b_char {
                        return a_char.cmp(&b_char);
                    }
                    a_chars.next();
                    b_chars.next();
                }
            }
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, None) => return std::cmp::Ordering::Equal,
        }
    }
}
