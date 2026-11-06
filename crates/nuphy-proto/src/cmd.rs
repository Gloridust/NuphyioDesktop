//! Command ids and event ids.
//!
//! Two protocol families exist (see [`crate::device::Family`]). Values are
//! taken from the enums in the NuPhyIO web app. Constants marked *(sim)* were
//! only observed in the app's built-in simulator, not on real hardware.

/// Mechanical (non-Hall) keyboards, e.g. Node75, Air75 V3, Halo75 Lite.
pub mod mech {
    pub const GET_DONGLE_NAME: u8 = 0x2F;
    pub const GET_BASE: u8 = 0xA0;
    pub const GET_FIRMWARE_INFO: u8 = 0xA1;
    pub const GET_DEFAULT_KEYS: u8 = 0xB1;
    pub const GET_USE_KEYS: u8 = 0xB2;
    pub const SET_USE_KEYS: u8 = 0xB3;
    pub const RESTORE_USE_LAYERS: u8 = 0xB4;
    pub const GET_SOCD: u8 = 0xB5;
    pub const SET_SOCD: u8 = 0xB6;
    pub const RESET_SOCD: u8 = 0xB7;
    pub const GET_TAP_DANCE: u8 = 0xB8;
    pub const SET_TAP_DANCE: u8 = 0xB9;
    pub const RESET_TAP_DANCE: u8 = 0xBA;
    pub const GET_TGL: u8 = 0xBB;
    pub const SET_TGL: u8 = 0xBC;
    pub const RESET_TGL: u8 = 0xBD;
    pub const SET_KEY_UPLOAD: u8 = 0xC1;
    pub const GET_MACRO: u8 = 0xC2;
    pub const SET_MACRO: u8 = 0xC3;
    pub const GET_LIGHT_COUNT: u8 = 0xD1;
    pub const GET_KEY_LIGHT_COLOR: u8 = 0xD2;
    pub const GET_LIGHT_STATE: u8 = 0xD5;
    pub const SET_LIGHT_STATE: u8 = 0xD6;
    pub const SET_SIGNAL_COLOR: u8 = 0xD8;
    pub const GET_KEYBOARD_FUNC: u8 = 0xE1;
    pub const SET_KEYBOARD_FUNC: u8 = 0xE2;
    pub const SET_DEBOUNCE_TIME: u8 = 0xE3;
    pub const TEST_DELAY_TIME: u8 = 0xE4;
    pub const SET_TOUCH_BAR_CONFIG: u8 = 0xE5;
    pub const GET_TOUCH_BAR_CONFIG: u8 = 0xE6;
    pub const SET_SECRET_KEY: u8 = 0xEE;
    /// Reboot into the IAP bootloader (firmware update). Never send casually.
    pub const SET_IAP_MODE: u8 = 0xEF;
    pub const RESTORE_FACTORY: u8 = 0xF1;
    pub const GET_SLEEP_INFO: u8 = 0xF3;
    pub const SET_SLEEP_CFG: u8 = 0xF5;
    pub const GET_APP_DEFINE_SIZE: u8 = 0xFA;
    pub const GET_APP_DEFINE: u8 = 0xFB;
    pub const SET_APP_DEFINE: u8 = 0xFC;
    pub const GET_DEBUG_ENABLE: u8 = 0xFD;
    pub const SET_DEBUG_LOG: u8 = 0xFE;

    /// Commands that only read state. The CLI's raw reader accepts only these.
    pub const READ_ONLY: &[u8] = &[
        GET_DONGLE_NAME,
        GET_BASE,
        GET_FIRMWARE_INFO,
        GET_DEFAULT_KEYS,
        GET_USE_KEYS,
        GET_SOCD,
        GET_TAP_DANCE,
        GET_TGL,
        GET_MACRO,
        GET_LIGHT_COUNT,
        GET_KEY_LIGHT_COLOR,
        GET_LIGHT_STATE,
        GET_KEYBOARD_FUNC,
        GET_TOUCH_BAR_CONFIG,
        GET_SLEEP_INFO,
        GET_APP_DEFINE_SIZE,
        GET_APP_DEFINE,
        GET_DEBUG_ENABLE,
    ];

    /// Unsolicited input reports (first byte is *not* `0xAA`).
    pub mod event {
        pub const MODE_STATE_CHANGE: u8 = 0xA2;
        pub const KEY_UPLOAD: u8 = 0xC1;
        pub const LIGHT_STATE_CHANGE: u8 = 0xD7;
        pub const KEYBOARD_RESET: u8 = 0xF2;
        pub const SLEEP_CFG_CHANGE: u8 = 0xF4;
        pub const LOG_UPLOAD: u8 = 0xFE;
    }
}

/// Hall-effect (HE) keyboards. Frame codec only; see docs for caveats.
pub mod hall {
    pub const QUICK_COMM_START: u8 = 1;
    pub const QUICK_COMM_END: u8 = 2;
    pub const GET_INFO: u8 = 3;
    pub const GET_BASE: u8 = 4;
    pub const GET_FUNC: u8 = 5;
    pub const SET_FUNC: u8 = 6;
    pub const GET_DEFAULT_KEY_MATRIX: u8 = 7;
    pub const GET_USE_KEY_MATRIX: u8 = 8;
    pub const SET_USE_KEY_MATRIX: u8 = 9;
    pub const GET_LED_DEFINE: u8 = 10;
    pub const SET_LED_DEFINE: u8 = 11;
    pub const GET_MACRO: u8 = 12;
    pub const SET_MACRO: u8 = 13;
    pub const GET_GAMEPAD: u8 = 0x10;
    pub const SET_GAMEPAD: u8 = 0x11;
    pub const GET_KEY_RT_INFO: u8 = 0xA0;
    pub const SET_KEY_RT_INFO: u8 = 0xA1;
    pub const GET_DKS_INFO: u8 = 0xA2;
    pub const SET_DKS_INFO: u8 = 0xA3;
    pub const GET_MT_KEY_INFO: u8 = 0xA4;
    pub const SET_MT_KEY_INFO: u8 = 0xA5;
    pub const GET_TGL_KEY_INFO: u8 = 0xA6;
    pub const SET_TGL_KEY_INFO: u8 = 0xA7;
    pub const START_CALIBRATION: u8 = 0xA8;
    pub const END_CALIBRATION: u8 = 0xA9;
    pub const GET_CALIBRATION_INFO: u8 = 0xAA;
    pub const LED_SYNC_DOWNLOAD: u8 = 0xDD;
    pub const LED_SYNC_UPLOAD: u8 = 0xDE;
    pub const RESTORE_FACTORY_SETTINGS: u8 = 0xEE;
    pub const GET_APP_DEFINE: u8 = 0xF1;
    pub const SET_APP_DEFINE: u8 = 0xF2;

    /// Unsolicited input reports (first byte is *not* `0xAA`). *(sim)*
    pub mod event {
        pub const CALIBRATION_STATUS: u8 = 0xA0;
        pub const STATE_CHANGE: u8 = 0xA1;
        pub const KEYBOARD_RESET: u8 = 0xA2;
        pub const KEYBOARD_FUNC_CFG: u8 = 0xA3;
        pub const DONGLE_CONNECTION: u8 = 0xFA;
    }
}
