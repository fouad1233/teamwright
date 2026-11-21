//! Hiding what looks like a secret in a text that is about to be kept.
//!
//! A digest of an earlier conversation is made of what the manager typed
//! there, and the manager may once have pasted a token or written a
//! password into a command. A digest travels to the other computers through
//! the sync folder ([decision 0041](../../../docs/decisions/0041-earlier-conversations-as-digests.md)),
//! so what looks like a secret is replaced by `[hidden]` before the manager
//! is even shown it.
//!
//! These rules guess. They hide too much rather than too little, and they
//! cannot promise to find everything; that is why the manager reads the
//! digest before it is kept.

/// What a hidden secret becomes.
pub const HIDDEN: &str = "[hidden]";

/// The beginnings of tokens that services hand out, which are secret
/// whatever follows them.
const KNOWN_PREFIXES: &[&str] = &[
    "sk-",
    "sk_",
    "rk_",
    "pk_live_",
    "ghp_",
    "gho_",
    "ghs_",
    "ghu_",
    "ghr_",
    "github_pat_",
    "glpat-",
    "xoxa-",
    "xoxb-",
    "xoxp-",
    "xoxs-",
    "xapp-",
    "AKIA",
    "ASIA",
    "AIza",
    "ya29.",
    "hf_",
    "npm_",
    "pypi-",
    "shpat_",
];

/// How many characters must follow a known beginning before it counts as a
/// token. "sk-learn" is a library, not a key.
const LEAST_AFTER_PREFIX: usize = 12;

/// How long a run of letters, digits and the signs of encoded data must be
/// to be taken for a key when it mixes capitals, small letters and digits.
const LEAST_RANDOM_RUN: usize = 32;

/// Words that, followed by `=` or `:`, say that a secret comes next.
const SECRET_NAMES: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "pass",
    "secret",
    "token",
    "apikey",
    "api_key",
    "api-key",
    "access_key",
    "accesskey",
    "private_key",
    "client_secret",
    "auth",
    "authorization",
    "bearer",
    "credentials",
];

/// Replaces what looks like a secret by [`HIDDEN`]: tokens with a known
/// beginning, long runs that look random, the value after a word such as
/// `password=` or `token:`, the word after `Bearer`, the name and password
/// in an address such as `https://name:secret@host`, and private keys. The
/// rest of the text, its spaces and its line breaks included, is kept as it
/// was.
#[must_use]
pub fn hide_secrets(text: &str) -> String {
    let text = hide_private_keys(text);
    let mut hidden = String::with_capacity(text.len());
    let mut hides_next_word = false;
    let mut word = String::new();
    for character in text.chars() {
        if character.is_whitespace() {
            if !word.is_empty() {
                hides_next_word = push_word(&mut hidden, &word, hides_next_word);
                word.clear();
            }
            hidden.push(character);
        } else {
            word.push(character);
        }
    }
    if !word.is_empty() {
        push_word(&mut hidden, &word, hides_next_word);
    }
    hidden
}

/// Adds one word to the text, hidden if it must be, and says whether the
/// word after it must be hidden too.
fn push_word(hidden: &mut String, word: &str, is_a_secret: bool) -> bool {
    if is_a_secret {
        // "Authorization: Bearer abc…": the kind of the secret is shown,
        // the secret is the word after it.
        if is_a_kind_of_credential(word) {
            hidden.push_str(word);
            return true;
        }
        hidden.push_str(HIDDEN);
        return false;
    }
    let (before, core, after) = strip_marks(word);
    if looks_like_a_token(core) {
        hidden.push_str(before);
        hidden.push_str(HIDDEN);
        hidden.push_str(after);
        return false;
    }
    if let Some((name, separator_end)) = secret_assignment(core) {
        hidden.push_str(before);
        hidden.push_str(&core[..separator_end]);
        if separator_end < core.len() {
            hidden.push_str(HIDDEN);
            hidden.push_str(after);
            return false;
        }
        hidden.push_str(after);
        // "password:" with the value in the next word.
        return !name.is_empty();
    }
    if let Some(shown) = address_without_credentials(core) {
        hidden.push_str(before);
        hidden.push_str(&shown);
        hidden.push_str(after);
        return false;
    }
    hidden.push_str(word);
    // "Bearer abc…": the token is the next word.
    is_a_kind_of_credential(core)
}

