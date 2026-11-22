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
    /// The value of a flag such as `--password`, or of `-p` after a
    /// program whose `-p` takes a password. It is hidden unless it is
    /// another flag.
    ValueOfFlag,
    /// The word after the bare name of a secret, as in `password = hunter2`:
    /// if it is only `=` or `:`, the word after it is the secret.
    AfterSecretName,
}

/// The programs whose `-p` takes a password, glued (`-pS3cr3t`) or, for
/// some, as the next word. Elsewhere `-p` means something else: a port
/// (`ssh -p 2222`), a patch (`git add -p`), parents (`mkdir -p`), so the
/// word after it is kept. `psql` is not here: its `-p` is a port.
const PASSWORD_FLAG_PROGRAMS: &[&str] = &[
    "mysql",
    "mysqldump",
    "mysqladmin",
    "mysqlimport",
    "mysqlshow",
    "mysqlcheck",
    "mariadb",
    "mariadb-dump",
    "mariadb-admin",
    "sshpass",
];

/// Words in a name written in capitals, such as `PGPASSWORD`, that say its
/// value is a secret.
const SECRET_CAPITALS: &[&str] = &["PASSWORD", "PASSWD", "SECRET", "TOKEN", "KEY"];

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
/// `password=`, `token:`, `password = ` or `PGPASSWORD=`, the word after
/// `Bearer`, the password of `-p` in a command of a program whose `-p`
/// takes one ([`PASSWORD_FLAG_PROGRAMS`]), the name and password in an
/// address such as `https://name:secret@host`, and private keys. The rest
/// of the text, its spaces and its line breaks included, is kept as it was.
#[must_use]
pub fn hide_secrets(text: &str) -> String {
    let text = hide_private_keys(text);
    let mut hidden = String::with_capacity(text.len());
    let mut state = State {
        next: Next::Ordinary,
        password_program: false,
    };
    let mut word = String::new();
    for character in text.chars() {
        if character.is_whitespace() {
            if !word.is_empty() {
                state = push_word(&mut hidden, &word, state);
                word.clear();
            }
            if matches!(character, '\n' | '\r') {
                // A new line is a new command.
                state.password_program = false;
            }
            hidden.push(character);
        } else {
            word.push(character);
        }
    }
    if !word.is_empty() {
        push_word(&mut hidden, &word, state);
    }
    hidden
}

/// What the words read so far say about the next one.
#[derive(Debug, Clone, Copy)]
struct State {
    /// What the next word must be.
    next: Next,
    /// Whether the command being read named a program whose `-p` takes a
    /// password, anywhere before (`sudo mysql`, `docker exec db mysql`). A
    /// line break, or a word that ends with `;`, `|` or `&`, ends a
    /// command.
    password_program: bool,
}

/// Adds one word to the text, hidden if it must be, and says what the words
/// after it must be.
fn push_word(hidden: &mut String, word: &str, state: State) -> State {
    let mut password_program = state.password_program || names_a_password_program(word);
    let next = push_word_in(hidden, word, state.next, password_program);
    if word.ends_with([';', '|', '&']) {
        password_program = false;
    }
    State {
        next,
        password_program,
    }
}

/// Whether a word names a program whose `-p` takes a password, by its name
/// or its path: `mysql`, `/usr/bin/mysql`, `mysql.exe`.
fn names_a_password_program(word: &str) -> bool {
    let (_, core, _) = strip_marks(word);
    let name = core.rsplit(['/', '\\']).next().unwrap_or(core);
    let lowered = name.to_ascii_lowercase();
    let name = lowered.strip_suffix(".exe").unwrap_or(&lowered);
    PASSWORD_FLAG_PROGRAMS.contains(&name)
}

