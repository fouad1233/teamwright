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

/// How many hexadecimal digits in a row make a key. A short commit hash of
/// 7 to 12 digits stays; a full hash of 40 and any longer run is hidden,
/// since a key and a full hash look the same.
const LEAST_HEX_RUN: usize = 32;

/// What the word after the one being read must be, as far as the words
/// before it say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    /// Nothing special.
    Ordinary,
    /// A secret: it comes after `Bearer` or a name such as `password:`.
    Secret,
    /// The value of a flag such as `--password` or `-p`. It is hidden
    /// unless it is another flag, or, when `unless_a_path`, a path: `-p`
    /// is also what `mkdir -p src/lib` says.
    ValueOfFlag { unless_a_path: bool },
}

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
    let mut next = Next::Ordinary;
    let mut word = String::new();
    for character in text.chars() {
        if character.is_whitespace() {
            if !word.is_empty() {
                next = push_word(&mut hidden, &word, next);
                word.clear();
            }
            hidden.push(character);
        } else {
            word.push(character);
        }
    }
    if !word.is_empty() {
        push_word(&mut hidden, &word, next);
    }
    hidden
}

/// Adds one word to the text, hidden if it must be, and says what the word
/// after it must be.
fn push_word(hidden: &mut String, word: &str, expected: Next) -> Next {
    match expected {
        Next::Ordinary => {}
        Next::Secret => {
            // "Authorization: Bearer abc…": the kind of the secret is shown,
            // the secret is the word after it.
            if is_a_kind_of_credential(word) {
                hidden.push_str(word);
                return Next::Secret;
            }
            push_hidden(hidden, word);
            return Next::Ordinary;
        }
        Next::ValueOfFlag { unless_a_path } => {
            let is_another_flag = word.starts_with('-');
            if !is_another_flag && !(unless_a_path && looks_like_a_path(word)) {
                push_hidden(hidden, word);
                return Next::Ordinary;
            }
        }
    }
    let (before, core, after) = strip_marks(word);
    if looks_like_a_token(core) {
        hidden.push_str(before);
        hidden.push_str(HIDDEN);
        hidden.push_str(after);
        return Next::Ordinary;
    }
    if let Some((name, separator_end)) = secret_assignment(core) {
        hidden.push_str(before);
        hidden.push_str(&core[..separator_end]);
        if separator_end < core.len() {
            hidden.push_str(HIDDEN);
            hidden.push_str(after);
            return Next::Ordinary;
        }
        hidden.push_str(after);
        // "password:" with the value in the next word.
        return if name.is_empty() {
            Next::Ordinary
        } else {
            Next::Secret
        };
    }
    if let Some(shown) = address_without_credentials(core) {
        hidden.push_str(before);
        hidden.push_str(&shown);
        hidden.push_str(after);
        return Next::Ordinary;
    }
    hidden.push_str(word);
    // "Bearer abc…": the token is the next word.
    if is_a_kind_of_credential(core) {
        return Next::Secret;
    }
    // "--password abc", "-p abc".
    match core {
        "-p" => Next::ValueOfFlag {
            unless_a_path: true,
        },
        _ if names_a_secret(core.trim_start_matches('-')) && core.starts_with("--") => {
            Next::ValueOfFlag {
                unless_a_path: false,
            }
        }
        _ => Next::Ordinary,
    }
}

/// Adds a word as [`HIDDEN`], keeping the quotation marks and punctuation
/// around it.
fn push_hidden(hidden: &mut String, word: &str) {
    let (before, _, after) = strip_marks(word);
    hidden.push_str(before);
    hidden.push_str(HIDDEN);
    hidden.push_str(after);
}

/// Whether a word is a path of a file or folder: it holds a separator or
/// starts like a relative or home path.
fn looks_like_a_path(word: &str) -> bool {
    word.contains(['/', '\\']) || word.starts_with(['.', '~'])
}

/// Whether a lower-case or mixed-case name, without leading hyphens, names
/// a secret: one of [`SECRET_NAMES`], alone or at the end of a longer name.
fn names_a_secret(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    SECRET_NAMES.iter().any(|secret| {
        lowered == *secret
            || lowered.ends_with(&format!("_{secret}"))
            || lowered.ends_with(&format!("-{secret}"))
            || lowered.ends_with(&format!(".{secret}"))
    })
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
    has_known_prefix
        || looks_random(core)
        || looks_like_a_web_token(core)
        || looks_like_a_hex_key(core)
}

/// Whether a word is a JSON Web Token: three runs of letters, digits,
/// hyphens and underscores joined by dots, the first starting `eyJ` (an
/// encoded `{"`). The last run, the signature, may be empty.
fn looks_like_a_web_token(core: &str) -> bool {
    let mut runs = core.split('.');
    // A full stop after the token ends a sentence and is not read as part
    // of it, so an unsigned token may arrive as two runs.
    let (Some(header), Some(claims), signature, None) =
        (runs.next(), runs.next(), runs.next(), runs.next())
    else {
        return false;
    };
    if signature.is_none() && !claims.starts_with("eyJ") {
        return false;
    }
    let signature = signature.unwrap_or_default();
    let is_base64url = |run: &str| {
        run.chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
    };
    header.starts_with("eyJ")
        && !claims.is_empty()
        && is_base64url(header)
        && is_base64url(claims)
        && is_base64url(signature)
}

