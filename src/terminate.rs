//! SIGTERM for `cortex run` (`cortex.instance.RunSource`, outcome `terminated`): a Kubernetes
//! Job's deadline or a `systemctl stop` sends it. Once [`install`] ran, the first SIGTERM only
//! sets a flag, which the run reads at its safe points ([`requested`]): before it uses what it
//! fetched, before each batch, and while it waits for a model call, which it then ends. The run
//! answers `terminated`, records its status and the metric file and exits 8. A second SIGTERM ends
//! the process at once with [`EXIT_SECOND_SIGNAL`], recording nothing.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

/// The exit status of a process a second SIGTERM ended: the shell's 128 + 15.
pub const EXIT_SECOND_SIGNAL: i32 = 143;

fn flag() -> &'static Arc<AtomicBool> {
    static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    FLAG.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

/// Handles SIGTERM from now on: the first sets the flag [`requested`] reads, a second exits.
pub fn install() -> Result<(), String> {
    use signal_hook::consts::SIGTERM;
    // Registered first, so it reads the flag as it was before this signal set it.
    signal_hook::flag::register_conditional_shutdown(SIGTERM, EXIT_SECOND_SIGNAL, flag().clone())
        .map_err(|e| format!("cannot handle SIGTERM: {e}"))?;
    signal_hook::flag::register(SIGTERM, flag().clone())
        .map_err(|e| format!("cannot handle SIGTERM: {e}"))?;
    Ok(())
}

/// Whether the process was sent SIGTERM since [`install`].
pub fn requested() -> bool {
    flag().load(Ordering::SeqCst)
}
