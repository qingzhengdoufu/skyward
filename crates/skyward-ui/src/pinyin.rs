//! Offline AOSP decoder; input stays in memory and private temporary files.
#[cfg(feature = "pinyin-input")]
use std::{
    ffi::CString,
    sync::{Mutex, OnceLock},
};

#[cfg(feature = "pinyin-input")]
unsafe extern "C" {
    fn im_open_decoder(system: *const std::ffi::c_char, user: *const std::ffi::c_char) -> bool;
    fn im_set_max_lens(spelling: usize, characters: usize);
    fn skyward_pinyin_query(input: *const std::ffi::c_char, index: usize, output: *mut u16, capacity: usize, consumed: *mut usize)
    -> usize;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub consumed: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Composition {
    pub enabled: bool,
    pub spelling: String,
    pub page: usize,
    pub candidates: Vec<Candidate>,
}

impl Composition {
    pub fn update(&mut self) {
        self.candidates = candidates(&self.spelling);
        self.page = 0;
    }
    pub fn clear(&mut self) {
        self.spelling.clear();
        self.candidates.clear();
        self.page = 0;
    }
    pub fn choose(&mut self, index: usize) -> Option<String> {
        let candidate = self.candidates.get(index)?.clone();
        self.spelling.drain(..candidate.consumed.min(self.spelling.len()));
        self.spelling = self.spelling.trim_start_matches('\'').into();
        self.update();
        Some(candidate.text)
    }
}

#[cfg(feature = "pinyin-input")]
struct Engine {
    _files: tempfile::TempDir,
}
#[cfg(feature = "pinyin-input")]
impl Engine {
    fn open() -> Option<Self> {
        let files = tempfile::Builder::new().prefix("skyward-pinyin-").tempdir_in("/tmp").ok()?;
        let system = files.path().join("system.dat");
        let user = files.path().join("user.dat");
        std::fs::write(&system, include_bytes!(concat!(env!("OUT_DIR"), "/res/raw/dict_pinyin.dat"))).ok()?;
        let (system, user) = (CString::new(system.to_str()?).ok()?, CString::new(user.to_str()?).ok()?);
        // The C decoder's singleton is owned by this process-wide mutex.
        if !unsafe { im_open_decoder(system.as_ptr(), user.as_ptr()) } {
            return None;
        }
        unsafe {
            im_set_max_lens(32, 16);
        }
        Some(Self { _files: files })
    }
}

#[cfg(not(feature = "pinyin-input"))]
pub fn candidates(_: &str) -> Vec<Candidate> {
    Vec::new()
}

#[cfg(feature = "pinyin-input")]
pub fn candidates(spelling: &str) -> Vec<Candidate> {
    if spelling.is_empty() || spelling.len() > 32 || !spelling.bytes().all(|c| c.is_ascii_lowercase() || c == b'\'') {
        return Vec::new();
    }
    static ENGINE: OnceLock<Mutex<Option<Engine>>> = OnceLock::new();
    let Ok(engine) = ENGINE.get_or_init(|| Mutex::new(Engine::open())).lock() else {
        return Vec::new();
    };
    if engine.is_none() {
        return Vec::new();
    }
    let input = CString::new(spelling).expect("validated ASCII spelling");
    let mut result = Vec::new();
    for index in 0..24 {
        let mut text = [0u16; 64];
        let mut consumed = 0;
        let count = unsafe { skyward_pinyin_query(input.as_ptr(), index, text.as_mut_ptr(), text.len(), &mut consumed) };
        if index >= count {
            break;
        }
        let end = text.iter().position(|c| *c == 0).unwrap_or(text.len());
        if end > 0
            && consumed > 0
            && let Ok(text) = String::from_utf16(&text[..end])
        {
            result.push(Candidate { text, consumed });
        }
    }
    result
}

#[cfg(all(test, feature = "pinyin-input"))]
mod tests {
    use super::*;
    #[test]
    fn real_dictionary_decodes_chinese_and_keeps_a_bounded_candidate_list() {
        let result = candidates("nihao");
        assert!(result.iter().any(|c| c.text == "你好"), "{result:?}");
        assert!(result.len() <= 24);
        for invalid in ["", "../../data", "你", "ni hao", "0123"] {
            assert!(candidates(invalid).is_empty());
        }
        assert!(candidates(&"a".repeat(33)).is_empty());
    }
    #[test]
    fn choosing_a_phrase_consumes_its_spelling_without_committing_raw_letters() {
        let mut composition = Composition {
            enabled: true,
            spelling: "nihao".into(),
            ..Composition::default()
        };
        composition.update();
        let index = composition.candidates.iter().position(|c| c.text == "你好").unwrap();
        assert_eq!(composition.choose(index).as_deref(), Some("你好"));
        assert!(composition.spelling.is_empty());
    }
}
