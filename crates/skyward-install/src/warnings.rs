//! Advisory output classification; never changes script execution or exit status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warning {
    MissingCommand,
    MissingFile,
    NoSpace,
    Permission,
    Io,
    Unpack,
}

pub fn classify(line: &str) -> Option<Warning> {
    let line = line.trim().to_ascii_lowercase();
    if line.contains(": inaccessible or not found") || line.contains(": command not found") {
        Some(Warning::MissingCommand)
    } else if line.contains(": no such file or directory") {
        Some(Warning::MissingFile)
    } else if line.contains(": no space left on device") {
        Some(Warning::NoSpace)
    } else if line.contains(": permission denied") || line.contains(": read-only file system") {
        Some(Warning::Permission)
    } else if line.contains(": input/output error") || line.contains(": i/o error") {
        Some(Warning::Io)
    } else if line.starts_with("zstd:") && (line.contains("error") || line.contains("unsupported") || line.contains("incomplete"))
        || line.starts_with("error: crc failed")
        || line.starts_with("error: data error")
    {
        Some(Warning::Unpack)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_diagnostics_are_advisory_categories_not_shell_decisions() {
        assert_eq!(
            classify("/skyward/run/update-binary[46]: checkDevice: inaccessible or not found"),
            Some(Warning::MissingCommand)
        );
        assert_eq!(classify("dd: frp: write error: No space left on device"), Some(Warning::NoSpace));
        assert_eq!(classify("unzip: x: No such file or directory"), Some(Warning::MissingFile));
        assert_eq!(classify("dd: super: Permission denied"), Some(Warning::Permission));
        assert_eq!(classify("dd: super: Input/output error"), Some(Warning::Io));
        assert_eq!(classify("zstd: stdin: incomplete frame"), Some(Warning::Unpack));
        for line in [
            "checkDevice",
            "0 errors",
            "Checking free space",
            "No errors found",
            "missing optional feature",
            "Everything is Ok",
            "if command -v tool; then tool; fi",
        ] {
            assert_eq!(classify(line), None, "{line}");
        }
    }
}
