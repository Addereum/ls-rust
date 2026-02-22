use atty::Stream;
use chrono::{DateTime, Local};
use clap::{Parser, ValueEnum, ArgAction};
use humansize::{format_size, DECIMAL};
use owo_colors::OwoColorize;
use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs::{self, DirEntry, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    name = "ruls",
    version,
    about = "Rust ls clone (common flags)",
    disable_help_flag = true
)]
struct Args {
    #[arg(long = "help", action = ArgAction::Help)]
    _help: Option<bool>,   // WICHTIG: Option<bool> oder kein normales bool

    #[arg(short = 'a', long = "all")]
    all: bool,

    #[arg(short = 'A', long = "almost-all")]
    almost_all: bool,

    #[arg(short = 'l', long = "long")]
    long: bool,

    #[arg(short = 'h', long = "human-readable")]
    human_readable: bool,

    #[arg(short = 'R', long = "recursive")]
    recursive: bool,

    #[arg(short = 'r', long = "reverse")]
    reverse: bool,

    #[arg(short = 't', long = "time")]
    sort_time: bool,

    #[arg(short = 'S', long = "size")]
    sort_size: bool,

    #[arg(short = '1', long = "one-per-line")]
    one_per_line: bool,

    #[arg(short = 'F', long = "classify")]
    classify: bool,

    #[arg(long = "dirs-first")]
    dirs_first: bool,

    #[arg(long = "color", value_enum, default_value_t = ColorWhen::Auto)]
    color: ColorWhen,

    #[arg(value_name = "PATH", default_value = ".", num_args = 0..)]
    paths: Vec<std::path::PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone)]
struct Item {
    path: PathBuf,
    file_name: OsString,
    meta: Metadata,
    // For symlinks we keep extra info; on many platforms meta follows symlink vs link differs.
    is_symlink: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let use_color = match args.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => atty::is(Stream::Stdout),
    };

    let multiple = args.paths.len() > 1;

    for (i, p) in args.paths.iter().enumerate() {
        if i > 0 {
            println!();
        }

        if multiple {
            println!("{}:", p.display());
        }

        if args.recursive {
            list_recursive(p, &args, use_color)?;
        } else {
            list_single_dir_or_file(p, &args, use_color)?;
        }
    }

    Ok(())
}

fn list_recursive(path: &Path, args: &Args, use_color: bool) -> io::Result<()> {
    // If path is file -> just print it
    if let Ok(m) = fs::symlink_metadata(path) {
        if !m.is_dir() {
            let item = mk_item_from_path(path.to_path_buf(), &m)?;
            print_items(&[item], args, use_color, None)?;
            return Ok(());
        }
    }

    // WalkDir includes root directory itself; we print per directory like `ls -R`.
    let mut first_dir = true;
    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
    {
        let dir_path = entry.path().to_path_buf();

        if !first_dir {
            println!();
        }
        first_dir = false;

        println!("{}:", dir_path.display());
        list_dir(&dir_path, args, use_color)?;
    }
    Ok(())
}

fn list_single_dir_or_file(path: &Path, args: &Args, use_color: bool) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        list_dir(path, args, use_color)
    } else {
        let item = mk_item_from_path(path.to_path_buf(), &meta)?;
        print_items(&[item], args, use_color, None)
    }
}

fn list_dir(path: &Path, args: &Args, use_color: bool) -> io::Result<()> {
    let mut items = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if !should_include(&entry, args) {
            continue;
        }
        let item = mk_item_from_entry(entry)?;
        items.push(item);
    }

    sort_items(&mut items, args);
    print_items(&items, args, use_color, Some(path))?;
    Ok(())
}

fn should_include(entry: &DirEntry, args: &Args) -> bool {
    let name = entry.file_name();
    let name = name.to_string_lossy();

    let is_dot = name.starts_with('.');
    if !is_dot {
        return true;
    }

    if args.all {
        return true;
    }

    if args.almost_all {
        return name != "." && name != "..";
    }

    false
}

fn mk_item_from_entry(entry: DirEntry) -> io::Result<Item> {
    // symlink_metadata doesn't follow symlink
    let meta = fs::symlink_metadata(entry.path())?;
    let is_symlink = meta.file_type().is_symlink();
    Ok(Item {
        path: entry.path(),
        file_name: entry.file_name(),
        meta,
        is_symlink,
    })
}

fn mk_item_from_path(path: PathBuf, meta: &Metadata) -> io::Result<Item> {
    let file_name = path
        .file_name()
        .map(|s| s.to_os_string())
        .unwrap_or_else(|| OsString::from(path.as_os_str()));
    Ok(Item {
        path,
        file_name,
        meta: meta.clone(),
        is_symlink: meta.file_type().is_symlink(),
    })
}

fn sort_items(items: &mut [Item], args: &Args) {
    items.sort_by(|a, b| compare_items(a, b, args));
    if args.reverse {
        items.reverse();
    }
}

