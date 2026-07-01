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

#[test]
fn bluesky_has_300_limit() {
    assert_eq!(Platform::Bluesky.spec().char_limit, Some(300));
}

#[test]
fn mastodon_has_500_limit() {
    assert_eq!(Platform::Mastodon.spec().char_limit, Some(500));
}

#[test]
fn discord_has_no_char_limit() {
    assert_eq!(Platform::Discord.spec().char_limit, None);
}

#[test]
fn linkedin_has_3000_limit() {
    assert_eq!(Platform::LinkedIn.spec().char_limit, Some(3000));
}

#[test]
fn every_v1_platform_has_a_slug() {
    for platform in Platform::all_v1() {
        assert!(!platform.slug().is_empty());
    }
}
