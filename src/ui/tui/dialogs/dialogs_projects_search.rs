use std::collections::{HashSet, VecDeque};
use std::fs;

use super::*;

const PROJECT_SEARCH_MAX_RESULTS: usize = 24;
const PROJECT_SEARCH_MAX_DIRS: usize = 2_500;
const PROJECT_SEARCH_MAX_DEPTH: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectPathSearchInput {
    pub(super) expanded_input: Option<PathBuf>,
    pub(super) search_root: PathBuf,
    pub(super) path_like: bool,
}

fn search_fallback_root() -> Option<PathBuf> {
    dirs::home_dir().or_else(|| std::env::current_dir().ok())
}

fn input_looks_like_path(input: &str) -> bool {
    let trimmed = input.trim();
    trimmed.starts_with("~/")
        || trimmed.starts_with("./")
        || trimmed.starts_with("../")
        || Path::new(trimmed).is_absolute()
        || trimmed.contains(std::path::MAIN_SEPARATOR)
}

fn expand_search_input_path(input: &str) -> Option<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(stripped) = trimmed.strip_prefix("~/") {
        return dirs::home_dir().map(|home| home.join(stripped));
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        return Some(path);
    }
    if trimmed.starts_with("./") || trimmed.starts_with("../") {
        return std::env::current_dir().ok().map(|cwd| cwd.join(path));
    }
    if trimmed.contains(std::path::MAIN_SEPARATOR) {
        return search_fallback_root().map(|root| root.join(path));
    }

    None
}

fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            if candidate.is_dir() {
                return Some(candidate.to_path_buf());
            }
            return candidate.parent().map(Path::to_path_buf);
        }
        current = candidate.parent();
    }
    None
}

pub(super) fn project_path_search_input(input: &str) -> Option<ProjectPathSearchInput> {
    let path_like = input_looks_like_path(input);
    let expanded_input = if path_like {
        expand_search_input_path(input)
    } else {
        None
    };

    let search_root = expanded_input
        .as_deref()
        .and_then(nearest_existing_ancestor)
        .or_else(search_fallback_root)?;

    Some(ProjectPathSearchInput {
        expanded_input,
        search_root,
        path_like,
    })
}

fn directory_is_repo_root(path: &Path) -> bool {
    path.join(".git").exists()
}

fn should_skip_project_search_dir(name: &str) -> bool {
    name == ".git"
        || name == "node_modules"
        || name == "target"
        || name == ".direnv"
        || name == ".cache"
        || name == ".local"
        || name == "Library"
        || (name.starts_with('.') && name != ".config")
}

fn ancestor_repo_roots(path: &Path) -> Vec<PathBuf> {
    path.ancestors()
        .filter(|ancestor| directory_is_repo_root(ancestor))
        .map(Path::to_path_buf)
        .collect()
}

pub(super) fn discover_project_repo_roots(search_root: &Path) -> Vec<PathBuf> {
    let mut discovered = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([(search_root.to_path_buf(), 0usize)]);
    let mut visited_dirs = 0usize;

    while let Some((dir, depth)) = queue.pop_front() {
        visited_dirs = visited_dirs.saturating_add(1);
        if visited_dirs > PROJECT_SEARCH_MAX_DIRS {
            break;
        }
        if !seen.insert(dir.clone()) {
            continue;
        }
        if directory_is_repo_root(&dir) {
            discovered.push(dir);
            if discovered.len() >= PROJECT_SEARCH_MAX_RESULTS {
                break;
            }
            continue;
        }
        if depth >= PROJECT_SEARCH_MAX_DEPTH {
            continue;
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut child_dirs = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let Ok(file_type) = entry.file_type() else {
                    return None;
                };
                if !file_type.is_dir() {
                    return None;
                }
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if should_skip_project_search_dir(name.as_ref()) {
                    return None;
                }
                Some(entry.path())
            })
            .collect::<Vec<_>>();
        child_dirs.sort();

        for child in child_dirs {
            queue.push_back((child, depth.saturating_add(1)));
        }
    }

    discovered
}

