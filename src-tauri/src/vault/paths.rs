use std::path::{Path, PathBuf};

/// Resolve the vault root.
///
/// `$GRIND_VAULT` wins, then the folder the user picked in the app, then a `vault` directory
/// next to the repo, then the caller-supplied fallback (the app data dir).
///
/// The repo probe is debug-only on purpose. `CARGO_MANIFEST_DIR` is baked in at compile
/// time, so in a packaged build it names a folder on the *developer's* machine — often under
/// Desktop or Documents. Calling `is_dir()` on it is enough to make macOS raise a
/// folder-access prompt on first launch, for a folder the app has no reason to read. A
/// release build with no configured vault therefore touches nothing and waits for the setup
/// screen to say where the vault is.
pub fn resolve_vault_root(chosen: Option<PathBuf>, fallback: Option<PathBuf>) -> PathBuf {
    if let Some(env) = std::env::var_os("GRIND_VAULT") {
        return PathBuf::from(env);
    }
    if let Some(chosen) = chosen {
        return chosen;
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("vault");
    if cfg!(debug_assertions) && dev.is_dir() {
        return dev.canonicalize().unwrap_or(dev);
    }
    fallback.unwrap_or(dev)
}

/// Directories a vault needs before anything can be written into it.
const VAULT_DIRS: [&str; 5] = [
    "syllabus",
    "knowledge",
    "quizzes",
    "progress/attempts",
    "progress/sessions",
];

/// Create the directory skeleton, and a sample syllabus when there is not a single one — an
/// empty vault gives the student nothing to press, and the file format is the one thing the
/// app cannot infer for them.
pub fn create_vault(root: &Path) -> std::io::Result<()> {
    for dir in VAULT_DIRS {
        std::fs::create_dir_all(root.join(dir))?;
    }
    let sample = root.join("syllabus").join("sample.md");
    let empty = std::fs::read_dir(root.join("syllabus"))
        .map(|entries| {
            !entries
                .flatten()
                .any(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("md"))
        })
        .unwrap_or(true);
    if empty {
        std::fs::write(&sample, SAMPLE_SYLLABUS)?;
    }
    Ok(())
}

/// The syllabus grammar by example: `# Title`, `## Section`, numbered topic lines.
const SAMPLE_SYLLABUS: &str = "\
# Приклад предмета

## 1. Назва розділу

1. Перша тема розділу.
2. Друга тема розділу: підпитання після двокрапки теж належать темі.
3. Третя тема розділу.

## 2. Другий розділ

1. Нумерація починається заново в кожному розділі.
2. Вкладена нумерація теж працює:
   1.1. Підтема — окрема тема зі своїм конспектом.
   1.2. Ще одна підтема.

Замініть цей файл на власний силабус: одна тека `syllabus/`, один файл на предмет,
ім'я файлу стає ідентифікатором предмета.
";

/// Accept either the vault itself or a folder containing one, so picking the project
/// directory by mistake still works.
pub fn normalise_vault_choice(picked: &Path) -> PathBuf {
    if picked.join("syllabus").is_dir() {
        return picked.to_path_buf();
    }
    let nested = picked.join("vault");
    if nested.join("syllabus").is_dir() {
        return nested;
    }
    picked.to_path_buf()
}

/// Count the syllabus files, or say why the vault cannot be read.
///
/// On macOS a denied folder-access prompt surfaces here as `PermissionDenied`, which means
/// something completely different from "you picked the wrong folder" and must not be
/// reported as the same thing.
pub fn describe_vault(root: &Path) -> Result<usize, String> {
    let syllabus = root.join("syllabus");
    match std::fs::read_dir(&syllabus) {
        Ok(entries) => Ok(entries
            .flatten()
            .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("md"))
            .count()),
        Err(error) => Err(match error.kind() {
            std::io::ErrorKind::PermissionDenied => format!(
                "macOS не дозволяє застосунку читати «{}». Оберіть теку сховища через системне вікно — такий вибір надає доступ явно.",
                root.display()
            ),
            std::io::ErrorKind::NotFound => format!(
                "У теці «{}» немає підтеки syllabus — це не схоже на сховище.",
                root.display()
            ),
            _ => format!("Не вдалося прочитати «{}»: {error}", syllabus.display()),
        }),
    }
}

