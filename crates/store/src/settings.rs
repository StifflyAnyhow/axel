//! Switches the owner sets on this machine, kept local (never in the shared
//! log): each one is off until the owner turns it on.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store};

/// A machine-wide switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Setting {
    /// Route new managed sessions through the token inspector's proxy (ADR 0011).
    Inspector,
}

impl Setting {
    pub const ALL: [Setting; 1] = [Setting::Inspector];

    /// The row's key.
    pub fn key(self) -> &'static str {
        match self {
            Setting::Inspector => "inspector",
        }
    }
}

/// How a switch is stored.
const ON: &str = "on";
const OFF: &str = "off";

impl Store {
    /// Whether `setting` is on.  Never set reads as off.
    pub fn setting_on(&self, setting: Setting) -> Result<bool, Error> {
        let value: Option<String> = self
            .conn
            .query_row("SELECT value FROM local_settings WHERE key = ?1", [setting.key()], |r| r.get(0))
            .optional()?;
        // Anything but "on" (a value a later build wrote, say) is off: a
        // switch that guards credentials fails closed.
        Ok(value.as_deref() == Some(ON))
    }

    /// Turn `setting` on or off.
    pub fn set_setting(&self, setting: Setting, on: bool) -> Result<(), Error> {
        let value = if on { ON } else { OFF };
        self.conn.execute(
            "INSERT INTO local_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![setting.key(), value],
        )?;
        debug_assert_eq!(self.setting_on(setting).ok(), Some(on));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_is_off_until_turned_on_and_off_again_after() {
        let store = Store::open_memory().unwrap();
        assert!(!store.setting_on(Setting::Inspector).unwrap());
        store.set_setting(Setting::Inspector, true).unwrap();
        assert!(store.setting_on(Setting::Inspector).unwrap());
        store.set_setting(Setting::Inspector, false).unwrap();
        assert!(!store.setting_on(Setting::Inspector).unwrap());
    }

    #[test]
    fn an_unknown_value_reads_as_off() {
        let store = Store::open_memory().unwrap();
        store.conn.execute("INSERT INTO local_settings VALUES ('inspector', 'yes')", []).unwrap();
        assert!(!store.setting_on(Setting::Inspector).unwrap());
    }
}
