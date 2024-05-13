// Override the version in Cargo.toml, since we are not using standard
// rust versioning scheme, and we're not really trying to model this
// as a crate, in terms of compatability.
//
// Version is of the format 0.YY.MM[.i], or 0.year.month.optional_minor_increment
// This is similar to Ubuntu's versioning scheme, and allows for a more immediate
// reference for when the last time the app was updated.
// Major version is kept at 0, since the app is perpetually in 'beta' due to there
// not being a tax-lawer on staff to verify anything.
pub const ACB_APP_VERSION: &str = "0.24.??.rust";