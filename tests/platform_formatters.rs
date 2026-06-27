use vessel::generation::platforms::Platform;

#[test]
fn twitter_has_280_limit() {
    let spec = Platform::Twitter.spec();
    assert_eq!(spec.char_limit, Some(280));
}

#[test]
fn twitter_validates_over_limit() {
    let long = "x".repeat(281);
    assert!(Platform::Twitter.validate_length(&long).is_some());
}

#[test]
fn twitter_validates_within_limit() {
    let short = "x".repeat(280);
    assert!(Platform::Twitter.validate_length(&short).is_none());
}

#[test]
fn all_v1_has_six_platforms() {
    assert_eq!(Platform::all_v1().len(), 6);
}

#[test]
fn github_release_has_no_char_limit() {
    assert_eq!(Platform::GitHubRelease.spec().char_limit, None);
}