#[derive(Debug, Clone)]
pub struct Vault {
    pub root: PathBuf,
}

impl Vault {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn syllabus_dir(&self) -> PathBuf {
        self.root.join("syllabus")
    }

    pub fn knowledge_dir(&self, subject: &str) -> PathBuf {
        self.root.join("knowledge").join(subject)
    }

    pub fn quizzes_dir(&self, subject: &str) -> PathBuf {
        self.root.join("quizzes").join(subject)
    }

    pub fn attempts_dir(&self) -> PathBuf {
        self.root.join("progress").join("attempts")
    }

    pub fn mastery_file(&self) -> PathBuf {
        self.root.join("progress").join("mastery.json")
    }

    pub fn study_file(&self) -> PathBuf {
        self.root.join("progress").join("study.json")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("progress").join("sessions")
    }

    /// `vault/knowledge/<subject>/02.07-nazva-temy.md`
    ///
    /// The `section.topic` prefix keeps directory order aligned with the syllabus and stays
    /// stable when unrelated topics are added.
    pub fn knowledge_file(
        &self,
        subject: &str,
        section: usize,
        topic: usize,
        title: &str,
    ) -> PathBuf {
        self.knowledge_dir(subject).join(format!(
            "{:02}.{:02}-{}.md",
            section,
            topic,
            slugify(title)
        ))
    }
}

/// Transliterate Ukrainian/Russian Cyrillic to an ASCII slug.
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    for ch in input.to_lowercase().chars() {
        let mapped: &str = match ch {
            'а' => "a",
            'б' => "b",
            'в' => "v",
            'г' => "h",
            'ґ' => "g",
            'д' => "d",
            'е' => "e",
            'є' => "ie",
            'ж' => "zh",
            'з' => "z",
            'и' => "y",
            'і' => "i",
            'ї' => "i",
            'й' => "i",
            'к' => "k",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'о' => "o",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'у' => "u",
            'ф' => "f",
            'х' => "kh",
            'ц' => "ts",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "shch",
            'ю' => "iu",
            'я' => "ia",
            'ы' => "y",
            'э' => "e",
            'ь' | 'ъ' | 'ʼ' | '\'' | '’' | '`' => "",
            c if c.is_ascii_alphanumeric() => {
                out.push(c);
                continue;
            }
            _ => "-",
        };
        out.push_str(mapped);
    }

    // collapse separators and trim to a sane filename length on a char boundary
    let mut slug = String::with_capacity(out.len());
    for c in out.chars() {
        if c == '-' {
            if !slug.ends_with('-') && !slug.is_empty() {
                slug.push('-');
            }
        } else {
            slug.push(c);
        }
    }
    let slug = slug.trim_matches('-');
    match slug.char_indices().nth(60) {
        None => slug.to_string(),
        Some((cut, _)) => slug[..cut].trim_end_matches('-').to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_ukrainian() {
        assert_eq!(slugify("Приклад назви теми"), "pryklad-nazvy-temy");
        // An ASCII abbreviation survives; the separator it leaves behind is collapsed into
        // the one the following space would have produced.
        assert_eq!(
            slugify("HTTP- запити та відповіді"),
            "http-zapyty-ta-vidpovidi"
        );
        assert_eq!(slugify("  ---  "), "");
    }

    #[test]
    fn accepts_a_parent_folder_that_contains_the_vault() {
        let dir = std::env::temp_dir().join(format!("grind-vault-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("vault").join("syllabus")).unwrap();

        assert_eq!(normalise_vault_choice(&dir), dir.join("vault"));
        assert_eq!(
            normalise_vault_choice(&dir.join("vault")),
            dir.join("vault")
        );
        // Nothing that looks like a vault: keep the choice and let `describe_vault` explain.
        let empty = dir.join("elsewhere");
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(normalise_vault_choice(&empty), empty);
        assert!(describe_vault(&empty).is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn truncates_without_splitting_chars() {
        let long = slugify(&"тема ".repeat(40));
        assert!(long.len() <= 60);
        assert!(!long.ends_with('-'));
    }
}
