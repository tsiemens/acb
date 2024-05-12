use regex::Regex;
use std::{fmt::Debug, iter::zip};

pub fn assert_re(pattern: &str, haystack: &str) {
	let re = Regex::new(pattern).unwrap();
    assert!(re.is_match(haystack),
            "{:?} did not match {:?}", haystack, re);
}

fn eprint_vecs<T: PartialEq + Debug>(left: &Vec<T>, right: &Vec<T>) {
    let mut err_str = "left != right. left: [\n".to_string();
    for o in left {
        err_str += &format!("{:?},\n", o).to_string();
    }
    err_str += "] != right: [\n";
    for o in right {
        err_str += &format!("{:?},\n", o).to_string();
    }
    eprintln!("{}", err_str);
}

pub fn assert_big_struct_eq<T: PartialEq + Debug>(left: T, right: T) {
    assert_eq!(left, right, "{:#?} != {:#?}", left, right);
}

pub fn assert_vec_eq<T: PartialEq + Debug>(left: Vec<T>, right: Vec<T>) {
    assert_vecr_eq(&left, &right);
}

pub fn assert_vecr_eq<T: PartialEq + Debug>(left: &Vec<T>, right: &Vec<T>) {
    if left == right {
        return
    }
    eprint_vecs(&left, &right);

    if left.len() != right.len() {
        eprintln!("size of left ({}) != size of right ({})", left.len(), right.len());
        panic!();
    }
    let mut i = 0;
    for (l, r) in zip(left, right) {
        if l != r {
            eprintln!("Mismatch at index {}:", i);
            eprintln!("left: {:#?} != right: {:#?}", l, r);
        }

        i += 1;
    }
    panic!();
}