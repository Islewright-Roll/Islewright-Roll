// SPDX-License-Identifier: GPL-3.0-or-later
//! What a dialog needs of the window that Day has no call for. The Java app's dialogs were windows
//! of their own: they dimmed the navigation bar too, and the system's back closed them first.
//! Here they are drawn in the one window, so on Android both are asked for by hand.

/// The window's background behind the bars, the theme's `islewright_wood`.
#[cfg(target_os = "android")]
const WOOD: u32 = 0xFF22180D;
/// The same under a dialog's dim of black at 0.6.
#[cfg(target_os = "android")]
const WOOD_DIMMED: u32 = 0xFF0E0A05;

/// How often the back button is looked for while something waits for it, in milliseconds.
#[cfg(target_os = "android")]
const BACK_POLL_MS: u32 = 50;

/// Dims what shows behind the root, or stops, and while dimmed sends the system's back to `back`
/// instead of letting it leave the app.
#[cfg(target_os = "android")]
pub fn dim(dimmed: bool, back: impl Fn() + 'static) {
    use std::cell::Cell;

    thread_local! {
        static WATCH: Cell<u64> = const { Cell::new(0) };
    }
    let watch = WATCH.with(|w| {
        w.set(w.get() + 1);
        w.get()
    });
    window_native((if dimmed { WOOD_DIMMED } else { WOOD }) as i32, dimmed);
    if !dimmed {
        return;
    }
    day::task(async move {
        while WATCH.with(|w| w.get()) == watch {
            day::sleep(BACK_POLL_MS).await;
            if WATCH.with(|w| w.get()) == watch && back_taken_native().unwrap_or(false) {
                back();
            }
        }
    });
}

/// Elsewhere nothing shows behind the root, and Escape is the key press it is.
#[cfg(not(target_os = "android"))]
pub fn dim(_dimmed: bool, _back: impl Fn() + 'static) {}

#[cfg(target_os = "android")]
day_bridge::bridge! {
    #[day_bridge::declare]
    extern "day" {
        fn window_native(color: i32, catch_back: bool);
        fn back_taken_native() -> Result<bool, day_bridge::Error>;
    }

    #[day_bridge::impl(java, platforms = [android])]
    java!(
        prelude = r#"
            import androidx.activity.ComponentActivity;
            import androidx.activity.OnBackPressedCallback;
            import dev.daybrite.day.bridge.DayRuntime;
        "#,
        body = r#"
            private static OnBackPressedCallback catcher;
            private static volatile boolean taken;

            public static void window_native(int color, boolean catchBack) {
                final ComponentActivity activity = (ComponentActivity) DayRuntime.ctx;
                activity.getWindow().getDecorView().setBackgroundColor(color);
                if (catcher != null) {
                    catcher.remove();
                    catcher = null;
                }
                taken = false;
                if (catchBack) {
                    catcher = new OnBackPressedCallback(true) {
                        @Override public void handleOnBackPressed() {
                            taken = true;
                        }
                    };
                    activity.getOnBackPressedDispatcher().addCallback(catcher);
                }
            }

            public static boolean back_taken_native() {
                boolean was = taken;
                taken = false;
                return was;
            }
        "#,
    );

    // Android is the only target compiling this module; the arm satisfies the rule that a bridge
    // always has an answer for an unclaimed target.
    #[day_bridge::impl(rust, platforms = [other])]
    fn window_native(_color: i32, _catch_back: bool) {}

    #[day_bridge::impl(rust, platforms = [other])]
    fn back_taken_native() -> Result<bool, day_bridge::Error> {
        Err(day_bridge::Error::Unsupported)
    }
}
