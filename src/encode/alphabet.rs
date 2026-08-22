#[derive(Clone, Copy)]
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
    #[must_use]
    pub const fn as_bytes(self) -> &'static AlphabetTable {
        match self {
            Self::Standard => &ALPHABET,
            Self::UrlSafe => &URLSAFE_ALPHABET,
        }
    }

    #[must_use]
    pub const fn table(self) -> AlphabetTable {
        *self.as_bytes()
    }

    #[must_use]
    pub const fn from_altchars(altchars: [u8; 2]) -> Option<Self> {
        match altchars {
            [b'-', b'_'] => Some(Self::UrlSafe),
            _ => None,
        }
    }

    #[must_use]
    pub const fn pairs(self) -> *const u16 {
        match self {
            Self::Standard => super::tables::STANDARD_PAIRS.as_ptr(),
            Self::UrlSafe => super::tables::URLSAFE_PAIRS.as_ptr(),
        }
    }
}
