//! Pandoc export jobs. No shell, no note writes, and no overwrite of destinations.
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Format {
    #[default]
    Pdf,
    Docx,
    Html,
    Epub,
    Latex,
}

impl Format {
    pub(crate) const ALL: [Self; 5] = [Self::Pdf, Self::Docx, Self::Html, Self::Epub, Self::Latex];

    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Html => "html",
            Self::Epub => "epub",
            Self::Latex => "tex",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::Docx => "DOCX",
            Self::Html => "HTML (embedded images)",
            Self::Epub => "EPUB",
            Self::Latex => "LaTeX",
        }
    }

    pub(crate) fn cycle(self, forward: bool) -> Self {
        let index = Self::ALL.iter().position(|&item| item == self).unwrap_or(0);
        Self::ALL[(index + if forward { 1 } else { Self::ALL.len() - 1 }) % Self::ALL.len()]
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Settings {
    pub format: Format,
    pub output_dir: Option<PathBuf>,
    pub csl: Option<PathBuf>,
    pub resource_dirs: Vec<PathBuf>,
    pub pdf_engine: Option<String>,
}

pub(crate) struct Job {
    pub source: PathBuf,
    pub directory: bool,
    pub destination: PathBuf,
    pub format: Format,
    pub root: PathBuf,
    pub bibliography: Option<PathBuf>,
    pub csl: Option<PathBuf>,
    pub resource_dirs: Vec<PathBuf>,
    pub pdf_engine: Option<String>,
    pub snapshots: HashMap<PathBuf, String>,
}

pub(crate) enum Update {
    Progress(String),
    Finished(Result<ExportResult, String>),
}

#[derive(Debug)]
pub(crate) struct ExportResult {
    pub destination: PathBuf,
    pub count: usize,
    pub warnings: String,
}

// Check executability without launching an external process or blocking the UI.
pub(crate) fn executable_exists(program: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let executable = |path: &Path| {
        fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    let path = Path::new(program);
    if program.is_empty() {
        return false;
    }
    if path.is_absolute() {
        return executable(path);
    }
    if path.components().count() != 1 || program.contains('/') {
        return false;
    }
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| executable(&dir.join(path))))
}

pub(crate) const PDF_ENGINES: [&str; 6] = [
    "typst",
    "xelatex",
    "lualatex",
    "pdflatex",
    "tectonic",
    "wkhtmltopdf",
];

pub(crate) fn engine_choices(configured: Option<&str>) -> Vec<String> {
    let mut choices = vec![String::new()]; // empty = automatic
    if let Some(engine) = configured.filter(|e| !e.is_empty() && *e != "auto") {
        choices.push(engine.to_owned());
    }
    for engine in PDF_ENGINES {
        if executable_exists(engine) && !choices.iter().any(|choice| choice.as_str() == engine) {
            choices.push(engine.to_owned());
        }
    }
    choices
}

pub(crate) fn select_pdf_engine(requested: Option<&str>) -> io::Result<String> {
    if let Some(engine) = requested.filter(|e| !e.is_empty() && *e != "auto") {
        return if executable_exists(engine) {
            Ok(engine.to_owned())
        } else {
            Err(invalid(format!(
                "PDF engine '{engine}' not found. Select Auto or install it."
            )))
        };
    }
    PDF_ENGINES
        .into_iter()
        .find(|engine| executable_exists(engine))
        .map(str::to_owned)
        .ok_or_else(|| invalid("No PDF engine found. Install Typst or a TeX engine and try again."))
}