/// Adds one word to the text, hidden if it must be, and says what the word
/// after it must be. `password_program` says whether `-p` takes a password
/// in this command.
fn push_word_in(hidden: &mut String, word: &str, expected: Next, password_program: bool) -> Next {
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
        Next::ValueOfFlag => {
            if !word.starts_with('-') {
                push_hidden(hidden, word);
                return Next::Ordinary;
            }
        }
        Next::AfterSecretName => {
            // "password = hunter2", "password =hunter2", "password : x".
            let separators = word.len() - word.trim_start_matches(['=', ':']).len();
            if separators > 0 {
                hidden.push_str(&word[..separators]);
                if separators == word.len() {
                    return Next::Secret;
                }
                push_hidden(hidden, &word[separators..]);
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
    // "mysql -uroot -pS3cr3t": the password is glued to the flag.
    if password_program
        && let Some(glued) = core.strip_prefix("-p")
        && !glued.is_empty()
        && !glued.starts_with('=')
    {
        hidden.push_str(before);
        hidden.push_str("-p");
        hidden.push_str(HIDDEN);
        hidden.push_str(after);
        return Next::Ordinary;
    }
    hidden.push_str(word);
    // "Bearer abc…": the token is the next word.
    if is_a_kind_of_credential(core) {
        return Next::Secret;
    }
    // "--password abc", and "-p abc" after a program whose -p takes a
    // password. Elsewhere "-p" is a port, a patch or parents.
    if (core == "-p" && password_program)
        || (core.starts_with("--") && names_a_secret(core.trim_start_matches('-')))
    {
        return Next::ValueOfFlag;
    }
    // "password = abc": the value may come after a separate "=".
    if !core.starts_with('-') && (names_a_secret(core) || is_a_capital_secret_name(core)) {
        return Next::AfterSecretName;
    }
    Next::Ordinary
}

/// Adds a word as [`HIDDEN`], keeping the quotation marks and punctuation
/// around it.
fn push_hidden(hidden: &mut String, word: &str) {
    let (before, _, after) = strip_marks(word);
    hidden.push_str(before);
    hidden.push_str(HIDDEN);
    hidden.push_str(after);
}

/// Whether a name written in capitals, digits and underscores names a
/// secret: `PGPASSWORD`, `MY_SECRET_2`, `GITHUB_TOKEN`, `AWS_KEY`.
fn is_a_capital_secret_name(name: &str) -> bool {
    name.chars().any(|character| character.is_ascii_uppercase())
        && name.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
        && SECRET_CAPITALS.iter().any(|word| name.contains(word))
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
    if !is_the_short_flag && !names_a_secret(bare) && !is_a_capital_secret_name(bare) {
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
    fn a_password_glued_to_the_p_of_a_database_program_is_hidden() {
        assert_eq!(
            hide_secrets("mysql -uroot -pS3cr3tPass mydb"),
            "mysql -uroot -p[hidden] mydb"
        );
        assert_eq!(
            hide_secrets("sudo /usr/bin/mysqldump -u root -pHunter2 shop > shop.sql"),
            "sudo /usr/bin/mysqldump -u root -p[hidden] shop > shop.sql"
        );
        assert_eq!(
            hide_secrets("docker exec db mariadb -p'hunter 2'"),
            "docker exec db mariadb -p[hidden] 2'"
        );
        assert_eq!(
            hide_secrets("sshpass -p hunter2 ssh pi@robot"),
            "sshpass -p [hidden] ssh pi@robot"
        );
    }

    #[test]
    fn the_word_after_p_is_kept_where_p_is_not_a_password() {
        for text in [
            "ssh -p 2222 host",
            "scp -P 2222 a host:b",
            "git add -p file.rs",
            "psql -h localhost -p 5432 -U me shop",
            "mkdir -p out",
            "ssh -p2222 host",
            // A new command after a database program.
            "mysql -u me shop; ssh -p 2222 host",
            "mysql -u me shop\nssh -p 2222 host",
        ] {
            assert_eq!(hide_secrets(text), text);
        }
    }

    #[test]
    fn a_secret_after_a_name_and_a_separate_equals_sign_is_hidden() {
        assert_eq!(hide_secrets("password = hunter2"), "password = [hidden]");
        assert_eq!(hide_secrets("password =hunter2"), "password =[hidden]");
        assert_eq!(hide_secrets("token : abc123 x"), "token : [hidden] x");
        assert_eq!(
            hide_secrets("PGPASSWORD = hunter2 psql"),
            "PGPASSWORD = [hidden] psql"
        );
        // A word after the name that is not a separator is kept.
        assert_eq!(hide_secrets("the password is good"), "the password is good");
    }

    #[test]
    fn a_value_after_a_name_in_capitals_that_says_secret_is_hidden() {
        assert_eq!(
            hide_secrets("PGPASSWORD=hunter2 psql shop"),
            "PGPASSWORD=[hidden] psql shop"
        );
        assert_eq!(
            hide_secrets("export GITHUBTOKEN=abc MYSECRET2=x AWSKEY=y"),
            "export GITHUBTOKEN=[hidden] MYSECRET2=[hidden] AWSKEY=[hidden]"
        );
        assert_eq!(
            hide_secrets("MYSQL_PWD_PASSWD=x"),
            "MYSQL_PWD_PASSWD=[hidden]"
        );
        // In capitals only: a word that merely holds "key" stays.
        assert_eq!(hide_secrets("monkey=1 Keyboard=us"), "monkey=1 Keyboard=us");
        assert_eq!(hide_secrets("PORT=8080 HOME=/x"), "PORT=8080 HOME=/x");
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
