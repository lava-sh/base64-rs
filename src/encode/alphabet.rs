pub const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub const URLSAFE_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Clone, Copy)]
pub enum Alphabet {
    Standard,
    UrlSafe,
}

impl Alphabet {
    pub const fn as_bytes(self) -> &'static [u8; 64] {
        match self {
            Self::Standard => ALPHABET,
            Self::UrlSafe => URLSAFE_ALPHABET,
        }
    }
}