pub(crate) fn preflight(job: &Job) -> io::Result<()> {
    if !executable_exists("pandoc") {
        return Err(invalid("Pandoc not found. Install pandoc and try again."));
    }
    if job.format == Format::Pdf {
        select_pdf_engine(job.pdf_engine.as_deref())?;
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> io::Error {
    let message: String = message.into();
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub(crate) fn resolve_path(value: &str, base: &Path) -> io::Result<PathBuf> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(invalid("Enter a nonempty path without control characters."));
    }
    let path = Path::new(value);
    if value == "~" || value.starts_with("~/") {
        let home = std::env::var_os("HOME").ok_or_else(|| invalid("HOME is not set."))?;
        return Ok(PathBuf::from(home).join(value.strip_prefix("~/").unwrap_or("")));
    }
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    })
}

pub(crate) fn suggested_path(
    source: &Path,
    directory: bool,
    format: Format,
    settings: &Settings,
) -> PathBuf {
    let base = settings
        .output_dir
        .as_deref()
        .unwrap_or_else(|| source.parent().unwrap_or(source));
    if directory {
        let name = source
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("notes"))
            .to_string_lossy();
        base.join(format!("{name}-export-{}", format.extension()))
    } else {
        base.join(source.file_name().unwrap_or(source.as_os_str()))
            .with_extension(format.extension())
    }
}

fn collect_notes(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_notes(&entry.path(), paths)?;
        } else if kind.is_file() && crate::markdown::is_markdown(&entry.path()) {
            paths.push(entry.path());
        }
    }
    Ok(())
}

fn plan(job: &Job) -> io::Result<Vec<(PathBuf, PathBuf)>> {
    if !job.directory {
        if !job
            .destination
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case(job.format.extension()))
        {
            return Err(invalid(format!(
                "Destination must end in .{}",
                job.format.extension()
            )));
        }
        return Ok(vec![(
            job.source.clone(),
            PathBuf::from(format!("document.{}", job.format.extension())),
        )]);
    }
    let mut paths = Vec::new();
    collect_notes(&job.source, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err(invalid("No Markdown notes in this directory."));
    }
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .map(|source| {
            let relative = source
                .strip_prefix(&job.source)
                .map_err(io::Error::other)?
                .with_extension(job.format.extension());
            if !seen.insert(relative.clone()) {
                return Err(invalid(format!(
                    "Output filename collision: {}",
                    relative.display()
                )));
            }
            Ok((source, relative))
        })
        .collect()
}

fn resource_paths(job: &Job, parent: &Path) -> Vec<PathBuf> {
    let mut resources = vec![parent.to_path_buf()];
    if parent.starts_with(&job.root) {
        for ancestor in parent.ancestors().take_while(|p| p.starts_with(&job.root)) {
            resources.push(ancestor.to_path_buf());
            resources.push(ancestor.join("assets"));
        }
    }
    resources.extend([job.root.clone(), job.root.join("assets")]);
    resources.extend(job.resource_dirs.iter().cloned());
    let mut seen = HashSet::new();
    resources.retain(|path| seen.insert(path.clone()));
    resources
}

fn pandoc_command(job: &Job, source: &Path, output: &Path) -> io::Result<Command> {
    let parent = source
        .parent()
        .ok_or_else(|| invalid("Missing note directory."))?;
    let resources = resource_paths(job, parent);
    let search = std::env::join_paths(resources).map_err(io::Error::other)?;
    let mut command = Command::new("pandoc");
    command
        .current_dir(parent)
        .args(["--from=markdown", "--standalone", "--resource-path"])
        .arg(search);
    match job.format {
        Format::Pdf => {
            if let Some(engine) = &job.pdf_engine {
                command.arg(format!("--pdf-engine={engine}"));
            }
        }
        Format::Docx => {
            command.arg("--to=docx");
        }
        Format::Html => {
            command.args(["--to=html5", "--embed-resources"]);
        }
        Format::Epub => {
            command.arg("--to=epub3");
        }
        Format::Latex => {
            command.arg("--to=latex");
        }
    }
    if let Some(bibliography) = &job.bibliography {
        command
            .args(["--citeproc", "--bibliography"])
            .arg(bibliography);
        if let Some(csl) = &job.csl {
            command.arg("--csl").arg(csl);
        }
    }
    command.arg("--output").arg(output).arg("-");
    Ok(command)
}

