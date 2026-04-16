#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Lang {
    #[cfg(feature = "de")]
    De,
    #[cfg(feature = "en")]
    En,
    #[cfg(feature = "tr")]
    Tr,
    #[cfg(feature = "ru")]
    Ru,
    #[cfg(feature = "fr")]
    Fr,
    #[cfg(feature = "es")]
    Es,
    #[cfg(feature = "it")]
    It,
    #[cfg(feature = "nl")]
    Nl,
    #[cfg(feature = "pt")]
    Pt,
    #[cfg(feature = "pl")]
    Pl,
    Unknown,
}

impl Lang {
    pub fn from_iso_639_1(code: &str) -> Option<Self> {
        match code {
            #[cfg(feature = "de")]
            "de" => Some(Lang::De),
            #[cfg(feature = "en")]
            "en" => Some(Lang::En),
            #[cfg(feature = "tr")]
            "tr" => Some(Lang::Tr),
            #[cfg(feature = "ru")]
            "ru" => Some(Lang::Ru),
            #[cfg(feature = "fr")]
            "fr" => Some(Lang::Fr),
            #[cfg(feature = "es")]
            "es" => Some(Lang::Es),
            #[cfg(feature = "it")]
            "it" => Some(Lang::It),
            #[cfg(feature = "nl")]
            "nl" => Some(Lang::Nl),
            #[cfg(feature = "pt")]
            "pt" => Some(Lang::Pt),
            #[cfg(feature = "pl")]
            "pl" => Some(Lang::Pl),
            "?" => Some(Lang::Unknown),
            _ => None,
        }
    }

    pub const fn iso_639_1(self) -> &'static str {
        match self {
            #[cfg(feature = "de")]
            Lang::De => "de",
            #[cfg(feature = "en")]
            Lang::En => "en",
            #[cfg(feature = "tr")]
            Lang::Tr => "tr",
            #[cfg(feature = "ru")]
            Lang::Ru => "ru",
            #[cfg(feature = "fr")]
            Lang::Fr => "fr",
            #[cfg(feature = "es")]
            Lang::Es => "es",
            #[cfg(feature = "it")]
            Lang::It => "it",
            #[cfg(feature = "nl")]
            Lang::Nl => "nl",
            #[cfg(feature = "pt")]
            Lang::Pt => "pt",
            #[cfg(feature = "pl")]
            Lang::Pl => "pl",
            Lang::Unknown => "?",
        }
    }

    pub const fn all_enabled() -> &'static [Lang] {
        &[
            #[cfg(feature = "de")]
            Lang::De,
            #[cfg(feature = "en")]
            Lang::En,
            #[cfg(feature = "tr")]
            Lang::Tr,
            #[cfg(feature = "ru")]
            Lang::Ru,
            #[cfg(feature = "fr")]
            Lang::Fr,
            #[cfg(feature = "es")]
            Lang::Es,
            #[cfg(feature = "it")]
            Lang::It,
            #[cfg(feature = "nl")]
            Lang::Nl,
            #[cfg(feature = "pt")]
            Lang::Pt,
            #[cfg(feature = "pl")]
            Lang::Pl,
        ]
    }
}
