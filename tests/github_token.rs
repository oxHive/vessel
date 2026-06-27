use vessel::generation::github::{decrypt_token, derive_encryption_key, encrypt_token};

#[test]
fn token_roundtrip() {
    let key = derive_encryption_key();
    let original = "ghp_testtoken123";
    let (enc, nonce) = encrypt_token(original, &key).unwrap();
    let decrypted = decrypt_token(&enc, &nonce, &key).unwrap();
    assert_eq!(decrypted, original);
}

#[test]
fn different_encryptions_of_same_token_differ() {
    let key = derive_encryption_key();
    let (enc1, _) = encrypt_token("token", &key).unwrap();
    let (enc2, _) = encrypt_token("token", &key).unwrap();
    assert_ne!(enc1, enc2); // different nonces produce different ciphertexts
}
