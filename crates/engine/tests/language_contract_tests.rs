//! Locks the engine's language table to the suite's wire `Language` (GB-143).
//!
//! The two enums are the same thirteen formats. This walks every variant in both directions and
//! asserts the spellings agree, so adding a format to one side without the other fails here rather
//! than at a consumer's deserialize.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::Language as WireLanguage;
use miter_engine::polyglot::PolyglotLanguage;

#[test]
fn test_every_language_round_trips_through_the_wire_enum() {
    assert_eq!(PolyglotLanguage::ALL.len(), 13);

    for language in PolyglotLanguage::ALL {
        let wire: WireLanguage = language.into();
        assert_eq!(
            wire.name(),
            language.name(),
            "wire and engine spellings disagree for {language:?}"
        );
        assert_eq!(
            WireLanguage::from_wire(language.name()),
            Some(wire),
            "wire parser rejects its own spelling for {language:?}"
        );

        let back: PolyglotLanguage = wire.into();
        assert_eq!(back, language, "round trip lost {language:?}");
    }
}

#[test]
fn test_wire_spellings_serialise_as_their_names() {
    for language in PolyglotLanguage::ALL {
        let json = serde_json::to_string(&WireLanguage::from(language)).expect("serialize");
        assert_eq!(json, format!("\"{}\"", language.name()));
    }
}