/// Whether a word names the kind of a credential that follows it.
fn is_a_kind_of_credential(word: &str) -> bool {
    word.eq_ignore_ascii_case("bearer") || word.eq_ignore_ascii_case("basic")
}

/// Splits a word into the quotation marks and punctuation around it and
/// what they surround. A full stop at the end of a sentence is not part of
/// a token either.
fn strip_marks(word: &str) -> (&str, &str, &str) {
    let is_mark = |character: char| "\"'`()[]{}<>,;.!?".contains(character);
    let start = word.find(|character| !is_mark(character)).unwrap_or(0);
    let end = word
        .rfind(|character| !is_mark(character))
        .map_or(word.len(), |last| {
            last + word[last..].chars().next().map_or(0, char::len_utf8)
        });
    if start >= end {
        return ("", word, "");
    }
    (&word[..start], &word[start..end], &word[end..])
}

/// Whether a word is a token: one with a known beginning, or a long run
/// that looks random.
fn looks_like_a_token(core: &str) -> bool {
    let has_known_prefix = KNOWN_PREFIXES.iter().any(|prefix| {
        core.starts_with(prefix)
            && core[prefix.len()..].chars().count() >= LEAST_AFTER_PREFIX
            && core[prefix.len()..]
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    });
    has_known_prefix || looks_random(core)
}

/// Whether a word is a long run of the characters of encoded data that
/// mixes capitals, small letters and digits. A path, a word and a
/// fingerprint in small letters and digits are not.
fn looks_random(core: &str) -> bool {
    core.chars().count() >= LEAST_RANDOM_RUN
        && core
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "+/=_-".contains(character))
        && core.chars().any(|character| character.is_ascii_digit())
        && core.chars().any(|character| character.is_ascii_uppercase())
        && core.chars().any(|character| character.is_ascii_lowercase())
}

/// Finds `password=…`, `TOKEN: …` and their kind in a word: the name and
/// where the separator ends. The name may stand after other words joined to
/// it, as in `DB_PASSWORD=…` or `--api-key=…`.
fn secret_assignment(core: &str) -> Option<(&str, usize)> {
    let separator = core.find(['=', ':'])?;
    let name = &core[..separator];
    let lowered = name.to_ascii_lowercase();
    let lowered = lowered.trim_start_matches('-');
    let names_a_secret = SECRET_NAMES.iter().any(|secret| {
        lowered == *secret
            || lowered.ends_with(&format!("_{secret}"))
            || lowered.ends_with(&format!("-{secret}"))
            || lowered.ends_with(&format!(".{secret}"))
    });
    if !names_a_secret {
        return None;
    }
    let mut separator_end = separator + 1;
    // "token:=", "password==" and the like.
    while core[separator_end..].starts_with(['=', ':']) {
        separator_end += 1;
    }
    // "http://" is an address, not a value, and never follows these names.
    Some((name, separator_end))
}

/// An address with a name and a password in it, `scheme://name:secret@host…`,
/// with the two hidden. Any other word gives none.
fn address_without_credentials(core: &str) -> Option<String> {
    let (scheme, rest) = core.split_once("://")?;
    let (credentials, host) = rest.split_once('@')?;
    if credentials.contains('/') || !credentials.contains(':') {
        return None;
    }
    Some(format!("{scheme}://{HIDDEN}@{host}"))
}