fn normalize_fuzzy_text(input: &str) -> String {
    input
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| if character == '\\' { '/' } else { character })
        .collect()
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let query_chars = query.chars().collect::<Vec<_>>();
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let mut search_start = 0usize;
    let mut previous_match: Option<usize> = None;
    let mut score = 0i64;

    for query_char in query_chars {
        let mut matched_index = None;
        for (index, candidate_char) in candidate_chars.iter().enumerate().skip(search_start) {
            if *candidate_char == query_char {
                matched_index = Some(index);
                break;
            }
        }
        let index = matched_index?;
        score += 4;
        if index == 0
            || matches!(
                candidate_chars[index.saturating_sub(1)],
                '/' | '-' | '_' | '.' | ' '
            )
        {
            score += 14;
        }
        if let Some(previous_index) = previous_match {
            if index == previous_index.saturating_add(1) {
                score += 18;
            } else {
                let gap = index.saturating_sub(previous_index.saturating_add(1));
                score -= i64::try_from(gap).unwrap_or(i64::MAX).min(8);
            }
        } else {
            score -= i64::try_from(index).unwrap_or(i64::MAX).min(12);
        }

        previous_match = Some(index);
        search_start = index.saturating_add(1);
    }

    if candidate.contains(query) {
        score += 40;
    }
    if candidate.starts_with(query) {
        score += 24;
    }

    score -= i64::try_from(candidate_chars.len() / 8).unwrap_or(0);
    Some(score)
}

pub(super) fn rank_project_path_matches(
    query: &str,
    expanded_input: Option<&Path>,
    candidates: &[PathBuf],
) -> Vec<ProjectPathMatch> {
    let query_basis = if input_looks_like_path(query) {
        expanded_input
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| query.trim().to_string())
    } else {
        query.trim().to_string()
    };
    let normalized_query = normalize_fuzzy_text(query_basis.as_str());
    let mut deduped = HashSet::new();
    let mut matches = Vec::new();

    for candidate in candidates {
        let candidate_key = candidate.to_string_lossy().to_string();
        if !deduped.insert(candidate_key.clone()) {
            continue;
        }

        let basename = project_display_name(candidate);
        let full_path = normalize_fuzzy_text(candidate_key.as_str());
        let basename_normalized = normalize_fuzzy_text(basename.as_str());
        let full_score = fuzzy_score(&normalized_query, &full_path);
        let basename_score = fuzzy_score(&normalized_query, &basename_normalized);
        let Some(mut score) = full_score.into_iter().chain(basename_score).max() else {
            continue;
        };

        if let Some(basename_score) = basename_score {
            score = score.saturating_add(basename_score.saturating_mul(3));
        }
        if let Some(expanded_input) = expanded_input {
            if expanded_input == candidate {
                score = score.saturating_add(200);
            } else if expanded_input.starts_with(candidate) {
                score = score.saturating_add(120);
            }
        }

        matches.push(ProjectPathMatch {
            path: candidate.clone(),
            score,
            already_added: false,
        });
    }

    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| {
                left.path
                    .components()
                    .count()
                    .cmp(&right.path.components().count())
            })
            .then_with(|| left.path.cmp(&right.path))
    });
    matches.truncate(PROJECT_SEARCH_MAX_RESULTS);
    matches
}

impl GroveApp {
    fn clear_project_add_dialog_matches(&mut self) {
        let Some(project_dialog) = self.project_dialog_mut() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_mut() else {
            return;
        };

        add_dialog.path_matches.clear();
        add_dialog.path_match_list.select(None);
    }

    pub(super) fn refresh_project_add_dialog_matches(&mut self) {
        let Some(project_dialog) = self.project_dialog() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_ref() else {
            return;
        };

        let query = add_dialog.path_input.value().trim().to_string();
        if query.is_empty() {
            self.clear_project_add_dialog_matches();
            return;
        }

        let Some(search_input) = project_path_search_input(&query) else {
            self.clear_project_add_dialog_matches();
            return;
        };
        if !search_input.path_like && query.chars().count() < 2 {
            self.clear_project_add_dialog_matches();
            return;
        }

        let previous_selected_path = add_dialog
            .selected_path_match()
            .map(|path_match| path_match.path.clone());

        let existing_project_paths = self
            .projects
            .iter()
            .map(|project| project.path.clone())
            .collect::<Vec<_>>();

        let Some(project_dialog) = self.project_dialog_mut() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_mut() else {
            return;
        };

        if add_dialog.cached_search_root.as_ref() != Some(&search_input.search_root) {
            add_dialog.cached_repo_roots = discover_project_repo_roots(&search_input.search_root);
            add_dialog.cached_search_root = Some(search_input.search_root.clone());
        }

        let mut candidates = add_dialog.cached_repo_roots.clone();
        if let Some(expanded_input) = search_input.expanded_input.as_deref() {
            candidates.extend(ancestor_repo_roots(expanded_input));
        } else {
            candidates.extend(ancestor_repo_roots(&search_input.search_root));
        }

        add_dialog.path_matches =
            rank_project_path_matches(&query, search_input.expanded_input.as_deref(), &candidates);
        for path_match in &mut add_dialog.path_matches {
            path_match.already_added = existing_project_paths
                .iter()
                .any(|project_path| refer_to_same_location(project_path, &path_match.path));
        }

        if add_dialog.path_matches.is_empty() {
            add_dialog.path_match_list.select(None);
            return;
        }

        if let Some(previous_selected_path) = previous_selected_path
            && let Some(index) = add_dialog
                .path_matches
                .iter()
                .position(|path_match| path_match.path == previous_selected_path)
        {
            add_dialog.set_selected_path_match_index(index);
            return;
        }

        add_dialog.set_selected_path_match_index(0);
    }

