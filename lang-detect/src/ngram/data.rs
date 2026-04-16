include!(concat!(env!("OUT_DIR"), "/ngram_data.rs"));

use crate::lang::Lang;

pub(crate) fn logprob(lang: Lang, trigram: &str) -> f32 {
    match lang {
        #[cfg(feature = "de")]
        Lang::De => DE_TRIGRAMS.get(trigram).copied().unwrap_or(DE_FLOOR),
        #[cfg(feature = "en")]
        Lang::En => EN_TRIGRAMS.get(trigram).copied().unwrap_or(EN_FLOOR),
        #[cfg(feature = "tr")]
        Lang::Tr => TR_TRIGRAMS.get(trigram).copied().unwrap_or(TR_FLOOR),
        #[cfg(feature = "ru")]
        Lang::Ru => RU_TRIGRAMS.get(trigram).copied().unwrap_or(RU_FLOOR),
        #[cfg(feature = "fr")]
        Lang::Fr => FR_TRIGRAMS.get(trigram).copied().unwrap_or(FR_FLOOR),
        #[cfg(feature = "es")]
        Lang::Es => ES_TRIGRAMS.get(trigram).copied().unwrap_or(ES_FLOOR),
        #[cfg(feature = "it")]
        Lang::It => IT_TRIGRAMS.get(trigram).copied().unwrap_or(IT_FLOOR),
        #[cfg(feature = "nl")]
        Lang::Nl => NL_TRIGRAMS.get(trigram).copied().unwrap_or(NL_FLOOR),
        #[cfg(feature = "pt")]
        Lang::Pt => PT_TRIGRAMS.get(trigram).copied().unwrap_or(PT_FLOOR),
        #[cfg(feature = "pl")]
        Lang::Pl => PL_TRIGRAMS.get(trigram).copied().unwrap_or(PL_FLOOR),
        _ => 0.0,
    }
}
