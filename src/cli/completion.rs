//! Completion-v1: words exclude the executable and include the current word.
//! Responses contain one plain candidate per line, without descriptions/secrets.
use clap::{Arg, Command};
use std::{collections::BTreeSet, path::Path};

fn candidates(mut command: Command, words: &[String]) -> Vec<String> {
    command.build();
    let current = words.last().map_or("", String::as_str);
    let completed = &words[..words.len().saturating_sub(1)];
    let mut pending: Option<Arg> = None;
    let mut position = 1;
    let mut flags = true;
    let mut used = BTreeSet::new();
    for word in completed {
        if pending.take().is_some() {
            continue;
        }
        if flags && word == "--" {
            flags = false;
            continue;
        }
        if flags && word.starts_with('-') {
            let flag = word.split('=').next().unwrap_or(word);
            if let Some(arg) = find_flag(&command, flag) {
                used.insert(arg.get_id().to_string());
                if !word.contains('=') && arg.get_action().takes_values() {
                    pending = Some(arg.clone());
                }
            }
            continue;
        }
        if let Some(sub) = command.find_subcommand(word).cloned() {
            command = sub;
            position = 1;
        } else {
            if let Some(arg) = command
                .get_positionals()
                .find(|arg| arg.get_index() == Some(position))
            {
                used.insert(arg.get_id().to_string());
            }
            position += 1;
        }
    }
    if let Some(arg) = pending {
        return values(&arg, current);
    }
    if flags
        && let Some((flag, prefix)) = current.split_once('=')
        && let Some(arg) = command.get_arguments().find(|arg| {
            arg.get_long()
                .is_some_and(|long| flag == format!("--{long}"))
        })
    {
        return values(arg, prefix)
            .into_iter()
            .map(|value| format!("{flag}={value}"))
            .collect();
    }
    let blocked: BTreeSet<_> = command
        .get_arguments()
        .filter(|arg| used.contains(arg.get_id().as_str()))
        .flat_map(|arg| command.get_arg_conflicts_with(arg))
        .map(|arg| arg.get_id().to_string())
        .collect();
    let mut result = Vec::new();
    if flags {
        for arg in command.get_arguments().filter(|arg| {
            !arg.is_hide_set()
                && (!used.contains(arg.get_id().as_str())
                    || matches!(arg.get_action(), clap::ArgAction::Append))
                && !blocked.contains(arg.get_id().as_str())
                && !command
                    .get_arg_conflicts_with(arg)
                    .iter()
                    .any(|conflict| used.contains(conflict.get_id().as_str()))
        }) {
            if let Some(long) = arg.get_long() {
                result.push(format!("--{long}"));
            }
            if let Some(short) = arg.get_short() {
                result.push(format!("-{short}"));
            }
        }
    }
    if !current.starts_with('-') {
        for sub in command.get_subcommands().filter(|sub| !sub.is_hide_set()) {
            result.push(sub.get_name().to_owned());
            result.extend(sub.get_visible_aliases().map(str::to_owned));
        }
        if let Some(arg) = command
            .get_positionals()
            .find(|arg| arg.get_index() == Some(position))
        {
            result.extend(values(arg, current));
        }
    }
    result.retain(|value| value.starts_with(current) && !value.chars().any(char::is_control));
    result.sort();
    result.dedup();
    result
}
fn values(arg: &Arg, prefix: &str) -> Vec<String> {
    let id = arg.get_id().as_str();
    let mut result = if matches!(id, "config" | "input" | "output") {
        paths(prefix)
    } else {
        arg.get_possible_values()
            .into_iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_owned())
            .collect()
    };
    result.retain(|value| value.starts_with(prefix) && !value.chars().any(char::is_control));
    result.sort();
    result.dedup();
    result
}
fn paths(prefix: &str) -> Vec<String> {
    let split = prefix.rfind(['/', '\\']).map_or(0, |index| index + 1);
    let parent = &prefix[..split];
    let directory = if parent.is_empty() {
        Path::new(".")
    } else {
        Path::new(parent)
    };
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if prefix[split..].is_empty() && name.starts_with('.') {
                return None;
            }
            Some(format!(
                "{parent}{name}{}",
                if entry.file_type().ok()?.is_dir() {
                    "/"
                } else {
                    ""
                }
            ))
        })
        .collect()
}

/// Handle the host's read-only completion query before normal CLI dispatch.
#[must_use]
pub fn handle(args: &[std::ffi::OsString]) -> bool {
    use clap::CommandFactory;
    if args.first().is_none_or(|arg| arg != "__complete") {
        return false;
    }
    let words: Vec<_> = args
        .iter()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let mut bytes = 0;
    for candidate in candidates(super::opts::Cli::command(), &words)
        .into_iter()
        .take(1000)
    {
        bytes += candidate.len() + 1;
        if bytes > 64 * 1024 {
            break;
        }
        println!("{candidate}");
    }
    true
}

fn find_flag<'a>(command: &'a Command, flag: &str) -> Option<&'a Arg> {
    command.get_arguments().find(|arg| {
        arg.get_long()
            .is_some_and(|long| flag == format!("--{long}"))
            || arg
                .get_short()
                .is_some_and(|short| flag == format!("-{short}"))
    })
}