/// Whether a word is a long run of hexadecimal digits, as a key or a
/// digest is written. A commit hash of 7 to 12 digits is shorter.
fn looks_like_a_hex_key(core: &str) -> bool {
    core.len() >= LEAST_HEX_RUN && core.chars().all(|character| character.is_ascii_hexdigit())
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
    // `"password": "x"` in JSON, and `'token'=…`: the quotes are not the name.
    let bare = name
        .trim_end_matches(['"', '\'', '`'])
        .trim_start_matches(['"', '\'', '`']);
    let bare = bare.trim_start_matches('-');
    // `-p=x` is the short flag of a password.
    let is_the_short_flag = name == "-p" && core[separator..].starts_with('=');
    if !is_the_short_flag && !names_a_secret(bare) {
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
        let text = "Please fix src/features/advisor/AdvisorView.tsx and commit 1e8ccc5a9f0b.";

        assert_eq!(hide_secrets(text), text);
    }

    #[test]
    fn short_commit_hashes_and_paths_are_kept() {
        for text in [
            "see commit 1e8ccc5 and 40cc186a",
            "show eff32b2d6a1c",
            "open /home/user/robotics/teamwright/crates/domain/src/secrets.rs",
            "mkdir -p crates/domain/src and cd ../src",
            "cp -p ./a.txt ~/b.txt",
            "the password is important and a token of thanks",
        ] {
            assert_eq!(hide_secrets(text), text);
        }
    }

    #[test]
    fn a_json_web_token_is_hidden() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";

        assert_eq!(
            hide_secrets(&format!("got {jwt} back")),
            "got [hidden] back"
        );
        assert_eq!(hide_secrets(&format!("({jwt}).")), "([hidden]).");
        // The signature of an unsigned token is empty.
        assert_eq!(
            hide_secrets("eyJhbGciOiJub25lIn0.eyJzdWIiOiJ4In0."),
            "[hidden]."
        );
        // Dots alone do not make a token.
        assert_eq!(
            hide_secrets("eyJ.only.two.dots.more"),
            "eyJ.only.two.dots.more"
        );
        assert_eq!(hide_secrets("version 1.2.3"), "version 1.2.3");
    }

    #[test]
    fn a_long_hexadecimal_key_is_hidden() {
        assert_eq!(
            hide_secrets("key 3f2a9c4e8b1d7a6f5e0c9b8a7d6e5f4a here"),
            "key [hidden] here"
        );
        assert_eq!(
            hide_secrets("sha 1e8ccc5a9f0b2c4d6e8f0a1b2c3d4e5f6a7b8c9d"),
            "sha [hidden]"
        );
        // 31 digits are not a key.
        assert_eq!(
            hide_secrets("3f2a9c4e8b1d7a6f5e0c9b8a7d6e5f4"),
            "3f2a9c4e8b1d7a6f5e0c9b8a7d6e5f4"
        );
    }

    #[test]
    fn the_value_of_a_flag_that_names_a_secret_is_hidden() {
        assert_eq!(
            hide_secrets("login --password hunter2 --user fouad"),
            "login --password [hidden] --user fouad"
        );
        assert_eq!(
            hide_secrets("tool --token abc123 run"),
            "tool --token [hidden] run"
        );
        assert_eq!(hide_secrets("x --secret s3cr3t"), "x --secret [hidden]");
        assert_eq!(hide_secrets("mysql -p hunter2"), "mysql -p [hidden]");
        assert_eq!(hide_secrets("mysql -p=hunter2"), "mysql -p=[hidden]");
        assert_eq!(hide_secrets("--password=hunter2"), "--password=[hidden]");
        // A flag after the flag is not its value.
        assert_eq!(
            hide_secrets("run --token --verbose"),
            "run --token --verbose"
        );
        // A path after -p is a path.
        assert_eq!(hide_secrets("mkdir -p out/logs"), "mkdir -p out/logs");
    }

    #[test]
    fn pairs_of_a_name_and_a_value_are_hidden() {
        assert_eq!(hide_secrets("token=abc123"), "token=[hidden]");
        assert_eq!(hide_secrets("api_key=abc123 x=1"), "api_key=[hidden] x=1");
        assert_eq!(hide_secrets("password=hunter2;"), "password=[hidden];");
        assert_eq!(
            hide_secrets(r#"{"password": "hunter2", "user": "fouad"}"#),
            r#"{"password": "[hidden]", "user": "fouad"}"#
        );
        assert_eq!(
            hide_secrets(r#"{"token":"abc123"}"#),
            r#"{"token":[hidden]"}"#
        );
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
