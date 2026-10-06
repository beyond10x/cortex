//! systemd user units: one timer and service per source, and one viewer per instance. The units
//! run a copy of cortex at `<home>/bin/cortex`, so a rebuild changes nothing until the next install.
//! Its version is recorded beside it, in `<home>/bin/cortex.version`.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;

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
}

pub fn source_unit(instance: &str, source: &str) -> String {
    format!("cortex-{instance}-{source}")
}

pub fn view_unit(instance: &str) -> String {
    format!("cortex-{instance}-view")
}

/// `KEY=value` for systemd's `Environment=`, quoted.
fn env_line(key: &str, value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("Environment=\"{key}={escaped}\"\n")
}

/// The variables a unit needs so `connectors` and `claude` find their configuration and sign-in:
/// the user manager's `PATH` holds none of the user's tool directories, and `claude` reads its
/// sign-in under `HOME`.
fn environment() -> String {
    let mut out = String::new();
    for (key, value) in std::env::vars() {
        let wanted = matches!(
            key.as_str(),
            "PATH"
                | "HOME"
                | "XDG_CONFIG_HOME"
                | "XDG_STATE_HOME"
                | "XDG_DATA_HOME"
                | "XDG_RUNTIME_DIR"
        ) || key.starts_with("CONNECTORS_");
        if wanted {
            out.push_str(&env_line(&key, &value));
        }
    }
    out
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

    fn write(&self, name: &str, text: &str) -> Result<(), String> {
        std::fs::create_dir_all(&self.unit_dir)
            .map_err(|e| format!("{}: {e}", self.unit_dir.display()))?;
        let path = self.unit_dir.join(name);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
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
        let tools = tool_environment(tools);
        self.write(
            &format!("{unit}.service"),
            &format!(
                "[Unit]\nDescription=cortex: run {instance}/{source}\n\n[Service]\nType=oneshot\n{}{tools}ExecStart=\"{}\" --home \"{}\" run {instance}/{source} --record-failure\n",
                environment(),
                exe.display(),
                self.home_root.display()
            ),
        )?;
        self.write(
            &format!("{unit}.timer"),
            &format!(
                "[Unit]\nDescription=cortex: schedule {instance}/{source}\n\n[Timer]\nOnCalendar={schedule}\nPersistent=true\nUnit={unit}.service\n\n[Install]\nWantedBy=timers.target\n"
            ),
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
        self.write(
            &format!("{unit}.service"),
            &format!(
                "[Unit]\nDescription=cortex: viewer of {instance}\n\n[Service]\n{}{}{}ExecStart=\"{}\" view --port {port}\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n",
                env_line("EKR_HOST", &store.host.display().to_string()),
                env_line("EKR_BACKEND", store.backend.name()),
                env_line("EKR_STORE", &store.store.display().to_string()),
                store.bin.display()
            ),
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
        if !self.unit_dir.join(&service).is_file() || !self.active(&service)? {
            return Ok((stopped(), None));
        }
        self.systemctl(&["stop", &service])?;
        let value = stopped();
        Ok((value, self.systemctl(&["start", &service]).err()))
    }

    pub fn set_source_timer(&self, instance: &str, source: &str, on: bool) -> Result<(), String> {
        let timer = format!("{}.timer", source_unit(instance, source));
        if on {
            self.systemctl(&["enable", "--now", &timer])
        } else {
            self.systemctl(&["disable", "--now", &timer])
        }
    }

    /// Stops and deletes every unit of `instance`.
    pub fn remove_instance(&self, instance: &str, sources: &[String]) -> Result<(), String> {
        let mut units: Vec<String> = sources
            .iter()
            .map(|s| format!("{}.timer", source_unit(instance, s)))
            .collect();
        units.push(format!("{}.service", view_unit(instance)));
        for unit in &units {
            let _ = self.systemctl(&["disable", "--now", unit]);
        }
        for source in sources {
            let base = source_unit(instance, source);
            for ext in ["timer", "service"] {
                let _ = std::fs::remove_file(self.unit_dir.join(format!("{base}.{ext}")));
            }
        }
        let _ = std::fs::remove_file(
            self.unit_dir
                .join(format!("{}.service", view_unit(instance))),
        );
        self.systemctl(&["daemon-reload"])
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering::{Equal, Greater, Less};

    use super::compare;

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