    pub(super) fn accept_selected_project_add_path_match(&mut self) {
        let Some(project_dialog) = self.project_dialog() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_ref() else {
            return;
        };
        let Some(path_match) = add_dialog.selected_path_match() else {
            return;
        };
        if path_match.already_added {
            return;
        }
        let selected_path = path_match.path.clone();

        {
            let Some(project_dialog) = self.project_dialog_mut() else {
                return;
            };
            let Some(add_dialog) = project_dialog.add_dialog.as_mut() else {
                return;
            };

            add_dialog
                .path_input
                .set_value(selected_path.display().to_string());
            if add_dialog.name_input.value().trim().is_empty() {
                add_dialog
                    .name_input
                    .set_value(project_display_name(&selected_path));
            }
            add_dialog.path_matches.clear();
            add_dialog.path_match_list.select(None);
        }
        self.focus_dialog_field(FOCUS_ID_PROJECT_ADD_ADD_BUTTON);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[derive(Debug)]
    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "grove-project-search-{label}-{}-{timestamp}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test dir should exist");
            Self { path }
        }

        fn repo(&self, relative: &str) -> PathBuf {
            let path = self.path.join(relative);
            fs::create_dir_all(path.join(".git")).expect("repo .git directory should exist");
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn project_path_search_input_uses_nearest_existing_ancestor() {
        let dir = TestDir::new("ancestor");
        let root = dir.path.join("code");
        fs::create_dir_all(&root).expect("search root should exist");
        let typed_path = root.join("grove").join("src").join("ui");
        let raw = typed_path.display().to_string();

        let input = project_path_search_input(&raw).expect("search input should resolve");

        assert_eq!(input.search_root, root);
        assert_eq!(input.expanded_input, Some(typed_path));
        assert!(input.path_like);
    }

    #[test]
    fn discover_project_repo_roots_finds_nested_git_directories() {
        let dir = TestDir::new("discover");
        let repo_a = dir.repo("code/grove");
        let repo_b = dir.repo("code/frankentui");

        let discovered = discover_project_repo_roots(&dir.path.join("code"));

        assert_eq!(discovered, vec![repo_b, repo_a]);
    }

    #[test]
    fn rank_project_path_matches_prefers_basename_and_ancestor_repo() {
        let candidates = vec![
            PathBuf::from("/Users/test/code/grove"),
            PathBuf::from("/Users/test/archive/old-grove-docs"),
            PathBuf::from("/Users/test/code/frankentui"),
        ];
        let typed_path = PathBuf::from("/Users/test/code/grove/src/ui/tui");

        let ranked = rank_project_path_matches("grove", Some(&typed_path), &candidates);

        assert_eq!(
            ranked
                .into_iter()
                .map(|path_match| path_match.path)
                .collect::<Vec<_>>(),
            vec![
                PathBuf::from("/Users/test/code/grove"),
                PathBuf::from("/Users/test/archive/old-grove-docs"),
            ]
        );
    }

    #[test]
    fn rank_project_path_matches_expands_tilde_queries_before_scoring() {
        let candidates = vec![
            PathBuf::from("/Users/test/projects/grove"),
            PathBuf::from("/Users/test/projects/frankentui"),
        ];
        let expanded_input = PathBuf::from("/Users/test/projects/");

        let ranked = rank_project_path_matches("~/projects/", Some(&expanded_input), &candidates);
        let ranked_paths = ranked
            .into_iter()
            .map(|path_match| path_match.path)
            .collect::<Vec<_>>();

        assert_eq!(ranked_paths.len(), candidates.len());
        assert!(ranked_paths.contains(&PathBuf::from("/Users/test/projects/grove")));
        assert!(ranked_paths.contains(&PathBuf::from("/Users/test/projects/frankentui")));
    }
}
