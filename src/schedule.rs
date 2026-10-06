//! systemd user units: one timer and service per source, and one viewer per instance. The units
//! run a copy of cortex at `<home>/bin/cortex`, so a rebuild changes nothing until the next install.
//! Its version is recorded beside it, in `<home>/bin/cortex.version`.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;

use cortex_model::instance as m;
use serde_json::{json, Value};

/// This cortex's version, recorded beside `<home>/bin/cortex` when it is placed there.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `<home>/bin/cortex`, the copy of cortex every source unit of the home runs.
pub fn binary_path(home_root: &Path) -> PathBuf {
    home_root.join("bin/cortex")
}

fn version_path(home_root: &Path) -> PathBuf {
    home_root.join("bin/cortex.version")
}

/// The version recorded for `<home>/bin/cortex`; `None` before a cortex that records it placed
/// one.
pub fn binary_version(home_root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(version_path(home_root)).ok()?;
    Some(text.trim().to_string()).filter(|v| !v.is_empty())
}

/// `major.minor.patch[-pre][+build]`, ordered as SemVer orders them, the pre-release compared as
/// text. `None` when either is not of that form.
pub fn compare(a: &str, b: &str) -> Option<Ordering> {
    fn parse(v: &str) -> Option<([u64; 3], Option<&str>)> {
        let v = v.split('+').next()?;
        let (core, pre) = match v.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (v, None),
        };
        let mut parts = core.split('.').map(|n| n.parse::<u64>().ok());
        let triple = [parts.next()??, parts.next()??, parts.next()??];
        parts.next().is_none().then_some((triple, pre))
    }
    let ((a, a_pre), (b, b_pre)) = (parse(a)?, parse(b)?);
    Some(a.cmp(&b).then(match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => x.cmp(y),
    }))
}

/// What placing this cortex at `<home>/bin/cortex` did: the version recorded before (`old`), this
/// cortex's (`new`), whether it replaced the binary, and why not when it did not.
#[derive(Debug)]
pub struct Placed {
    pub old: Option<String>,
    pub new: String,
    pub replaced: bool,
    pub reason: Option<String>,
}

impl Placed {
    pub fn json(&self) -> Value {
        let mut out = json!({"replaced": self.replaced, "old": self.old, "new": self.new});
        if let Some(reason) = &self.reason {
            out["reason"] = json!(reason);
        }
        out
    }
}

pub struct Systemd {
    /// `systemctl`, or a stand-in (`CORTEX_SYSTEMCTL`).
    pub systemctl: PathBuf,
    /// `$XDG_CONFIG_HOME/systemd/user`, or `CORTEX_UNIT_DIR`.
    pub unit_dir: PathBuf,
    pub home_root: PathBuf,
    /// Each unit this value wrote over a unit another home had orphaned, as `<unit> (home
    /// <that home>)` ([`Owner::Orphaned`]).
    pub taken_over: RefCell<Vec<String>>,
}

pub fn source_unit(instance: &str, source: &str) -> String {
    format!("cortex-{instance}-{source}")
}

pub fn view_unit(instance: &str) -> String {
    format!("cortex-{instance}-view")
}

/// The `[Unit]` key a unit names the home that wrote it with. systemd ignores a key that starts
/// with `X-`.
const HOME_KEY: &str = "X-CortexHome=";

/// The home `text`, a unit file, was written for: its `X-CortexHome=` line, or for a unit an
/// earlier cortex wrote, the `--home` of a source service's `ExecStart` or the home of a viewer's
/// `EKR_HOST` (`<home>/instances/<name>/host.json`). `None` for a unit that names neither.
fn home_of(text: &str) -> Option<PathBuf> {
    if let Some(home) = text.lines().find_map(|l| l.strip_prefix(HOME_KEY)) {
        return Some(PathBuf::from(home));
    }
    text.lines().find_map(|line| {
        if let Some(exec) = line.strip_prefix("ExecStart=") {
            let (_, after) = exec.split_once(" --home \"")?;
            return after.split_once('"').map(|(home, _)| PathBuf::from(home));
        }
        let host = line
            .strip_prefix("Environment=\"EKR_HOST=")?
            .strip_suffix('"')?
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");
        Path::new(&host).ancestors().nth(3).map(Path::to_path_buf)
    })
}

/// Whether `a` and `b` name one home.
fn same_home(a: &Path, b: &Path) -> bool {
    a == b
        || matches!(
            (std::fs::canonicalize(a), std::fs::canonicalize(b)),
            (Ok(a), Ok(b)) if a == b
        )
}

