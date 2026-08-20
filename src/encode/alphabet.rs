#[repr(transparent)]
pub struct AlphabetTable(pub [u8; 64]);

pub const ALPHABET: AlphabetTable =
    AlphabetTable(*b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/");

pub const URLSAFE_ALPHABET: AlphabetTable =
    AlphabetTable(*b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_");

#[derive(Clone, Copy)]
pub enum Alphabet {
    Standard,
    UrlSafe,
}

impl Alphabet {
    pub const fn as_bytes(self) -> &'static AlphabetTable {
        match self {
            Self::Standard => &ALPHABET,
            Self::UrlSafe => &URLSAFE_ALPHABET,
        }
    }
}
