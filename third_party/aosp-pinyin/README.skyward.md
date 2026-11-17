# AOSP Pinyin Decoder

Source: LineageOS/android_packages_inputmethods_PinyinIME
Revision: a08d08cdea4319fb74b55289e1b4e1b1a5249bc2
License: Apache-2.0; original NOTICE and per-file notices retained.
The decoder, headers, dictionary builder, original raw dictionary and notices
are vendored. The build regenerates the binary dictionary for 64-bit
little-endian targets; upstream's prebuilt ARM32 dictionary is not used.
Skyward adds a bounded C ABI adapter and disables Android-only debug logging
through an include shim. User dictionary data is temporary, never persisted.