fn compare_items(a: &Item, b: &Item, args: &Args) -> Ordering {
    // dirs-first (optional primary key)
    if args.dirs_first {
        let ad = a.meta.is_dir();
        let bd = b.meta.is_dir();
        match (ad, bd) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
    }

    if args.sort_size {
        let sa = a.meta.len();
        let sb = b.meta.len();
        match sb.cmp(&sa) {
            Ordering::Equal => {}
            ord => return ord,
        }
    } else if args.sort_time {
        let ta = mtime(&a.meta);
        let tb = mtime(&b.meta);
        match tb.cmp(&ta) {
            Ordering::Equal => {}
            ord => return ord,
        }
    }

    // fallback: name sort (case-sensitive like many ls defaults)
    a.file_name.cmp(&b.file_name)
}

fn print_items(items: &[Item], args: &Args, use_color: bool, base_dir: Option<&Path>) -> io::Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    if args.long {
        for it in items {
            let line = format_long(it, args, use_color)?;
            println!("{line}");
        }
        return Ok(());
    }

    // Simple mode: one-per-line vs space-separated (basic)
    if args.one_per_line {
        for it in items {
            println!("{}", format_name(it, args, use_color)?);
        }
    } else {
        // Minimal "columns": just join by two spaces (not terminal-width aware).
        let mut first = true;
        for it in items {
            if !first {
                print!("  ");
            }
            first = false;
            print!("{}", format_name(it, args, use_color)?);
        }
        println!();
    }

    // base_dir unused now but kept for easy extension (relative path printing, etc.)
    let _ = base_dir;
    Ok(())
}

fn format_long(it: &Item, args: &Args, use_color: bool) -> io::Result<String> {
    let perms = format_permissions(&it.meta);
    let nlink = format_nlink(&it.meta);
    let owner = format_owner(&it.meta);
    let group = format_group(&it.meta);
    let size = format_size_field(it, args);
    let time = format_mtime(&it.meta);
    let name = format_name(it, args, use_color)?;

    // symlink target (unix-ish behavior)
    let link_part = if it.is_symlink {
        match fs::read_link(&it.path) {
            Ok(target) => format!(" -> {}", target.display()),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    Ok(format!("{perms} {nlink:>2} {owner:<8} {group:<8} {size:>8} {time} {name}{link_part}"))
}

fn format_name(it: &Item, args: &Args, use_color: bool) -> io::Result<String> {
    let base = it.file_name.to_string_lossy().to_string();
    let mut s = if use_color {
        colorize_name(it, &base)
    } else {
        base
    };

    if args.classify {
        s.push_str(classify_suffix(it));
    }

    Ok(s)
}

fn classify_suffix(it: &Item) -> &'static str {
    // Match common ls -F indicators
    if it.meta.is_dir() {
        "/"
    } else if it.is_symlink {
        "@"
    } else if is_executable(&it.meta) {
        "*"
    } else {
        ""
    }
}

fn colorize_name(it: &Item, name: &str) -> String {
    // Basic scheme:
    // - dirs: blue
    // - symlinks: cyan
    // - executables: green
    // - others: default
    if it.meta.is_dir() {
        name.blue().to_string()
    } else if it.is_symlink {
        name.cyan().to_string()
    } else if is_executable(&it.meta) {
        name.green().to_string()
    } else {
        name.to_string()
    }
}

fn format_size_field(it: &Item, args: &Args) -> String {
    if args.human_readable {
        format_size(it.meta.len(), DECIMAL)
    } else {
        it.meta.len().to_string()
    }
}

fn format_mtime(meta: &Metadata) -> String {
    let dt: DateTime<Local> = meta
        .modified()
        .ok()
        .and_then(|st| DateTime::<Local>::from(st).into())
        .unwrap_or_else(|| Local::now());

    dt.format("%b %e %H:%M").to_string()
}

fn mtime(meta: &Metadata) -> u64 {
    // cross-platform-ish: std::time::SystemTime conversion
    meta.modified()
        .ok()
        .and_then(|st| st.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn format_permissions(meta: &Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();

        let file_type = if meta.is_dir() {
            'd'
        } else if meta.file_type().is_symlink() {
            'l'
        } else {
            '-'
        };

        let bits = [
            (mode & 0o400 != 0, 'r'),
            (mode & 0o200 != 0, 'w'),
            (mode & 0o100 != 0, 'x'),
            (mode & 0o040 != 0, 'r'),
            (mode & 0o020 != 0, 'w'),
            (mode & 0o010 != 0, 'x'),
            (mode & 0o004 != 0, 'r'),
            (mode & 0o002 != 0, 'w'),
            (mode & 0o001 != 0, 'x'),
        ];

        let mut s = String::with_capacity(10);
        s.push(file_type);
        for (set, ch) in bits {
            s.push(if set { ch } else { '-' });
        }
        s
    }

    #[cfg(not(unix))]
    {
        // Fallback
        if meta.is_dir() {
            "d---------".to_string()
        } else {
            "----------".to_string()
        }
    }
}
fn is_executable(_meta: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        _meta.permissions().mode() & 0o111 != 0 && !_meta.is_dir()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn format_nlink(_meta: &Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        _meta.nlink().to_string()
    }
    #[cfg(not(unix))]
    {
        "1".to_string()
    }
}

fn format_owner(_meta: &Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        _meta.uid().to_string()
    }
    #[cfg(not(unix))]
    {
        "-".to_string()
    }
}

fn format_group(_meta: &Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        _meta.gid().to_string()
    }
    #[cfg(not(unix))]
    {
        "-".to_string()
    }
}