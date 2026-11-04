// SPDX-License-Identifier: GPL-3.0-or-later
//! What a dialog needs of the window that Day has no call for. The Java app's dialogs were windows
//! of their own: they dimmed the navigation bar too, and the system's back closed them first.
//! Here they are drawn in the one window, so on Android both are asked for by hand. So is the
//! scroll thumb's colour, which the theme can only tie to the system's dark mode, not the app's.

/// The window's background behind the bars, the theme's `islewright_wood`.
#[cfg(target_os = "android")]
const WOOD: u32 = 0xFF22180D;
/// The same under a dialog's dim of black at 0.6.
#[cfg(target_os = "android")]
const WOOD_DIMMED: u32 = 0xFF0E0A05;

/// The scroll thumb in the light palette and the dark, the theme's `islewright_thumb`.
#[cfg(target_os = "android")]
const THUMB: u32 = 0xFF5A3C14;
#[cfg(target_os = "android")]
const THUMB_DARK: u32 = 0xFFB8A47C;

/// A colour under a dialog's dim of black at 0.6, opaque.
#[cfg(target_os = "android")]
fn under_dim((r, g, b): (u8, u8, u8)) -> u32 {
    let d = |c: u8| (c as u32 * 2 + 2) / 5;
    0xFF00_0000 | d(r) << 16 | d(g) << 8 | d(b)
}

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
    // A page's cover paints its own colour behind the bars, so it is dimmed with the window
    let surround = super::page::surround_rgb();
    let cover = if dimmed {
        under_dim(surround)
    } else {
        0xFF00_0000 | (surround.0 as u32) << 16 | (surround.1 as u32) << 8 | surround.2 as u32
    };
    window_native(
        (if dimmed { WOOD_DIMMED } else { WOOD }) as i32,
        dimmed,
        cover as i32,
    );
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

/// Colours every scroll view's thumb for the app's palette, now and as they appear.
#[cfg(target_os = "android")]
pub fn scroll_thumb(dark: bool) {
    thumb_native((if dark { THUMB_DARK } else { THUMB }) as i32);
}

/// Elsewhere the platform's own thumb stands.
#[cfg(not(target_os = "android"))]
pub fn scroll_thumb(_dark: bool) {}

#[cfg(target_os = "android")]
day_bridge::bridge! {
    #[day_bridge::declare]
    extern "day" {
        fn window_native(color: i32, catch_back: bool, cover: i32);
        fn thumb_native(color: i32);
        fn back_taken_native() -> Result<bool, day_bridge::Error>;
    }

    #[day_bridge::impl(java, platforms = [android])]
    java!(
        prelude = r#"
            import android.graphics.drawable.ColorDrawable;
            import android.os.Build;
            import android.view.View;
            import android.view.ViewGroup;
            import android.widget.ScrollView;
            import androidx.activity.ComponentActivity;
            import androidx.activity.OnBackPressedCallback;
            import dev.daybrite.day.bridge.DayCover;
            import dev.daybrite.day.bridge.DayRuntime;
        "#,
        body = r#"
            private static OnBackPressedCallback catcher;
            private static volatile boolean taken;

            private static int thumb;
            private static boolean watching;

            /** Calls `each` on `v` and every view under it. */
            private static void walk(View v, java.util.function.Consumer<View> each) {
                each.accept(v);
                if (v instanceof ViewGroup) {
                    ViewGroup g = (ViewGroup) v;
                    for (int i = 0; i < g.getChildCount(); i++) {
                        walk(g.getChildAt(i), each);
                    }
                }
            }

            public static void window_native(int color, boolean catchBack, int cover) {
                final ComponentActivity activity = (ComponentActivity) DayRuntime.ctx;
                View decor = activity.getWindow().getDecorView();
                decor.setBackgroundColor(color);
                // A full cover (a page) is opaque behind the navigation bar; a sheet has its
                // own rounded shape, not a flat colour, and keeps it
                walk(decor, v -> {
                    if (v instanceof DayCover && v.getBackground() instanceof ColorDrawable) {
                        v.setBackgroundColor(cover);
                    }
                });
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

            /** The thumb drawable is API 29's; before it the theme's colour stands. */
            public static void thumb_native(int color) {
                if (Build.VERSION.SDK_INT < 29) {
                    return;
                }
                thumb = color;
                final View decor = ((ComponentActivity) DayRuntime.ctx).getWindow().getDecorView();
                // A page builds its scroll view after this is called, so every layout looks
                if (!watching) {
                    watching = true;
                    decor.getViewTreeObserver().addOnGlobalLayoutListener(() -> recolor(decor));
                }
                recolor(decor);
            }

            private static void recolor(View decor) {
                walk(decor, v -> {
                    if (v instanceof ScrollView && !Integer.valueOf(thumb).equals(v.getTag(THUMB_TAG))) {
                        v.setTag(THUMB_TAG, thumb);
                        ((ScrollView) v).setVerticalScrollbarThumbDrawable(new ColorDrawable(thumb));
                    }
                });
            }

            /** A view tag key of the app's own, holding the thumb colour a scroll view was given. */
            private static final int THUMB_TAG = 0x7f1a5e01;

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
    fn window_native(_color: i32, _catch_back: bool, _cover: i32) {}

    #[day_bridge::impl(rust, platforms = [other])]
    fn thumb_native(_color: i32) {}

    #[day_bridge::impl(rust, platforms = [other])]
    fn back_taken_native() -> Result<bool, day_bridge::Error> {
        Err(day_bridge::Error::Unsupported)
    }
}