/// Replaces every private key, from its `-----BEGIN … PRIVATE KEY-----`
/// line to its end line, by [`HIDDEN`]. A key whose end is missing is
/// hidden to the end of the text.
fn hide_private_keys(text: &str) -> String {
    const BEGIN: &str = "-----BEGIN";
    const END: &str = "-----END";
    let mut rest = text;
    let mut kept = String::with_capacity(text.len());
    while let Some(start) = rest.find(BEGIN) {
        let line_end = rest[start..]
            .find('\n')
            .map_or(rest.len(), |end| start + end);
        if !rest[start..line_end].contains("PRIVATE KEY") {
            kept.push_str(&rest[..line_end]);
            rest = &rest[line_end..];
            continue;
        }
        kept.push_str(&rest[..start]);
        kept.push_str(HIDDEN);
        match rest[line_end..].find(END) {
            Some(end) => {
                let after_end = line_end + end + END.len();
                let close = rest[after_end..]
                    .find("-----")
                    .map_or(rest.len(), |close| after_end + close + "-----".len());
                rest = &rest[close..];
            }
            None => rest = "",
        }
    }
    kept.push_str(rest);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_words_paths_and_fingerprints_are_kept() {
        let text = "Please fix src/features/advisor/AdvisorView.tsx and commit 1e8ccc5a9f0b2c4d6e8f0a1b2c3d4e5f6a7b8c9d.";

        assert_eq!(hide_secrets(text), text);
    }

    #[test]
    fn spaces_and_line_breaks_are_kept() {
        let text = "one  two\n\nthree\tfour";

        assert_eq!(hide_secrets(text), text);
    }

    #[test]
    fn a_token_with_a_known_beginning_is_hidden() {
        assert_eq!(
            hide_secrets("use ghp_abcdefghijklmnop1234 for it"),
            "use [hidden] for it"
        );
        assert_eq!(
            hide_secrets("the key is \"sk-proj-aaaabbbbccccdddd\"."),
            "the key is \"[hidden]\"."
        );
    }

    #[test]
    fn a_short_word_with_a_known_beginning_is_kept() {
        assert_eq!(hide_secrets("install sk-learn"), "install sk-learn");
    }

    #[test]
    fn a_long_run_that_looks_random_is_hidden() {
        assert_eq!(
            hide_secrets("value Zm9vYmFyQmF6UXV4MTIzNDU2Nzg5MGFiY2RlZg== here"),
            "value [hidden] here"
        );
    }

    #[test]
    fn the_value_after_the_name_of_a_secret_is_hidden() {
        assert_eq!(
            hide_secrets("run with DB_PASSWORD=hunter2 please"),
            "run with DB_PASSWORD=[hidden] please"
        );
        assert_eq!(
            hide_secrets("--api-key=abc123 --verbose"),
            "--api-key=[hidden] --verbose"
        );
        assert_eq!(hide_secrets("password: hunter2"), "password: [hidden]");
        assert_eq!(hide_secrets("Token: abc"), "Token: [hidden]");
    }

    #[test]
    fn a_name_that_only_ends_like_a_secret_is_kept() {
        assert_eq!(hide_secrets("compass=north"), "compass=north");
        assert_eq!(hide_secrets("keyword: value"), "keyword: value");
    }

    #[test]
    fn the_word_after_bearer_is_hidden() {
        assert_eq!(
            hide_secrets("Authorization: Bearer abc.def.ghi"),
            "Authorization: Bearer [hidden]"
        );
        assert_eq!(
            hide_secrets("header Bearer abc.def"),
            "header Bearer [hidden]"
        );
    }

    #[test]
    fn the_name_and_password_in_an_address_are_hidden() {
        assert_eq!(
            hide_secrets("clone https://fouad:s3cret@example.com/repo.git now"),
            "clone https://[hidden]@example.com/repo.git now"
        );
        assert_eq!(
            hide_secrets("mail me at someone@example.com"),
            "mail me at someone@example.com"
        );
    }

    #[test]
    fn a_private_key_is_hidden_from_its_beginning_to_its_end() {
        let text = "key:\n-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\nBBBB\n-----END OPENSSH PRIVATE KEY-----\ndone";

        assert_eq!(hide_secrets(text), "key:\n[hidden]\ndone");
    }

    #[test]
    fn a_private_key_without_its_end_is_hidden_to_the_end() {
        let text = "-----BEGIN RSA PRIVATE KEY-----\nAAAA";

        assert_eq!(hide_secrets(text), "[hidden]");
    }

    #[test]
    fn a_public_certificate_is_kept() {
        let text = "-----BEGIN CERTIFICATE-----\nabc\n-----END CERTIFICATE-----";

        assert_eq!(hide_secrets(text), text);
    }

    #[test]
    fn letters_of_other_scripts_are_kept() {
        let text = "Ölçüm cihazı için şema — größer";

        assert_eq!(hide_secrets(text), text);
    }
}
