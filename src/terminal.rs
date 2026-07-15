use std::io::{self, BufWriter, Stdout, Write, stdout};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        DisableFocusChange, EnableFocusChange, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute, queue,
    style::ResetColor,
    terminal::{
        Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement,
    },
};

const OUTPUT_BUFFER_CAPACITY: usize = 64 * 1024;

/// Tracks whether the process may currently have terminal modes enabled.
///
/// This is intentionally process-global so the panic hook can restore the
/// terminal without borrowing the active [`TerminalSession`].
static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);
static KEYBOARD_ENHANCEMENT_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Owns the terminal modes used by the game and restores them when dropped.
pub struct TerminalSession {
    output: BufWriter<Stdout>,
    restored: bool,
    keyboard_enhancement_pushed: bool,
}

impl TerminalSession {
    /// Enters raw mode and switches to a clean alternate screen.
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);

        let mut session = Self {
            output: BufWriter::with_capacity(OUTPUT_BUFFER_CAPACITY, stdout()),
            restored: false,
            keyboard_enhancement_pushed: false,
        };

        let setup_result = execute!(
            session.output,
            EnterAlternateScreen,
            EnableFocusChange,
            Hide,
            DisableLineWrap,
            Clear(ClearType::All),
            MoveTo(0, 0)
        );

        if let Err(error) = setup_result {
            // Setup commands can partially succeed, so always attempt every
            // restoration command before returning the original error.
            let _ = session.restore();
            return Err(error);
        }

        if supports_keyboard_enhancement().unwrap_or(false) {
            let flags = KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
            // Mark it active before writing because an I/O error can occur
            // after a terminal has already consumed the command.
            session.keyboard_enhancement_pushed = true;
            KEYBOARD_ENHANCEMENT_ACTIVE.store(true, Ordering::SeqCst);
            if let Err(error) = execute!(session.output, PushKeyboardEnhancementFlags(flags)) {
                let _ = session.restore();
                return Err(error);
            }
        }

        Ok(session)
    }

    /// Whether this backend will reliably emit key-release events.
    #[must_use]
    pub const fn reliable_key_releases(&self) -> bool {
        cfg!(windows) || self.keyboard_enhancement_pushed
    }

    /// Writes one complete, prebuilt frame and flushes it once.
    ///
    /// The cursor is moved to the origin before the frame, so callers do not
    /// need to include a leading cursor command in their frame buffer.
    pub fn write_frame(&mut self, frame: &[u8]) -> io::Result<()> {
        queue!(self.output, MoveTo(0, 0))?;
        self.output.write_all(frame)?;
        self.output.flush()
    }

    /// Clears the alternate screen and returns the cursor to its origin.
    pub fn clear(&mut self) -> io::Result<()> {
        execute!(self.output, Clear(ClearType::All), MoveTo(0, 0))
    }

    /// Restores the user's terminal, attempting all cleanup operations even
    /// when an earlier operation fails. Calling this more than once is safe.
    pub fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }

        // The panic hook may already have restored the process-global terminal
        // state before unwinding reaches this guard.
        if !TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
            self.restored = true;
            return Ok(());
        }

        let mut first_error = None;
        if self.keyboard_enhancement_pushed {
            remember_first_error(
                &mut first_error,
                execute!(self.output, PopKeyboardEnhancementFlags),
            );
            self.keyboard_enhancement_pushed = false;
            KEYBOARD_ENHANCEMENT_ACTIVE.store(false, Ordering::SeqCst);
        }
        remember_first_error(&mut first_error, execute!(self.output, DisableFocusChange));
        remember_first_error(&mut first_error, execute!(self.output, ResetColor));
        remember_first_error(&mut first_error, execute!(self.output, Show));
        remember_first_error(&mut first_error, execute!(self.output, EnableLineWrap));
        remember_first_error(
            &mut first_error,
            execute!(self.output, LeaveAlternateScreen),
        );
        remember_first_error(&mut first_error, self.output.flush());
        remember_first_error(&mut first_error, disable_raw_mode());

        self.restored = true;
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// Restores terminal modes without requiring access to a [`TerminalSession`].
///
/// This is intended for a panic hook. Errors are deliberately ignored because
/// panic cleanup must be best-effort and must never cause a second panic.
pub fn restore_terminal_best_effort() {
    if !TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }

    let mut output = BufWriter::with_capacity(OUTPUT_BUFFER_CAPACITY, stdout());
    if KEYBOARD_ENHANCEMENT_ACTIVE.swap(false, Ordering::SeqCst) {
        let _ = execute!(output, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(output, DisableFocusChange);
    let _ = execute!(output, ResetColor);
    let _ = execute!(output, Show);
    let _ = execute!(output, EnableLineWrap);
    let _ = execute!(output, LeaveAlternateScreen);
    let _ = output.flush();
    let _ = disable_raw_mode();
}

fn remember_first_error(first_error: &mut Option<io::Error>, result: io::Result<()>) {
    if let Err(error) = result
        && first_error.is_none()
    {
        *first_error = Some(error);
    }
}