/// Why every command refuses the home `root`: its path holds a line break. Every unit the home
/// writes records the home on one line (`X-CortexHome=`, the service's `--home`), and systemd reads
/// a unit line by line, so the home would no longer know its own units from another home's.
pub fn home_refusal(root: &Path) -> Option<String> {
    root.to_string_lossy().contains(['\n', '\r']).then(|| {
        format!(
            "the cortex home {:?} holds a line break; each systemd unit cortex writes records its \
             home on one line, and a line break would cut it there. Use a home path without one",
            root.display().to_string()
        )
    })
}

/// Refuses a unit whose `values` (what each is, and its text) hold a line break: systemd reads a
/// unit line by line, so the rest of the value would be read as a line of its own. The refusal
/// names what holds it, not the text, which may be a credential (`CONNECTORS_*`).
fn single_line<W: std::fmt::Display>(
    values: impl IntoIterator<Item = (W, String)>,
) -> Result<(), String> {
    for (what, value) in values {
        if value.contains(['\n', '\r']) {
            return Err(format!(
                "{what} holds a line break, and a systemd unit holds one value per \
                 line; cortex writes no unit with it"
            ));
        }
    }
    Ok(())
}

/// Whose a unit file another home wrote is.
enum Owner {
    /// That home holds an active instance of the name: the unit is its own.
    Live(PathBuf),
    /// That home's registry cannot be read (why), so it may hold the instance.
    Unreadable(PathBuf, String),
    /// That home does not exist, or holds no active instance of the name. No command of it
    /// reaches the unit again, and the home that writes it next takes it over.
    Orphaned(PathBuf),
}

impl Owner {
    fn home(&self) -> &Path {
        match self {
            Owner::Live(home) | Owner::Unreadable(home, _) | Owner::Orphaned(home) => home,
        }
    }
}

/// `KEY=value` for systemd's `Environment=`, quoted: systemd unquotes `\\` and `\"` there and
/// expands `%` specifiers, so those are escaped.
fn env_line(key: &str, value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    format!("Environment=\"{key}={escaped}\"\n")
}

/// One `ExecStart=` word, double-quoted: systemd unquotes `\\` and `\"`, and expands `%` specifiers
/// and `$` variables, so those are escaped too.
fn exec_quote(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('$', "$$");
    format!("\"{escaped}\"")
}

/// `bin` as an `ExecStart=` program: systemd does not search the unit's `PATH`, so a bare name is
/// looked up on this process's `PATH`, and a relative path is made absolute. A bare name found
/// nowhere is written as it is.
fn on_path(bin: &Path) -> PathBuf {
    if bin.components().count() == 1 && bin.is_relative() {
        std::env::var_os("PATH")
            .iter()
            .flat_map(std::env::split_paths)
            .map(|dir| dir.join(bin))
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| bin.to_path_buf())
    } else {
        std::path::absolute(bin).unwrap_or_else(|_| bin.to_path_buf())
    }
}

/// The variables a unit needs so `connectors` and `claude` find their configuration and sign-in:
/// the user manager's `PATH` holds none of the user's tool directories, and `claude` reads its
/// sign-in under `HOME`.
fn environment() -> String {
    environment_values()
        .map(|(key, value)| env_line(&key, &value))
        .collect()
}

/// The variables [`environment`] writes, by name.
fn environment_values() -> impl Iterator<Item = (String, String)> {
    std::env::vars().filter_map(|(key, value)| {
        let wanted = matches!(
            key.as_str(),
            "PATH"
                | "HOME"
                | "XDG_CONFIG_HOME"
                | "XDG_STATE_HOME"
                | "XDG_DATA_HOME"
                | "XDG_RUNTIME_DIR"
        ) || key.starts_with("CONNECTORS_");
        wanted.then_some((key, value))
    })
}

/// `CORTEX_CONNECTORS`, `CORTEX_CLAUDE` and `CORTEX_CODEX` for a source unit, so a scheduled run
/// uses the binaries this process runs. A bare name is written as it is and found on the unit's
/// `PATH`, the one this process has; a path is made absolute.
fn tool_environment(tools: &crate::run::Tools) -> String {
    let named = |bin: &Path| {
        if bin.components().count() == 1 && bin.is_relative() {
            bin.to_path_buf()
        } else {
            std::path::absolute(bin).unwrap_or_else(|_| bin.to_path_buf())
        }
    };
    [
        ("CORTEX_CONNECTORS", &tools.connectors.bin),
        ("CORTEX_CLAUDE", &tools.claude),
        ("CORTEX_CODEX", &tools.codex),
    ]
    .into_iter()
    .map(|(key, bin)| env_line(key, &named(bin).display().to_string()))
    .collect()
}

