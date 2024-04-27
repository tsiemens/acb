use regex::Regex;

pub fn assert_re(pattern: &str, haystack: &str) {
	let re = Regex::new(pattern).unwrap();
    assert!(re.is_match(haystack),
            "{:?} did not match {:?}", haystack, re);
}