fn convert(job: &Job, source: &Path, output: &Path) -> io::Result<String> {
    let mut input = tempfile::tempfile()?;
    if let Some(text) = job.snapshots.get(source) {
        input.write_all(text.as_bytes())?;
    } else {
        let mut file = fs::File::open(source)?;
        io::copy(&mut file, &mut input)?;
    }
    input.seek(SeekFrom::Start(0))?;
    let mut errors = tempfile::tempfile()?;
    let mut child = pandoc_command(job, source, output)?
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::from(errors.try_clone()?))
        .spawn()
        .map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    err.kind(),
                    "Pandoc not found. Install pandoc and try again.",
                )
            } else {
                err
            }
        })?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() >= Duration::from_secs(120) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Pandoc timed out (120 seconds per note).",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    errors.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    errors.take(8192).read_to_end(&mut bytes)?;
    let detail = String::from_utf8_lossy(&bytes).trim().to_owned();
    if !status.success() {
        return Err(io::Error::other(format!("Pandoc {status}: {detail}")));
    }
    if !output.is_file() {
        return Err(io::Error::other("Pandoc did not create an output file."));
    }
    Ok(detail)
}

pub(crate) fn run(mut job: Job, updates: &Sender<Update>) -> io::Result<ExportResult> {
    let name = job
        .destination
        .file_name()
        .ok_or_else(|| invalid("Invalid destination name."))?;
    let parent = job
        .destination
        .parent()
        .ok_or_else(|| invalid("Missing destination directory."))?
        .canonicalize()?;
    let destination = parent.join(name);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Destination exists. Choose another name.",
        ));
    }
    if job.directory && destination.starts_with(&job.source) {
        return Err(invalid(
            "Choose a destination outside the source directory.",
        ));
    }
    let plan = plan(&job)?;
    preflight(&job)?;
    if job.format == Format::Pdf {
        job.pdf_engine = Some(select_pdf_engine(job.pdf_engine.as_deref())?);
    }
    let count = plan.len();
    // Stage on the destination filesystem and publish only after every conversion succeeds.
    let staging = tempfile::Builder::new()
        .prefix(".zettui-export-")
        .tempdir_in(&parent)?;
    let payload = staging.path().join("documents");
    fs::create_dir(&payload)?;
    let mut warning_count = 0;
    let mut warnings = String::new();
    for (index, (source, relative)) in plan.iter().enumerate() {
        let _ = updates.send(Update::Progress(format!(
            "Exporting {}/{}: {}",
            index + 1,
            count,
            source.display()
        )));
        let output = payload.join(relative);
        fs::create_dir_all(
            output
                .parent()
                .ok_or_else(|| invalid("Invalid output path."))?,
        )?;
        let detail = convert(&job, source, &output)
            .map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", source.display())))?;
        if !detail.is_empty() {
            warning_count += 1;
            // Bound the diagnostic buffer for large batch exports.
            if warnings.len() < 32_768 {
                warnings.push_str(&format!("\n{}:\n{detail}", source.display()));
            }
        }
    }
    let staged = if job.directory {
        payload
    } else {
        payload.join(&plan[0].1)
    };
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    renameat_with(CWD, &staged, CWD, &destination, RenameFlags::NOREPLACE)
        .map_err(io::Error::from)?;
    if warning_count > 0 {
        warnings = format!("Pandoc diagnostics in {warning_count} document(s):{warnings}");
        if warnings.len() >= 32_768 {
            warnings.push_str("\nAdditional diagnostics may be omitted.");
        }
    }
    Ok(ExportResult {
        destination,
        count,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(directory: bool) -> (tempfile::TempDir, Job) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("notes");
        fs::create_dir(&root).unwrap();
        let note = root.join("Note.md");
        fs::write(&note, "Disk content").unwrap();
        let job = Job {
            source: if directory { root.clone() } else { note },
            directory,
            destination: temp
                .path()
                .join(if directory { "export" } else { "Note.html" }),
            format: Format::Html,
            root,
            bibliography: None,
            csl: None,
            resource_dirs: Vec::new(),
            pdf_engine: None,
            snapshots: HashMap::new(),
        };
        (temp, job)
    }

    #[test]
    fn export_defaults_to_pdf() {
        assert_eq!(Settings::default().format, Format::Pdf);
        assert!(engine_choices(Some("auto")).first().unwrap().is_empty());
        assert!(select_pdf_engine(Some("zettui-no-such-pdf-engine-987654321")).is_err());
    }

    #[test]
    fn export_resource_paths_include_nested_assets_without_duplicates() {
        let (_temp, job) = fixture(false);
        let parent = job.root.join("section/nested");
        let paths = resource_paths(&job, &parent);
        assert_eq!(paths.first(), Some(&parent));
        assert!(paths.contains(&job.root.join("section/assets")));
        assert!(paths.contains(&job.root.join("assets")));
        assert_eq!(paths.iter().collect::<HashSet<_>>().len(), paths.len());
        assert!(!paths.contains(&job.root.parent().unwrap().join("assets")));
    }

    #[test]
    fn export_directory_plan_preserves_structure_and_skips_hidden_and_symlinks() {
        let (_temp, job) = fixture(true);
        fs::create_dir(job.source.join("nested")).unwrap();
        fs::write(job.source.join("nested/B.md"), "b").unwrap();
        fs::write(job.source.join(".secret.md"), "secret").unwrap();
        std::os::unix::fs::symlink(job.source.join("Note.md"), job.source.join("link.md")).unwrap();
        let paths: Vec<_> = plan(&job)
            .unwrap()
            .into_iter()
            .map(|(_, relative)| relative)
            .collect();
        assert_eq!(
            paths,
            [PathBuf::from("Note.html"), PathBuf::from("nested/B.html")]
        );
        fs::write(job.source.join("Note.markdown"), "duplicate").unwrap();
        assert!(plan(&job).is_err());
    }

    #[test]
    fn export_refuses_existing_target_without_launching_pandoc() {
        let (_temp, job) = fixture(false);
        fs::write(&job.destination, "existing").unwrap();
        let destination = job.destination.clone();
        let (tx, _rx) = std::sync::mpsc::channel();
        assert_eq!(
            run(job, &tx).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read_to_string(destination).unwrap(), "existing");
    }

    #[test]
    fn export_passes_paths_as_literal_arguments() {
        let (_temp, mut job) = fixture(false);
        job.bibliography = Some(job.root.join("My refs.json"));
        job.csl = Some(job.root.join("My style.csl"));
        let command = pandoc_command(&job, &job.source, &job.destination).unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--citeproc".to_owned()));
        assert!(args.contains(&job.bibliography.unwrap().to_string_lossy().into_owned()));
        assert!(args.contains(&job.csl.unwrap().to_string_lossy().into_owned()));
        assert_eq!(args.last().unwrap(), "-");
    }

    #[test]
    fn export_html_live_buffer_when_pandoc_is_available() {
        if !Command::new("pandoc")
            .arg("--version")
            .output()
            .is_ok_and(|result| result.status.success())
        {
            return;
        }
        let (_temp, mut job) = fixture(false);
        job.snapshots
            .insert(job.source.clone(), "# Unsaved title\n\nLive buffer.".into());
        let destination = job.destination.clone();
        let source = job.source.clone();
        let (tx, _rx) = std::sync::mpsc::channel();
        run(job, &tx).unwrap();
        let html = fs::read_to_string(destination).unwrap();
        assert!(html.contains("Live buffer."));
        assert_eq!(fs::read_to_string(source).unwrap(), "Disk content");
    }
}