impl Systemd {
    pub fn from_env(home_root: &Path) -> Self {
        let systemctl = std::env::var_os("CORTEX_SYSTEMCTL")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("systemctl"));
        let unit_dir = std::env::var_os("CORTEX_UNIT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let config = std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
                    });
                config.join("systemd/user")
            });
        Self {
            systemctl,
            unit_dir,
            home_root: home_root.to_path_buf(),
            taken_over: RefCell::default(),
        }
    }

    fn systemctl(&self, args: &[&str]) -> Result<(), String> {
        let status = Command::new(&self.systemctl)
            .arg("--user")
            .args(args)
            .status()
            .map_err(|e| format!("cannot run {}: {e}", self.systemctl.display()))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("systemctl --user {} failed", args.join(" ")))
        }
    }

    /// Whether `unit` is running: `systemctl --user is-active --quiet` exits 0.
    fn active(&self, unit: &str) -> Result<bool, String> {
        Command::new(&self.systemctl)
            .args(["--user", "is-active", "--quiet", unit])
            .status()
            .map(|status| status.success())
            .map_err(|e| format!("cannot run {}: {e}", self.systemctl.display()))
    }

    /// Who the unit file `name` of `instance` belongs to, when it exists and another home wrote it.
    /// A timer that names no home belongs to its service's.
    fn foreign(&self, instance: &str, name: &str) -> Option<Owner> {
        let read = |name: &str| std::fs::read_to_string(self.unit_dir.join(name)).ok();
        let home = home_of(&read(name)?).or_else(|| {
            let service = name.strip_suffix(".timer")?;
            home_of(&read(&format!("{service}.service"))?)
        })?;
        if same_home(&home, &self.home_root) {
            return None;
        }
        // A home that does not exist reads as an empty registry.
        Some(match crate::home::Home::new(home.clone()).load_registry() {
            Ok(registry)
                if registry
                    .instances
                    .get(instance)
                    .is_some_and(|i| i.state == m::InstanceState::Active) =>
            {
                Owner::Live(home)
            }
            Ok(_) => Owner::Orphaned(home),
            Err(e) => Owner::Unreadable(home, e),
        })
    }

    /// Why the unit file `name` of `instance`, which `owner` wrote, is not written, started or
    /// stopped by this home, with the step that clears it: `install` for `create`, `adopt` and
    /// `update`, which can pass `--no-units`, and otherwise for a command that only signals it.
    fn refusal(&self, instance: &str, name: &str, owner: &Owner, install: bool) -> String {
        let path = self.unit_dir.join(name);
        let (path, home) = (path.display(), owner.home().display());
        let other = "cortex changes no unit of another home";
        match (owner, install) {
            (Owner::Live(_), true) => format!(
                "{path} belongs to the cortex home {home}, which has an instance of the same \
                 name; {other}. Remove the instance there, or pass --no-units"
            ),
            (Owner::Live(_), false) => format!(
                "{path} belongs to the cortex home {home}, which has an instance of the same \
                 name; {other}. Remove the instance there, then `update` {instance} here to \
                 install this home's units"
            ),
            (Owner::Unreadable(_, e), true) => format!(
                "{path} belongs to the cortex home {home}, whose registry cannot be read ({e}), \
                 so it may hold an instance of the same name; {other}. Repair that registry, or \
                 pass --no-units"
            ),
            (Owner::Unreadable(_, e), false) => format!(
                "{path} belongs to the cortex home {home}, whose registry cannot be read ({e}), \
                 so it may hold an instance of the same name; {other}. Repair that registry"
            ),
            (Owner::Orphaned(_), _) => format!(
                "{path} belongs to the cortex home {home}, which holds no instance {instance}; \
                 `update` {instance} here takes its units over"
            ),
        }
    }

    /// Why the units of `instance`'s `sources` (and its viewer, when `view`) cannot be written:
    /// another home that holds the instance, or may hold it, wrote one of them. A unit whose home
    /// holds no such instance is not a reason; writing it takes it over.
    pub fn foreign_units(&self, instance: &str, sources: &[String], view: bool) -> Option<String> {
        let mut names: Vec<String> = sources
            .iter()
            .flat_map(|s| {
                let unit = source_unit(instance, s);
                [format!("{unit}.service"), format!("{unit}.timer")]
            })
            .collect();
        if view {
            names.push(format!("{}.service", view_unit(instance)));
        }
        names.iter().find_map(|n| match self.foreign(instance, n)? {
            Owner::Orphaned(_) => None,
            owner => Some(self.refusal(instance, n, &owner, true)),
        })
    }

    /// Writes the unit files `units` (name, text) of `instance`, each text with this home recorded
    /// under its `[Unit]` line. When another home that holds the instance (or may) wrote one of
    /// them, none is written and the answer names that home; one an orphaning home wrote is taken
    /// over and named in [`Systemd::taken_over`]. Every owner is read before any unit is written:
    /// a timer that names no home is its service's, and the service is written first.
    fn write(&self, instance: &str, units: &[(String, String)]) -> Result<(), String> {
        let mut taken = Vec::new();
        for (name, _) in units {
            taken.push(match self.foreign(instance, name) {
                Some(Owner::Orphaned(home)) => Some(format!("{name} (home {})", home.display())),
                Some(owner) => return Err(self.refusal(instance, name, &owner, true)),
                None => None,
            });
        }
        std::fs::create_dir_all(&self.unit_dir)
            .map_err(|e| format!("{}: {e}", self.unit_dir.display()))?;
        for ((name, text), taken) in units.iter().zip(taken) {
            let text = text.replacen(
                "[Unit]\n",
                &format!("[Unit]\n{HOME_KEY}{}\n", self.home_root.display()),
                1,
            );
            let path = self.unit_dir.join(name);
            std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
            self.taken_over.borrow_mut().extend(taken);
        }
        Ok(())
    }

    /// Places this cortex at `<home>/bin/cortex`, which every source unit of the home runs, and
    /// records its version beside it ([`binary_version`]). A binary recorded as newer than this
    /// cortex is kept unless `replace_newer`: replacing it would move every timer of the home to
    /// an older cortex. The same or an older version, or none recorded, is replaced.
    pub fn place_binary(&self, replace_newer: bool) -> Result<Placed, String> {
        let exe = std::env::current_exe().map_err(|e| format!("cannot find cortex itself: {e}"))?;
        let dest = binary_path(&self.home_root);
        let old = binary_version(&self.home_root);
        let mut placed = Placed {
            old: old.clone(),
            new: VERSION.to_string(),
            replaced: false,
            reason: None,
        };
        if exe == dest {
            placed.reason = Some(format!("this cortex is {}", dest.display()));
            return Ok(placed);
        }
        let newer = old
            .as_deref()
            .and_then(|old| compare(old, VERSION))
            .is_some_and(|o| o == Ordering::Greater);
        if newer && dest.exists() && !replace_newer {
            placed.reason = Some(format!(
                "{} is cortex {}, newer than this cortex {VERSION}, and every timer of the home \
                 runs it; it is kept. Pass --replace-binary to replace it",
                dest.display(),
                old.unwrap_or_default()
            ));
            return Ok(placed);
        }
        std::fs::create_dir_all(dest.parent().expect("bin dir")).map_err(|e| e.to_string())?;
        let tmp = dest.with_extension("new");
        std::fs::copy(&exe, &tmp).map_err(|e| format!("cannot copy cortex: {e}"))?;
        std::fs::rename(&tmp, &dest).map_err(|e| format!("cannot place cortex: {e}"))?;
        crate::home::write_atomic(
            &version_path(&self.home_root),
            format!("{VERSION}\n").as_bytes(),
        )
        .map_err(|e| format!("cannot record the version of cortex: {e}"))?;
        placed.replaced = true;
        Ok(placed)
    }

    /// The source's service runs the `tools` this process runs ([`tool_environment`]), and
    /// `<home>/bin/cortex`, which [`Systemd::place_binary`] places.
    pub fn install_source(
        &self,
        instance: &str,
        source: &str,
        schedule: &str,
        tools: &crate::run::Tools,
    ) -> Result<(), String> {
        let unit = source_unit(instance, source);
        let exe = binary_path(&self.home_root);
        single_line(
            [
                ("the schedule", schedule.to_string()),
                ("the cortex home", self.home_root.display().to_string()),
                (
                    "the connectors binary",
                    tools.connectors.bin.display().to_string(),
                ),
                ("the claude binary", tools.claude.display().to_string()),
                ("the codex binary", tools.codex.display().to_string()),
            ]
            .into_iter()
            .map(|(what, value)| (what.to_string(), value))
            .chain(
                environment_values()
                    .map(|(key, value)| (format!("{key} in the environment"), value)),
            ),
        )?;
        let tools = tool_environment(tools);
        self.write(
            instance,
            &[
                (
                    format!("{unit}.service"),
                    format!(
                        "[Unit]\nDescription=cortex: run {instance}/{source}\n\n[Service]\nType=oneshot\n{}{tools}ExecStart={} --home {} run {instance}/{source} --record-failure\n",
                        environment(),
                        exec_quote(&exe.display().to_string()),
                        exec_quote(&self.home_root.display().to_string())
                    ),
                ),
                (
                    format!("{unit}.timer"),
                    format!(
                        "[Unit]\nDescription=cortex: schedule {instance}/{source}\n\n[Timer]\nOnCalendar={schedule}\nPersistent=true\nUnit={unit}.service\n\n[Install]\nWantedBy=timers.target\n"
                    ),
                ),
            ],
        )?;
        self.systemctl(&["daemon-reload"])?;
        self.systemctl(&["enable", "--now", &format!("{unit}.timer")])
    }

    pub fn install_view(
        &self,
        instance: &str,
        store: &crate::ekr::Store,
        port: u16,
    ) -> Result<(), String> {
        let unit = view_unit(instance);
        single_line([
            ("EKR_HOST", store.host.display().to_string()),
            ("EKR_STORE", store.store.display().to_string()),
            ("the ekr binary", store.bin.display().to_string()),
            ("the cortex home", self.home_root.display().to_string()),
        ])?;
        let store_env = [
            env_line("EKR_HOST", &store.host.display().to_string()),
            env_line("EKR_BACKEND", store.backend.name()),
            env_line("EKR_STORE", &store.store.display().to_string()),
        ]
        .concat();
        // A launched store's viewer is `ekr view` started by `connectors connections launch`,
        // which needs the variables a source unit gives `connectors`.
        let (env, exec) = match &store.launch {
            None => (
                store_env,
                format!(
                    "{} view --port {port}",
                    exec_quote(&store.bin.display().to_string())
                ),
            ),
            Some(launch) => {
                let connectors = on_path(&launch.connectors);
                single_line(
                    [
                        ("the connectors binary", connectors.display().to_string()),
                        ("the adapter", launch.connection.adapter.clone()),
                        ("the connection", launch.connection.connection.clone()),
                    ]
                    .into_iter()
                    .map(|(what, value)| (what.to_string(), value))
                    .chain(
                        environment_values()
                            .map(|(key, value)| (format!("{key} in the environment"), value)),
                    ),
                )?;
                let args = serde_json::json!(["view", "--port", port.to_string()]).to_string();
                (
                    format!(
                        "{}{}{store_env}",
                        environment(),
                        env_line("CORTEX_CONNECTORS", &connectors.display().to_string())
                    ),
                    format!(
                        "{} connections launch --adapter {} --connection {} --consumer {} --args {}",
                        exec_quote(&connectors.display().to_string()),
                        exec_quote(&launch.connection.adapter),
                        exec_quote(&launch.connection.connection),
                        crate::ekr::CONSUMER,
                        exec_quote(&args)
                    ),
                )
            }
        };
        self.write(
            instance,
            &[(
                format!("{unit}.service"),
                format!(
                    "[Unit]\nDescription=cortex: viewer of {instance}\n\n[Service]\n{env}ExecStart={exec}\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n",
                ),
            )],
        )?;
        self.systemctl(&["daemon-reload"])?;
        self.systemctl(&["enable", "--now", &format!("{unit}.service")])
    }

    /// Stops the viewer of `instance` when it is running, runs `stopped`, and starts the viewer
    /// again whatever `stopped` answered. A viewer that was not running (the operator stopped it)
    /// is left stopped, and an instance with no viewer unit (created with `--no-units`, or
    /// removed) only runs `stopped`. Answers its value and why the viewer did not start again,
    /// when it did not; a viewer that cannot be stopped runs nothing.
    pub fn restart_view<T>(
        &self,
        instance: &str,
        stopped: impl FnOnce() -> T,
    ) -> Result<(T, Option<String>), String> {
        let service = format!("{}.service", view_unit(instance));
        // Another home's viewer of an instance of the same name is not this instance's.
        if !self.unit_dir.join(&service).is_file()
            || self.foreign(instance, &service).is_some()
            || !self.active(&service)?
        {
            return Ok((stopped(), None));
        }
        self.systemctl(&["stop", &service])?;
        let value = stopped();
        Ok((value, self.systemctl(&["start", &service]).err()))
    }

    pub fn set_source_timer(&self, instance: &str, source: &str, on: bool) -> Result<(), String> {
        let timer = format!("{}.timer", source_unit(instance, source));
        if let Some(owner) = self.foreign(instance, &timer) {
            return Err(self.refusal(instance, &timer, &owner, false));
        }
        if on {
            self.systemctl(&["enable", "--now", &timer])
        } else {
            self.systemctl(&["disable", "--now", &timer])
        }
    }

    /// Stops and deletes every unit of `instance` this home wrote. A unit of the same name another
    /// home wrote is left running and in place; the answer names each, with its home.
    pub fn remove_instance(
        &self,
        instance: &str,
        sources: &[String],
    ) -> Result<Vec<String>, String> {
        let mut files: Vec<String> = sources
            .iter()
            .flat_map(|s| {
                let unit = source_unit(instance, s);
                [format!("{unit}.timer"), format!("{unit}.service")]
            })
            .collect();
        let view = format!("{}.service", view_unit(instance));
        files.push(view.clone());
        let (kept, own): (Vec<_>, Vec<_>) = files
            .into_iter()
            .map(|f| (self.foreign(instance, &f), f))
            .partition(|(home, _)| home.is_some());
        // The timers and the viewer are what is enabled; a source's service only runs from its
        // timer.
        for (_, file) in &own {
            if file.ends_with(".timer") || *file == view {
                let _ = self.systemctl(&["disable", "--now", file]);
            }
        }
        for (_, file) in &own {
            let _ = std::fs::remove_file(self.unit_dir.join(file));
        }
        self.systemctl(&["daemon-reload"])?;
        Ok(kept
            .into_iter()
            .map(|(owner, file)| match owner {
                Some(owner) => format!("{file} (home {})", owner.home().display()),
                None => file,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering::{Equal, Greater, Less};

    use super::{compare, exec_quote, Systemd};

    /// Every value cortex writes into a unit is read back by systemd as written: `ExecStart=` words
    /// are unquoted with `\\` and `\"` and expand `%` and `$`; `Environment=` values are unquoted
    /// the same way and expand `%`.
    #[test]
    fn a_source_unit_quotes_its_exec_start_words_and_its_environment_values() {
        let tmp = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
        let home = tmp.path().join("h o\"m%e$X");
        let units = tmp.path().join("units");
        let systemd = Systemd {
            systemctl: std::path::PathBuf::from("true"),
            unit_dir: units.clone(),
            home_root: home.clone(),
            taken_over: Default::default(),
        };
        let tools = crate::run::Tools {
            connectors: crate::connectors::Connectors {
                bin: tmp.path().join("c%h/connectors"),
            },
            claude: "claude".into(),
            codex: "codex".into(),
        };
        systemd
            .install_source("q", "news", "daily", &tools)
            .unwrap();
        let unit = std::fs::read_to_string(units.join("cortex-q-news.service")).unwrap();
        let exec = format!(
            "ExecStart={} --home {} run q/news --record-failure\n",
            exec_quote(&home.join("bin/cortex").display().to_string()),
            exec_quote(&home.display().to_string())
        );
        assert!(unit.contains(&exec), "{exec} in\n{unit}");
        let connectors = format!(
            "Environment=\"CORTEX_CONNECTORS={}/c%%h/connectors\"\n",
            tmp.path().display()
        );
        assert!(unit.contains(&connectors), "{connectors} in\n{unit}");
    }

    #[test]
    fn versions_are_ordered_as_semver_orders_them() {
        assert_eq!(compare("0.2.0", "0.1.9"), Some(Greater));
        assert_eq!(compare("0.10.0", "0.9.0"), Some(Greater));
        assert_eq!(compare("1.0.0", "1.0.0"), Some(Equal));
        assert_eq!(compare("1.0.0+build.7", "1.0.0"), Some(Equal));
        assert_eq!(compare("1.0.0-rc.1", "1.0.0"), Some(Less));
        assert_eq!(compare("1.0.0-rc.2", "1.0.0-rc.1"), Some(Greater));
        assert_eq!(compare("0.1", "0.1.0"), None);
        assert_eq!(compare("dev", "0.1.0"), None);
    }
}
