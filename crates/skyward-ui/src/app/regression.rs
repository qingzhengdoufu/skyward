use super::*;
use crate::color::over;
use crate::fmt;
use skyward_info::{Boot, Charge, Cpu, Storage};
use skyward_install::{Failure, Refusal};
use skyward_net::reach::Reach;
use skyward_vab::SnapshotState;

const W: u32 = 1440;
const H: u32 = 3200;
const PPD: f32 = 3.5;

#[cfg(feature = "pinyin-input")]
#[test]
fn native_text_fields_share_chinese_composition_without_changing_english_keys() {
    for screen in [
        Screen::Search,
        Screen::Files,
        Screen::Rename,
        Screen::WlanPassword,
        Screen::WlanAdd,
        Screen::Decrypt,
        Screen::BrowserForm,
    ] {
        let mut app = App::new(screen, W, H, PPD);
        app.state.files_search = true;
        app.state.data = Data::Locked {
            user: 0,
            keys: Keys::Letters,
            checking: false,
        };
        app.state.adding = Some(Adding {
            name: String::new(),
            security: Security::Open,
            field: Field::Name,
            choosing: false,
        });
        app.state.browser.field = Some((0, 0));
        app.state.browser.form = vec![vec![String::new()]];
        assert!(!app.state.ime.enabled);
        app.act(Action::ImeToggle, 0);
        for ch in "nihao".chars() {
            app.act(Action::Type(Cap::Char(ch)), 1);
        }
        assert_eq!(app.typing(), "", "{screen:?}: no raw pinyin in field");
        app.act(Action::Type(Cap::Backspace), 2);
        assert_eq!(app.state.ime.spelling, "niha");
        app.act(Action::Type(Cap::Char('o')), 3);
        let index = app
            .state
            .ime
            .candidates
            .iter()
            .position(|candidate| candidate.text == "你好")
            .unwrap();
        let effects = app.act(Action::ImePick(index), 4);
        assert_eq!(app.typing(), "你好", "{screen:?}");
        if screen == Screen::Files {
            assert!(matches!(effects.as_slice(), [Effect::FindFiles { query, .. }] if query == "你好"));
        }
        app.act(Action::ImeToggle, 5);
        app.act(Action::Type(Cap::Char('x')), 6);
        assert_eq!(app.typing(), "你好x");
        app.act(Action::Type(Cap::Backspace), 7);
        app.act(Action::Type(Cap::Backspace), 8);
        assert_eq!(app.typing(), "你", "{screen:?}: backspace removes a Unicode character");
    }
}

#[cfg(feature = "pinyin-input")]
#[test]
fn chinese_terminal_and_editor_commit_without_running_or_breaking_the_line() {
    for screen in [Screen::Terminal, Screen::Editor] {
        let mut app = App::new(screen, W, H, PPD);
        app.term_started(24, 80);
        app.state.editor = Some(Editor::new("/tmp/example.txt".into(), "", 40, 10));
        app.act(Action::ImeToggle, 0);
        for ch in "nihao".chars() {
            assert!(app.act(Action::Type(Cap::Char(ch)), 1).is_empty());
        }
        let effects = app.act(Action::Type(Cap::Enter), 2);
        if screen == Screen::Terminal {
            assert_eq!(effects, [Effect::TermInput("你好".as_bytes().to_vec())]);
            assert_eq!(app.act(Action::Type(Cap::Enter), 3), [Effect::TermInput(vec![13])]);
            app.term.as_mut().unwrap().ctrl = true;
            assert_eq!(app.act(Action::Type(Cap::Char('c')), 4), [Effect::TermInput(vec![3])]);
        } else {
            assert!(effects.is_empty());
            assert_eq!(app.state.editor.as_ref().unwrap().text(), "你好");
            app.act(Action::Type(Cap::Enter), 3);
            assert_eq!(app.state.editor.as_ref().unwrap().text(), "你好\n");
        }
        app.state.ime.spelling = "secret".into();
        app.open(Screen::Search);
        assert!(app.state.ime.spelling.is_empty() && !app.state.ime.enabled);
    }
}

#[cfg(feature = "pinyin-input")]
#[test]
fn native_keyboard_candidates_fit_above_the_keys_on_each_text_page() {
    let (fonts, info) = (fonts(), info());
    for (width, height, density) in [(720, 1600, 1.875), (W, H, PPD)] {
        for screen in [
            Screen::Search,
            Screen::Files,
            Screen::Rename,
            Screen::WlanPassword,
            Screen::WlanAdd,
            Screen::Terminal,
            Screen::Editor,
            Screen::Decrypt,
        ] {
            let mut app = App::new(screen, width, height, density);
            app.state.files_search = true;
            app.state.joining = Some(network("WiFi", -45, Security::Psk));
            app.state.adding = Some(Adding {
                name: String::new(),
                security: Security::Open,
                field: Field::Name,
                choosing: false,
            });
            app.state.editor = Some(Editor::new("/tmp/example.txt".into(), "", 40, 10));
            app.state.data = Data::Locked {
                user: 0,
                keys: Keys::Letters,
                checking: false,
            };
            let frame = app.frame(&fonts, &info);
            let (hits, bottom) = if let Some(dock) = &frame.dock {
                (&dock.hits, dock.canvas.h as f32)
            } else {
                (&frame.hits, frame.canvas.h as f32)
            };
            let toggle = hits
                .iter()
                .find(|hit| hit.action == Action::ImeToggle)
                .unwrap_or_else(|| panic!("{screen:?} missing language toggle"))
                .rect;
            let letter = hits.iter().find(|hit| hit.action == Action::Type(Cap::Char('q'))).unwrap().rect;
            assert!(
                toggle.y + toggle.h <= letter.y + 1.0 && toggle.y >= 0.0 && toggle.y + toggle.h <= bottom,
                "{screen:?}"
            );
        }
    }
}

#[test]
fn image_restore_lists_and_confirms_the_named_original_partition_without_writing() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Restore, W, H, PPD);
    app.state.data = Data::Open;
    for name in ["boot_a", "super"] {
        let entry = BackupEntry {
            place: Place::Storage,
            name: format!("20261119-170607-{name}"),
            held: Ok(Held {
                made: 1795079167,
                size: 4096,
                parts: [false; 4],
                device: String::new(),
                system: None,
                slot: None,
            }),
            image: Some(ImageCopy {
                partition: name.into(),
                sha256: "ab".repeat(32),
            }),
        };
        app.set_backups(vec![entry.clone()]);
        app.act(Action::OpenBackup(0), 0);
        assert!(
            app.frame(&f, &i)
                .hits
                .iter()
                .any(|hit| hit.action == Action::Confirm(Confirm::Restore))
        );
        assert_eq!(
            app.confirmed(Confirm::Restore, 1),
            [Effect::RestoreImage {
                place: Place::Storage,
                folder: entry.name.clone(),
                partition: name.into(),
                size: 4096,
                sha256: "ab".repeat(32)
            }]
        );
        app.state.data = Data::Closed;
        assert!(app.confirmed(Confirm::Restore, 2).is_empty());
        app.state.data = Data::Open;
        app.key(Key::Back, 3);
    }
}

#[test]
fn script_diagnostic_warnings_do_not_change_a_success_or_failure_outcome() {
    use skyward_install::warnings::Warning;
    let mut app = App::new(Screen::Installing, W, H, PPD);
    app.install(Event::Running, 0);
    for _ in 0..100 {
        app.install(Event::Warning(Warning::MissingCommand), 1);
    }
    assert_eq!(app.state.script_warnings, [Warning::MissingCommand]);
    let outcome = Outcome::Ran { wipe_cache: false };
    app.install(Event::Finished(outcome.clone()), 2);
    assert!(matches!(&app.state.install, Install::Finished { outcome: actual, .. } if *actual == outcome));
    app.begin_install(Install::Starting, Source::Computer, None);
    assert!(app.state.script_warnings.is_empty());
    app.install(Event::Warning(Warning::Io), 3);
    let outcome = Outcome::Failed(Failure::Script(1));
    app.install(Event::Finished(outcome.clone()), 4);
    assert!(matches!(&app.state.install, Install::Finished { outcome: actual, .. } if *actual == outcome));
}

#[test]
fn browser_canvas_is_fixed_and_keyboard_does_not_reposition_the_web_page() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.set_navigation(true);
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    let theme = app.theme();
    let area = crate::browser::full_viewport(&theme);
    assert!(
        area.y < widgets::bar_bottom(&theme),
        "web content starts below status, not the unused page toolbar"
    );
    let pixels = vec![220; area.w as usize * area.h as usize * 4];
    app.state.browser.picture = Some(BrowserPicture {
        width: area.w as u32,
        height: area.h as u32,
        pixels: std::sync::Arc::new(pixels),
    });
    let mut before = Canvas::new(W, H);
    app.compose(&mut before, &f, &i, "12:00", 1000);
    assert_eq!(app.max_scroll(), 0.0, "the dock does not add scrollable page height");
    let point = (area.w / 2.0, area.y + 200.0);
    app.touch(Touch::Down(point.0, point.1), &f, 1100);
    app.touch(Touch::Move(point.0, point.1 - 50.0), &f, 1110);
    app.touch(Touch::Up, &f, 1120);
    assert_eq!(app.top().1, 0.0);
    app.browser_plugin_focus(true);
    app.end_turn();
    let mut after = Canvas::new(W, H);
    app.compose(&mut after, &f, &i, "12:00", 2000);
    let upper = crate::browser::full_input_viewport(&theme);
    let first = (area.y as usize + 10) * W as usize * 4;
    let last = (upper.y as usize + upper.h as usize - 10) * W as usize * 4;
    assert_eq!(
        &before.data[first..last],
        &after.data[first..last],
        "keyboard overlays, not moves or rescales, the web canvas"
    );
    assert_eq!(app.max_scroll(), 0.0);
}

#[test]
fn native_web_viewport_keeps_top_and_bottom_content_clear_of_browser_chrome() {
    for (w, h, ppd) in [(720, 1600, 1.875), (1440, 3200, 3.5), (1440, 3200, 3.75)] {
        let mut app = App::new(Screen::Browser, w, h, ppd);
        app.set_navigation(true);
        app.state.browser.full_active = true;
        let t = app.theme();
        let area = crate::browser::full_viewport(&t);
        assert_eq!(area.w, w as f32);
        assert!(area.y >= t.m.dp(48.0));
        assert!(area.y + area.h <= h as f32 - t.bottom_inset - t.m.dp(122.0));
        assert_eq!(area.w % 4.0, 0.0);
        assert_eq!(area.h % 4.0, 0.0);
    }
}

#[test]
fn full_browser_explains_why_starting_requires_decrypted_data() {
    let mut app = App::new(Screen::BrowserFull, W, H, PPD);
    assert!(app.act(Action::BrowserPluginStart, 0).is_empty());
    assert!(matches!(app.state.browser.full, BrowserFull::Failed(ref why) if why.contains("data")));
    assert_eq!(app.screen(), Screen::BrowserFull);
}

#[test]
fn browser_long_address_is_ellipsized_before_the_menu() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.url = format!("https://example.com/{}", "long-query-".repeat(100));
    let t = app.theme();
    let dock = app.frame(&f, &i).dock.as_ref().unwrap();
    let bar = dock.hits.iter().find(|h| h.action == Action::BrowserAddress).unwrap().rect;
    let menu = dock.hits.iter().find(|h| h.action == Action::BrowserMenu).unwrap().rect;
    let gap = dock.canvas.crop(
        (bar.x + bar.w + 2.0) as u32,
        bar.y as u32,
        (menu.x - bar.x - bar.w - 4.0) as u32,
        bar.h as u32,
    );
    assert!(
        gap.data
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [t.pal.bg.r, t.pal.bg.g, t.pal.bg.b])
    );
}

#[cfg(feature = "pinyin-input")]
#[test]
fn browser_pinyin_candidates_commit_at_the_cursor_and_enter_submits_chinese() {
    let mut app = App::new(Screen::BrowserAddress, W, H, PPD);
    app.act(Action::BrowserImeToggle, 0);
    for ch in "nihao".chars() {
        app.type_key(Cap::Char(ch), 0);
    }
    assert!(app.state.browser.address.is_empty());
    let index = app.state.browser.ime.candidates.iter().position(|c| c.text == "你好").unwrap();
    app.act(Action::BrowserImePick(index), 0);
    assert_eq!(app.state.browser.address, "你好");
    assert_eq!(app.state.browser.address_cursor, "你好".len());
    assert!(app.state.browser.ime.spelling.is_empty());
    let effects = app.type_key(Cap::Enter, 1);
    assert!(matches!(effects.as_slice(), [Effect::BrowserOpen { url, .. }] if url.contains("%E4%BD%A0") || url.contains("%e4%bd%a0")));
}

#[test]
fn compact_pattern_grid_has_nonoverlapping_touch_targets_and_unchanged_credentials() {
    let (f, i) = (fonts(), info());
    for (w, h, ppd) in [(720, 1600, 1.875), (W, H, PPD)] {
        let mut app = App::new(Screen::Decrypt, w, h, ppd);
        app.data_waiting(Some((0, Keys::Pattern)), Vec::new());
        let (cx, cy, span) = app.frame(&f, &i).pattern.unwrap();
        assert!(span <= ppd * 220.0);
        assert!(span / 2.0 > ppd * widgets::PATTERN_REACH_DP * 2.0);
        let dots = widgets::pattern_dots(cx, cy, span);
        for dot in [0, 1, 4, 8] {
            if dot == 0 {
                app.touch(Touch::Down(dots[dot].0, dots[dot].1), &f, 1000);
            } else {
                app.touch(Touch::Move(dots[dot].0, dots[dot].1), &f, 1010);
            }
        }
        assert_eq!(
            app.touch(Touch::Up, &f, 1100),
            vec![Effect::Unlock {
                user: 0,
                code: "1259".into()
            }]
        );
    }
}

#[test]
fn control_center_draws_live_right_hand_status_marks_without_an_extra_clock() {
    let f = fonts();
    let i = DeviceInfo {
        slot: Some(Slot::A),
        battery: Some(battery_at(100, Charge::Charging)),
        ..info()
    };
    let mut app = App::new(Screen::ControlCenter, W, H, PPD);
    let t = app.theme();
    let mut actual = Canvas::new(W, H);
    app.compose(&mut actual, &f, &i, "12:00", 1000);
    let mut expected = Canvas::new(W, H);
    widgets::status_marks(&mut expected, &t, &f, &app.bar("12:00", &i), t.pal.status);
    let mut checked = 0;
    for (mark, pixel) in expected.data.as_chunks::<4>().0.iter().zip(actual.data.as_chunks::<4>().0) {
        if mark[3] == 255 {
            assert_eq!(&mark[..3], &pixel[..3]);
            checked += 1;
        }
    }
    assert!(checked > 100, "checked {checked} opaque status pixels");
}

#[test]
fn browser_bottom_controls_float_over_a_fullscreen_viewport_without_receiving_web_touches() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.set_navigation(true);
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    let theme = app.theme();
    let area = crate::browser::full_viewport(&theme);
    let frame = app.frame(&f, &i);
    let dock = frame.dock.as_ref().unwrap();
    let top = theme.m.h as f32 - dock.canvas.h as f32;
    assert!(area.x >= 0.0 && area.w <= W as f32);
    assert_eq!(area.x * 2.0 + area.w, W as f32);
    assert!(area.y >= theme.m.dp(48.0));
    assert_eq!(area.w, W as f32);
    assert_eq!(area.h % 4.0, 0.0);
    assert!(area.y + area.h <= top);
    let interactive = frame.browser_area.unwrap();
    assert!(interactive.y + interactive.h <= top + 1.0);
    assert_eq!(dock.hits.len(), 8, "six tools, address and menu");
    for hit in &dock.hits[..6] {
        assert!(hit.rect.w >= theme.m.dp(44.0));
        let x = (hit.rect.x + theme.m.dp(3.0)) as u32;
        let y = (hit.rect.y + theme.m.dp(3.0)) as u32;
        let at = ((y * dock.canvas.w + x) * 4) as usize;
        assert_eq!(&dock.canvas.data[at..at + 3], &[0, 0, 0], "no old individual button background");
    }
    assert!(dock.hits.iter().any(|h| h.action == Action::BrowserMenu));
    assert!(dock.hits.iter().any(|h| h.action == Action::BrowserAddress));
    let address = dock.hits.iter().find(|hit| hit.action == Action::BrowserAddress).unwrap().rect;
    let (x, y) = address.centre();
    assert!(
        app.touch(Touch::Down(x, y + top), &f, 1000)
            .iter()
            .all(|effect| !matches!(effect, Effect::BrowserPluginTouch { .. }))
    );
    app.touch(Touch::Up, &f, 1010);
    assert_eq!(app.screen(), Screen::BrowserAddress);
}

#[cfg(feature = "aera-browser")]
#[test]
fn immersive_browser_uses_every_display_pixel_and_has_no_hidden_chrome_hits() {
    let (f, i) = (fonts(), info());
    for (w, h, density) in [(720, 1600, 1.875), (W, H, 3.75)] {
        let mut app = App::new(Screen::Browser, w, h, density);
        app.set_navigation(true);
        app.state.browser.full_active = true;
        app.state.browser.view = BrowserView::Full;
        app.state.data = Data::Open;
        app.act(Action::BrowserMenu, 100);
        assert!(
            app.frame(&f, &i)
                .hits
                .iter()
                .any(|hit| hit.action == Action::BrowserImmersiveToggle)
        );
        app.act(Action::BrowserImmersiveToggle, 200);
        assert!(app.browser_immersive());
        assert_eq!(app.nav_height(), 0);
        let area = app.browser_viewport();
        assert_eq!(area, Rect::new(0.0, 0.0, w as f32, h as f32));
        app.browser_plugin_pixels(w, h, vec![72; w as usize * h as usize * 4]);
        app.state.browser.full_progress = Some(1.0);
        app.end_turn();
        let frame = app.frame(&f, &i);
        assert_eq!(frame.browser_area, Some(area));
        assert!(frame.dock.is_none() && frame.hits.is_empty());
        assert!(!frame.back && !frame.header && frame.fixed);
        let mut canvas = Canvas::new(w, h);
        app.compose(&mut canvas, &f, &i, "12:00", 1000);
        assert!(
            canvas.data.as_chunks::<4>().0.iter().all(|pixel| *pixel == [72; 4]),
            "no clock, status, dock or navigation is composited over the webpage"
        );
        for (x, y) in [
            (density * 20.0, density * 48.0),
            (w as f32 - density * 20.0, density * 48.0),
            (w as f32 / 2.0, h as f32 - density * 32.0),
        ] {
            assert!(matches!(
                app.touch(Touch::Down(x, y), &f, 1100).as_slice(),
                [Effect::BrowserPluginTouch { phase: 0, .. }]
            ));
            assert!(matches!(
                app.touch(Touch::Up, &f, 1110).as_slice(),
                [Effect::BrowserPluginTouch { phase: 2, .. }]
            ));
            assert_eq!(app.screen(), Screen::Browser);
        }
    }
}

#[test]
fn immersive_top_pull_and_hardware_back_always_restore_a_recovery_exit() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.set_navigation(true);
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    app.set_browser_immersive(true);
    app.frame(&f, &i);
    assert!(app.touch(Touch::Down(500.0, 5.0), &f, 1000).is_empty());
    assert!(matches!(
        app.touch(Touch::Up, &f, 1010).as_slice(),
        [Effect::BrowserPluginTap { .. }]
    ));
    assert!(app.browser_immersive(), "a short tap at the reserved edge does not exit");
    app.touch(Touch::Down(500.0, 5.0), &f, 1100);
    let effects = app.touch(Touch::Move(500.0, 200.0), &f, 1120);
    assert!(effects.contains(&Effect::BrowserPluginExitFullscreen));
    app.touch(Touch::Up, &f, 1130);
    app.tick(3000);
    assert_eq!(app.screen(), Screen::ControlCenter);
    assert!(!app.state.browser.immersive && app.shade.is_some());
    assert!(app.nav_height() > 0);
    app.dismiss_shade();
    assert_eq!(app.screen(), Screen::Browser);
    assert!(app.frame(&f, &i).dock.is_some());
    app.set_browser_immersive(true);
    assert_eq!(app.key(Key::Back, 4000), [Effect::BrowserPluginExitFullscreen]);
    assert!(!app.browser_immersive());
    assert_eq!(app.screen(), Screen::Browser, "back exits immersion before leaving the page");
}

#[test]
fn immersive_exit_works_from_all_edges_and_volume_up_without_navigating_the_page() {
    let (f, i) = (fonts(), info());
    for landscape in [false, true] {
        for edge in 0..4 {
            let mut app = App::new(Screen::Browser, W, H, PPD);
            app.set_navigation(true);
            app.state.browser.full_active = true;
            app.state.browser.view = BrowserView::Full;
            app.state.browser.landscape_mode = landscape;
            app.set_browser_immersive(true);
            app.frame(&f, &i);
            let dp = PPD;
            let (start, end) = match edge {
                0 => ((500.0, dp * 30.0), (dp * 20.0 + 500.0, dp * 58.0)),
                1 => ((500.0, H as f32 - dp * 10.0), (500.0, H as f32 - dp * 40.0)),
                2 => ((dp * 5.0, 500.0), (dp * 35.0, 500.0)),
                _ => ((W as f32 - dp * 5.0, 500.0), (W as f32 - dp * 35.0, 500.0)),
            };
            assert!(app.touch(Touch::Down(start.0, start.1), &f, 1000).is_empty());
            let effects = app.touch(Touch::Move(end.0, end.1), &f, 1010);
            assert!(effects.contains(&Effect::BrowserPluginExitFullscreen));
            assert!(!effects.iter().any(|effect| matches!(effect, Effect::BrowserPluginTouch { .. })));
            app.touch(Touch::Up, &f, 1020);
            app.tick(3000);
            assert!(!app.browser_immersive());
            assert_eq!(app.screen(), if edge == 0 { Screen::ControlCenter } else { Screen::Browser });
            assert!(app.nav_height() > 0);
        }
    }
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    app.set_browser_immersive(true);
    assert_eq!(app.key(Key::VolumeUp, 5000), [Effect::BrowserPluginExitFullscreen]);
    assert!(app.key_up(Key::VolumeUp, 5010).is_empty());
    assert_eq!(app.screen(), Screen::Browser);
    assert!(!app.browser_immersive());
}

#[test]
fn opening_control_center_explicitly_leaves_immersion() {
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.full_active = true;
    app.set_browser_immersive(true);
    assert!(
        app.act(Action::OpenControlCenter, 1000)
            .contains(&Effect::BrowserPluginExitFullscreen)
    );
    assert!(!app.state.browser.immersive);
    assert_eq!(app.screen(), Screen::ControlCenter);
}

#[test]
fn immersive_edge_taps_pass_through_but_cancelled_drags_do_not_click() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    app.set_browser_immersive(true);
    app.frame(&f, &i);
    let (x, y) = (PPD * 5.0, 500.0);
    assert!(app.touch(Touch::Down(x, y), &f, 1000).is_empty());
    let rect = app.browser_viewport();
    let expected = (((x - rect.x) / rect.w * 1000.0) as u16, ((y - rect.y) / rect.h * 1000.0) as u16);
    assert_eq!(
        app.touch(Touch::Up, &f, 1010),
        [Effect::BrowserPluginTap {
            x: expected.0,
            y: expected.1
        }]
    );
    assert!(app.touch(Touch::Up, &f, 1020).is_empty(), "release is not replayed");
    app.touch(Touch::Down(x, y), &f, 1100);
    assert!(app.touch(Touch::Move(x, y + PPD * 30.0), &f, 1110).is_empty());
    assert!(app.touch(Touch::Up, &f, 1120).is_empty(), "an aborted edge swipe is not a click");
    assert!(app.browser_immersive());
    let middle = W as f32 / 2.0;
    assert!(matches!(
        app.touch(Touch::Down(middle, y), &f, 1200).as_slice(),
        [Effect::BrowserPluginTouch { phase: 0, .. }]
    ));
    assert!(matches!(
        app.touch(Touch::Up, &f, 1210).as_slice(),
        [Effect::BrowserPluginTouch { phase: 2, .. }]
    ));
    app.touch(Touch::Down(x, y), &f, 1300);
    let exit = app.touch(Touch::Move(x + PPD * 30.0, y + PPD * 20.0), &f, 1310);
    assert!(exit.contains(&Effect::BrowserPluginExitFullscreen));
    assert!(app.touch(Touch::Move(x + PPD * 60.0, y), &f, 1320).is_empty());
    assert!(app.touch(Touch::Up, &f, 1330).is_empty());
    assert!(!app.browser_immersive());
}

#[test]
fn immersive_engine_viewport_stays_fixed_when_keyboard_or_another_page_opens() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.set_navigation(true);
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    app.set_browser_immersive(true);
    let area = app.browser_viewport();
    app.browser_plugin_focus(true);
    assert_eq!(app.screen(), Screen::BrowserAddress);
    assert_eq!(app.browser_viewport(), area);
    assert!(app.nav_height() > 0, "keyboard retains recovery navigation");
    let frame = app.frame(&f, &i);
    assert!(frame.browser_area.unwrap().h < area.h);
    assert!(frame.dock.is_none());
    app.browser_plugin_focus(false);
    assert!(app.browser_immersive());
    app.act(Action::NavHome, 1000);
    assert!(!app.browser_immersive());
    assert!(app.nav_height() > 0);
    assert_eq!(app.browser_plugin_fullscreen(true), [Effect::BrowserPluginExitFullscreen]);
    assert_eq!(app.screen(), Screen::Home, "background fullscreen cannot steal recovery focus");
}

#[cfg(feature = "aera-browser")]
#[test]
fn immersive_resize_drops_inflight_old_pixels_and_failure_restores_navigation() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.set_navigation(true);
    app.state.data = Data::Open;
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    let previous = app.browser_viewport();
    app.browser_plugin_fullscreen(true);
    app.browser_plugin_pixels(
        previous.w as u32,
        previous.h as u32,
        vec![128; previous.w as usize * previous.h as usize * 4],
    );
    assert!(app.state.browser.picture.is_none());
    app.browser_plugin_pixels(W, H, vec![128; W as usize * H as usize * 4]);
    assert!(app.state.browser.picture.is_some());
    app.browser_plugin_media_playing(true, 1000);
    for n in 0..3 {
        app.browser_plugin_failed("test failure".into(), 1100 + n);
    }
    assert!(!app.browser_immersive());
    assert!(app.nav_height() > 0);
    assert!(app.frame(&f, &i).dock.is_some());
}

#[test]
fn webpage_keys_edit_the_focused_page_instead_of_an_empty_local_copy() {
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.full_active = true;
    app.browser_plugin_focus(true);
    assert_eq!(app.screen(), Screen::BrowserAddress);
    assert_eq!(app.type_key(Cap::Backspace, 0), vec![Effect::BrowserPluginKey(8)]);
    assert_eq!(app.type_key(Cap::Char('x'), 0), vec![Effect::BrowserPluginText("x".into())]);
    assert_eq!(app.type_key(Cap::Space, 0), vec![Effect::BrowserPluginText(" ".into())]);
    assert!(app.state.browser.address.is_empty());
    assert!(app.state.browser.full_text);
    assert_eq!(app.type_key(Cap::Enter, 1), vec![Effect::BrowserPluginKey(13)]);
    assert_eq!(app.screen(), Screen::Browser);
    app.browser_plugin_focus(true);
    app.act(Action::NavBack, 2);
    assert_eq!(app.screen(), Screen::Browser);
    assert!(!app.state.browser.full_text);
}

#[cfg(feature = "pinyin-input")]
#[test]
fn webpage_pinyin_commits_to_the_engine_and_backspace_deletes_composition_first() {
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.full_active = true;
    app.browser_plugin_focus(true);
    app.act(Action::BrowserImeToggle, 0);
    for ch in "nihaox".chars() {
        assert!(app.type_key(Cap::Char(ch), 0).is_empty());
    }
    assert!(app.type_key(Cap::Backspace, 0).is_empty());
    assert_eq!(app.state.browser.ime.spelling, "nihao");
    let index = app
        .state
        .browser
        .ime
        .candidates
        .iter()
        .position(|candidate| candidate.text == "你好")
        .unwrap();
    assert_eq!(
        app.act(Action::BrowserImePick(index), 0),
        vec![Effect::BrowserPluginText("你好".into())]
    );
    assert!(app.state.browser.address.is_empty());
    assert_eq!(app.type_key(Cap::Backspace, 0), vec![Effect::BrowserPluginKey(8)]);
}

#[test]
fn keyboard_drags_never_move_recovery_or_leak_into_the_webpage() {
    let (f, i) = (fonts(), info());
    for (w, h, ppd) in [(720, 1600, 1.875), (W, H, PPD)] {
        for direct in [false, true] {
            let mut app = App::new(Screen::Browser, w, h, ppd);
            app.set_navigation(true);
            app.state.browser.full_active = true;
            app.state.browser.view = BrowserView::Full;
            if direct {
                app.browser_plugin_focus(true);
            } else {
                app.act(Action::BrowserAddress, 0);
            }
            app.end_turn();
            let t = app.theme();
            let frame = app.frame(&f, &i);
            assert!(frame.fixed);
            assert!(
                frame
                    .hits
                    .iter()
                    .filter(|hit| matches!(hit.action, Action::Type(_)))
                    .all(|hit| hit.rect.y + hit.rect.h <= h as f32 - t.bottom_inset + 1.0)
            );
            let key = frame
                .hits
                .iter()
                .find(|hit| hit.action == Action::Type(Cap::Char('g')))
                .unwrap()
                .rect;
            let (x, y) = key.centre();
            for dy in [-ppd * 80.0, ppd * 80.0] {
                let mut effects = app.touch(Touch::Down(x, y), &f, 1000);
                effects.extend(app.touch(Touch::Move(x, y + dy), &f, 1010));
                effects.extend(app.touch(Touch::Up, &f, 1020));
                assert!(effects.iter().all(|effect| matches!(effect, Effect::Haptic)));
                assert_eq!(app.top().1, 0.0);
                assert_eq!(app.pull, 0.0);
                assert!(app.bounce.is_none() && app.fling.is_none());
                assert_eq!(app.screen(), Screen::BrowserAddress);
            }
            let area = app.frame(&f, &i).browser_area.unwrap();
            let effects = app.touch(Touch::Down(area.w / 2.0, area.y + 50.0), &f, 1100);
            assert!(matches!(effects.as_slice(), [Effect::BrowserPluginTouch { phase: 0, .. }]));
            app.touch(Touch::Up, &f, 1110);
        }
    }
}

#[test]
fn full_page_bookmarks_and_menu_zoom_remain_usable() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    app.state.browser.url = "https://example.com/".into();
    app.act(Action::BrowserBookmark, 0);
    assert_eq!(app.state.browser.bookmarks.len(), 1);
    app.act(Action::BrowserBookmark, 0);
    assert!(app.state.browser.bookmarks.is_empty());
    app.act(Action::BrowserMenu, 1000);
    app.frame(&f, &i);
    let fx = app.act(Action::BrowserZoomIn, 1100);
    assert_eq!(app.screen(), Screen::Browser);
    assert_eq!(fx, vec![Effect::BrowserPluginPinch { scale: 1.2 }]);
}

#[test]
fn smaller_navigation_buttons_vibrate_once_and_obey_the_touch_preference() {
    let (f, i) = (fonts(), info());
    for enabled in [true, false] {
        for button in 0..3 {
            let mut app = App::new(Screen::Home, W, H, PPD);
            app.set_navigation(true);
            app.state.vibrate = enabled;
            app.frame(&f, &i);
            assert_eq!(app.nav_height(), (PPD * 32.0) as u32);
            let x = app.navigation_buttons()[button].centre().0;
            let y = H as f32 - app.nav_height() as f32 / 2.0;
            let expected = if enabled { vec![Effect::Haptic] } else { Vec::new() };
            assert_eq!(app.touch(Touch::Down(x, y), &f, 1000), expected);
            assert!(
                app.touch(Touch::Down(x, y), &f, 1001).is_empty(),
                "duplicate down does not vibrate twice"
            );
            assert!(
                app.touch(Touch::Up, &f, 1010).is_empty(),
                "release performs navigation without another haptic"
            );
            assert_eq!(app.screen(), if button == 2 { Screen::Tasks } else { Screen::Home });
        }
    }
}

#[test]
fn navigation_uses_a_compact_group_and_dragging_out_cancels_the_button() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.set_navigation(true);
    app.frame(&f, &i);
    let buttons = app.navigation_buttons();
    assert!(buttons[0].x > 0.0 && buttons[2].x + buttons[2].w < W as f32);
    assert_eq!(buttons[0].w, PPD * 84.0);
    let (x, y) = buttons[1].centre();
    assert!(app.touch(Touch::Down(5.0, y), &f, 1000).is_empty());
    app.touch(Touch::Up, &f, 1001);
    assert_eq!(app.screen(), Screen::Settings);
    app.touch(Touch::Down(x, y), &f, 1010);
    app.touch(Touch::Move(5.0, y), &f, 1020);
    app.touch(Touch::Up, &f, 1030);
    assert_eq!(app.screen(), Screen::Settings);
    assert!(app.nav_press.is_none());
}

#[test]
fn control_center_has_no_header_settings_shortcut() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::ControlCenter, W, H, PPD);
    let top = app.theme().m.dp(132.0);
    let frame = app.frame(&f, &i);
    let settings: Vec<_> = frame
        .hits
        .iter()
        .filter(|hit| hit.action == Action::Open(Screen::Settings))
        .collect();
    assert_eq!(settings.len(), 1, "the grid settings shortcut is retained");
    assert!(settings[0].rect.y >= top, "no upper-right header shortcut remains");
}

#[test]
fn control_tiles_hold_to_open_without_toggling_or_clicking_on_release() {
    let (f, i) = (fonts(), info());
    for vibrate in [false, true] {
        for (action, page) in [
            (Action::Toggle(Switch::Wlan), Screen::Wlan),
            (Action::Toggle(Switch::Vibrate), Screen::Settings),
            (Action::Toggle(Switch::FileTransfer), Screen::Settings),
            (Action::Toggle(Switch::DoubleTap), Screen::ScreenOff),
            (Action::Open(Screen::Browser), Screen::Browser),
            (Action::Open(Screen::Files), Screen::Files),
            (Action::Open(Screen::BrowserDownloads), Screen::BrowserDownloads),
            (Action::Open(Screen::Terminal), Screen::Terminal),
            (Action::Open(Screen::RootModules), Screen::RootModules),
            (Action::Open(Screen::Partitions), Screen::Partitions),
            (Action::Open(Screen::Settings), Screen::Settings),
            (Action::NavTasks, Screen::Tasks),
        ] {
            let mut app = App::new(Screen::Home, W, H, PPD);
            app.state.wlan = Some(connected("Home"));
            app.state.double_tap_supported = true;
            app.state.data = Data::Open;
            if page == Screen::Browser {
                app.state.browser.full_active = true;
                app.state.browser.view = BrowserView::Full;
            }
            app.state.vibrate = vibrate;
            let old = (app.state.vibrate, app.state.file_transfer, app.state.double_tap);
            app.open_shade(0, false);
            app.settle_shade(true, 0, 0.0);
            app.tick(2000);
            let frame = app.frame(&f, &i);
            let (x, y) = frame.hits.iter().find(|hit| hit.action == action).unwrap().rect.centre();
            assert!(app.touch(Touch::Down(x, y), &f, 3000).is_empty());
            assert_eq!(app.deadline(3000), Some(3000 + LONG_PRESS_MS));
            assert!(app.tick(3000 + LONG_PRESS_MS - 1).is_empty());
            let effects = app.tick(3000 + LONG_PRESS_MS);
            assert_eq!(
                effects.iter().filter(|effect| **effect == Effect::Haptic).count(),
                usize::from(vibrate)
            );
            assert!(
                !effects
                    .iter()
                    .any(|effect| matches!(effect, Effect::Wlan(_) | Effect::FileTransfer(_)))
            );
            assert_eq!(app.screen(), page);
            if page == Screen::Wlan {
                assert!(
                    effects.contains(&Effect::ScanWlan),
                    "timed navigation must reach the runtime scan handler"
                );
            }
            assert!(app.shade.is_none());
            assert_eq!((app.state.vibrate, app.state.file_transfer, app.state.double_tap), old);
            assert!(app.touch(Touch::Up, &f, 3500).is_empty());
            let mut canvas = Canvas::new(W, H);
            app.compose(&mut canvas, &f, &i, "12:00", 3500);
            assert!(app.tick(3600).is_empty());
        }
    }
}

#[test]
fn moving_away_cancels_control_hold_and_short_tap_still_toggles() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::ControlCenter, W, H, PPD);
    app.state.vibrate = true;
    let frame = app.frame(&f, &i);
    let (x, y) = frame
        .hits
        .iter()
        .find(|hit| hit.action == Action::Toggle(Switch::Vibrate))
        .unwrap()
        .rect
        .centre();
    app.touch(Touch::Down(x, y), &f, 3000);
    app.touch(Touch::Move(x + PPD * 20.0, y), &f, 3010);
    assert!(app.tick(3500).is_empty());
    assert!(app.touch(Touch::Up, &f, 3510).is_empty());
    assert!(app.state.vibrate);
    app.touch(Touch::Down(x, y), &f, 4000);
    app.touch(Touch::Up, &f, 4100);
    assert!(!app.state.vibrate);
    assert_eq!(app.screen(), Screen::ControlCenter);
}

#[test]
fn browser_address_changes_reuse_one_browser_page() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.data = Data::Open;
    app.act(Action::Open(Screen::Browser), 0);
    for n in 0..10 {
        app.act(Action::BrowserAddress, 0);
        app.state.browser.address = format!("https://example.com/{n}");
        app.act(Action::Type(Cap::Enter), 0);
        assert_eq!(app.screen(), Screen::Browser);
        assert_eq!(app.stack.len(), 2);
    }
    app.act(Action::NavHome, 0);
    assert_eq!(app.screen(), Screen::Home);
}

#[cfg(feature = "aera-browser")]
#[test]
fn browser_entry_defaults_to_full_mode_and_resumes_without_another_worker() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.data = Data::Open;
    assert!(
        matches!(app.act(Action::Open(Screen::Browser), 0).as_slice(), [Effect::BrowserPluginStart { url, .. }] if url == "https://cn.bing.com/" || url == "https://cn.bing.com")
    );
    assert_eq!(app.screen(), Screen::Browser);
    assert!(app.state.browser.prefer_full && app.state.browser.full_active);
    app.act(Action::NavHome, 1);
    assert!(app.act(Action::Open(Screen::Browser), 2).is_empty());
    assert_eq!(app.screen(), Screen::Browser);
}

#[cfg(feature = "aera-browser")]
#[test]
fn browser_recovery_tries_full_twice_then_falls_back_without_a_restart_loop() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.data = Data::Open;
    app.act(Action::Open(Screen::Browser), 0);
    for attempt in 1..=2 {
        let effects = app.browser_plugin_failed("test worker terminated".into(), attempt);
        assert!(matches!(effects.as_slice(), [Effect::BrowserPluginStart { .. }]));
        assert_eq!(app.state.browser.full_failures, attempt as u8);
        app.browser_navigated("https://cn.bing.com/".into());
        assert_eq!(app.state.browser.full_failures, attempt as u8);
    }
    let effects = app.browser_plugin_failed("test worker terminated".into(), 3);
    assert!(matches!(effects.as_slice(), [Effect::BrowserOpen { .. }]));
    assert!(!app.state.browser.full_active && !app.state.browser.prefer_full);
    assert!(app.browser_plugin_failed("stale failure".into(), 4).is_empty());
    assert!(matches!(
        app.act(Action::BrowserReload, 5).as_slice(),
        [Effect::BrowserPluginStart { .. }]
    ));
    assert_eq!(app.state.browser.full_failures, 0);
}

#[cfg(feature = "aera-browser")]
#[test]
fn hidden_browser_failure_never_restarts_or_switches_the_users_page() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.data = Data::Open;
    app.act(Action::Open(Screen::Browser), 0);
    app.act(Action::NavHome, 1);
    assert!(app.browser_plugin_failed("worker stopped".into(), 2).is_empty());
    assert_eq!(app.screen(), Screen::Home);
    assert!(!app.state.browser.full_active);
    assert!(matches!(
        app.act(Action::Open(Screen::Browser), 3).as_slice(),
        [Effect::BrowserPluginStart { .. }]
    ));
}

#[cfg(feature = "aera-browser")]
#[test]
fn control_center_keeps_its_hits_when_a_full_browser_is_active() {
    let f = fonts();
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.set_navigation(true);
    app.state.browser.full_active = true;
    app.state.data = Data::Open;
    app.state.wlan = Some(connected("Skyward"));
    app.act(Action::OpenControlCenter, 0);
    app.tick(2000);
    let frame = app.frame(&f, &info());
    let wifi = frame
        .hits
        .iter()
        .find(|hit| matches!(hit.action, Action::Toggle(Switch::Wlan)))
        .map(|hit| hit.rect)
        .expect("control center keeps the WLAN hit");
    let (x, y) = wifi.centre();
    app.browser_plugin_pixels(4, 4, vec![255; 64]);
    assert!(app.frame.is_some(), "background frames must keep control-center hit regions");
    app.touch(Touch::Down(x, y), &f, 2100);
    app.browser_plugin_pixels(4, 4, vec![128; 64]);
    let effects = app.touch(Touch::Up, &f, 2110);
    assert_eq!(effects, vec![Effect::Wlan(false)]);
    assert!(app.shade.is_some() && app.screen() == Screen::ControlCenter);
    assert_eq!(app.state.wlan.as_ref().unwrap().power, Power::Off);
    let brightness = app
        .frame(&f, &info())
        .hits
        .iter()
        .find(|hit| hit.action == Action::Slider(Slider::Brightness))
        .unwrap()
        .rect;
    let cy = brightness.centre().1;
    app.touch(Touch::Down(brightness.x + brightness.w * 0.25, cy), &f, 2200);
    app.browser_plugin_pixels(4, 4, vec![64; 64]);
    app.touch(Touch::Move(brightness.x + brightness.w * 0.75, cy), &f, 2210);
    app.touch(Touch::Up, &f, 2220);
    assert!((app.state.brightness - 0.75).abs() < 0.001);
}

#[test]
fn media_playback_blocks_idle_sleep_but_pause_and_manual_power_still_work() {
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.full_active = true;
    app.state.screen_off = 0;
    app.tick(0);
    app.browser_plugin_media_playing(true, 1000);
    app.tick(60_000);
    assert!(app.display_on() && app.idle_due().is_none());
    app.sleep(60_001);
    app.tick(60_002);
    assert!(!app.display_on(), "media events must not override manual sleep");
    app.wake(60_003);
    app.browser_plugin_media_playing(false, 60_100);
    app.tick(75_101);
    assert!(!app.display_on());
}

#[cfg(feature = "aera-browser")]
#[test]
fn video_failure_does_not_fall_back_to_a_text_renderer_or_restart_forever() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.data = Data::Open;
    app.act(Action::Open(Screen::Browser), 0);
    app.browser_plugin_media_playing(true, 1);
    for at in 2..=3 {
        assert!(matches!(
            app.browser_plugin_failed("media interrupted".into(), at).as_slice(),
            [Effect::BrowserPluginStart { .. }]
        ));
    }
    assert!(app.browser_plugin_failed("media interrupted".into(), 4).is_empty());
    assert!(!app.state.browser.full_active && app.state.browser.prefer_full);
    assert!(matches!(app.state.browser.view, BrowserView::Failed { .. }));
    assert!(matches!(
        app.act(Action::BrowserReload, 5).as_slice(),
        [Effect::BrowserPluginStart { .. }]
    ));
}

#[test]
fn browser_pinch_is_bounded_and_cannot_operate_the_control_center() {
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.full_active = true;
    app.state.browser.desktop_mode = true;
    assert_eq!(app.browser_pinch(1.25, 1000), vec![Effect::BrowserPluginPinch { scale: 1.25 }]);
    assert!(app.browser_pinch(f32::NAN, 1001).is_empty());
    assert!(app.browser_pinch(0.0, 1001).is_empty());
    app.act(Action::OpenControlCenter, 1010);
    assert!(app.browser_pinch(1.5, 1020).is_empty());
}

#[test]
fn a_partial_shade_reveals_its_header_immediately() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.set_navigation(true);
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:34", 0);
    app.touch(Touch::Down(500.0, 10.0), &f, 1000);
    app.touch(Touch::Move(500.0, 600.0), &f, 1100);
    app.compose(&mut c, &f, &i, "12:34", 1100);
    assert_eq!(app.shade.as_ref().unwrap().extent, 590.0);
    let header = c.crop(60, 110, 800, 160);
    assert!(
        header
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 180 && p[1] > 180 && p[2] > 180),
        "the large clock is visible during the pull"
    );
}

#[test]
fn navigation_home_parks_the_page_without_cancelling_an_installer() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Installing, W, H, PPD);
    app.set_navigation(true);
    app.state.install = Install::Scripting {
        stage: None,
        running: true,
    };
    let job = app.state.install.clone();
    assert!(app.act(Action::NavHome, 1000).is_empty());
    assert_eq!(app.screen(), Screen::Home);
    assert_eq!(app.state.install, job);
    assert!(app.act(Action::NavTasks, 1100).is_empty());
    assert_eq!(app.screen(), Screen::Tasks);
    assert!(app.act(Action::ResumeTask, 1200).is_empty());
    assert_eq!(app.screen(), Screen::Installing);
    assert_eq!(app.state.install, job);
    app.frame(&f, &i);
    let y = H as f32 - app.nav_height() as f32 / 2.0;
    app.touch(Touch::Down(W as f32 / 2.0, y), &f, 2000);
    assert!(app.touch(Touch::Up, &f, 2010).is_empty());
    assert_eq!(app.screen(), Screen::Home);
    assert_eq!(app.state.install, job);
}

#[test]
fn shade_tracks_the_finger_then_dismisses_to_the_previous_scroll_position() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Settings, &f, &i);
    app.set_navigation(true);
    app.frame(&f, &i);
    app.top_mut().1 = 200.0;
    app.touch(Touch::Down(500.0, 10.0), &f, 1000);
    app.touch(Touch::Move(500.0, 400.0), &f, 1100);
    assert_eq!(app.screen(), Screen::ControlCenter);
    assert_eq!(app.shade.as_ref().unwrap().extent, 390.0);
    app.touch(Touch::Up, &f, 1120);
    app.tick(2200);
    assert_eq!(app.shade.as_ref().unwrap().extent, H as f32);
    app.frame(&f, &i);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Slider(Slider::Brightness))
        .unwrap()
        .rect;
    app.touch(Touch::Down(track.x + track.w * 0.5, track.centre().1), &f, 2300);
    app.touch(Touch::Move(track.x + track.w * 0.8, track.centre().1 - 10.0), &f, 2340);
    app.touch(Touch::Up, &f, 2360);
    assert!(app.shade.is_some());
    app.touch(Touch::Down(500.0, 60.0), &f, 2500);
    app.touch(Touch::Move(500.0, -500.0), &f, 2580);
    app.touch(Touch::Up, &f, 2590);
    app.tick(3700);
    assert!(app.shade.is_none());
    assert_eq!(app.screen(), Screen::Settings);
    assert_eq!(app.top().1, 200.0);
}

#[test]
fn browser_touch_forwards_down_move_and_up_without_waiting_for_release() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.set_navigation(true);
    app.state.browser.full_active = true;
    app.state.browser.view = BrowserView::Full;
    let area = app.frame(&f, &i).browser_area.unwrap();
    let (x, y) = area.centre();
    let (px, py) = (500, 500);
    assert_eq!(
        app.touch(Touch::Down(x, y), &f, 1000),
        vec![Effect::BrowserPluginTouch { phase: 0, x: px, y: py }]
    );
    assert!(matches!(
        app.touch(Touch::Move(x, y - 200.0), &f, 1030).as_slice(),
        [Effect::BrowserPluginTouch { phase: 1, .. }]
    ));
    assert!(matches!(
        app.touch(Touch::Up, &f, 1040).as_slice(),
        [Effect::BrowserPluginTouch { phase: 2, .. }]
    ));
    assert!(matches!(app.drag, Drag::Idle));
}

#[test]
fn navigation_leaves_keyboard_and_browser_tools_above_its_reserved_strip() {
    let (f, i) = (fonts(), info());
    for screen in [Screen::BrowserAddress, Screen::Terminal, Screen::WlanPassword, Screen::Search] {
        let mut app = App::new(screen, W, H, PPD);
        app.set_navigation(true);
        let theme = app.theme();
        if screen == Screen::WlanPassword {
            continue;
        }
        let fr = app.frame(&f, &i);
        for hit in fr.hits.iter().filter(|h| matches!(h.action, Action::Type(_))) {
            assert!(hit.rect.y + hit.rect.h <= theme.m.h as f32 + 1.0);
        }
        if let Some(dock) = &fr.dock {
            assert!(dock.canvas.h < theme.m.h);
        }
    }
}

#[test]
fn aurora_tabs_keep_the_previous_page_and_navigation_stays_in_the_selected_engine() {
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.browser_read("https://example.com/one".into());
    app.act(Action::BrowserNewTab, 0);
    app.browser_read("https://example.com/two".into());
    app.act(Action::BrowserPickTab(0), 0);
    assert_eq!(app.state.browser.url, "https://example.com/one");
    app.state.browser.full_active = true;
    let effects = app.browser_read("https://example.com/three".into());
    assert!(matches!(effects.as_slice(), [Effect::BrowserPluginStart { .. }]));
    assert!(matches!(app.act(Action::BrowserBack,0).as_slice(),[Effect::BrowserPluginStart {url,..}] if url.ends_with("/one")));
    assert_eq!(app.act(Action::BrowserZoomIn, 0), vec![Effect::BrowserPluginPinch { scale: 1.2 }]);
    assert_eq!(
        app.act(Action::BrowserPluginScroll(-300.0), 0),
        vec![Effect::BrowserPluginScroll { dy: -300 }]
    );
}

#[test]
fn shade_gesture_and_image_picker_have_explicit_entry_points() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    assert!(!app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Open(Screen::PowerOff)));
    app.touch(Touch::Down(500.0, 10.0), &f, 1000);
    app.touch(Touch::Move(500.0, 240.0), &f, 1100);
    app.touch(Touch::Up, &f, 1200);
    assert_eq!(app.screen(), Screen::ControlCenter);
    assert!(app.act(Action::Toggle(Switch::Wlan), 1300).is_empty());
    app.state.data = Data::Open;
    assert!(matches!(
        app.act(Action::FlashPartitionImage, 1400).as_slice(),
        [Effect::ReadPlace(Place::Storage, ..)]
    ));
    assert!(app.state.image_only);
    app.act(Action::Open(Screen::Install), 1500);
    assert!(!app.state.image_only);
}

#[test]
fn browser_file_offers_download_to_decrypted_internal_storage_only() {
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.browser.view = BrowserView::File(skyward_net::web::FileOffer {
        url: "https://example.com/a.zip".into(),
        name: "a.zip".into(),
        kind: "application/zip".into(),
        size: Some(42),
    });
    assert!(app.act(Action::BrowserSaveFile, 0).is_empty());
    app.state.data = Data::Open;
    let effects = app.act(Action::BrowserSaveFile, 1);
    assert!(matches!(effects.as_slice(),[Effect::BrowserDownload{path,..}] if path=="/data/media/0/Download/Skyward/a.zip"));
    app.browser_saved(42, Some(42));
    app.browser_save_ended(Ok(()), 2);
    assert_eq!(
        app.act(Action::BrowserOpenDownload(0), 3),
        vec![Effect::InstallPath("/data/media/0/Download/Skyward/a.zip".into())]
    );
}

#[test]
fn paused_browser_downloads_resume_from_one_list_entry_and_failures_can_retry() {
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.state.data = Data::Open;
    app.browser_offered_download("https://example.com/rom.zip".into(), "rom.zip".into(), 0);
    app.browser_saved(40, Some(100));
    assert_eq!(app.act(Action::BrowserStopDownload, 1), vec![Effect::StopBrowser]);
    app.browser_save_ended(Err("stopped as asked".into()), 2);
    assert_eq!(app.state.browser.downloads[0].state, BrowserSave::Stopped);
    assert_eq!(app.state.browser.downloads[0].done, 40);
    assert!(
        matches!(app.act(Action::BrowserOpenDownload(0), 3).as_slice(), [Effect::BrowserDownload { url, .. }] if url == "https://example.com/rom.zip")
    );
    assert_eq!(app.state.browser.downloads.len(), 1);
    app.browser_saved(40, Some(100));
    app.browser_save_ended(Err("connection reset".into()), 4);
    assert!(matches!(
        app.act(Action::BrowserOpenDownload(0), 5).as_slice(),
        [Effect::BrowserDownload { .. }]
    ));
    assert_eq!(app.state.browser.downloads.len(), 1);
}

#[test]
fn full_browser_search_uses_html_instead_of_the_native_rss_reader() {
    let mut app = App::new(Screen::BrowserAddress, W, H, PPD);
    app.state.browser.full_active = true;
    assert!(
        matches!(app.browser_go("中文搜索", 0).as_slice(), [Effect::BrowserPluginStart { url, .. }] if url.starts_with("https://cn.bing.com/search?q=") && !url.contains("format=rss"))
    );
    assert_eq!(
        skyward_net::web::full_address("https://example.com/page?format=rss"),
        "https://example.com/page?format=rss"
    );
}

#[test]
fn native_rss_searches_are_converted_when_opened_or_revisited_in_full_mode() {
    let rss = "https://cn.bing.com/search?format=rss&setlang=zh-Hans&q=nihao";
    let html = "https://cn.bing.com/search?setlang=zh-Hans&q=nihao";
    let mut app = App::new(Screen::Browser, W, H, PPD);
    app.browser_read(rss.into());
    app.browser_read("https://example.com/".into());
    app.state.browser.full_active = true;
    assert!(matches!(app.act(Action::BrowserBack, 0).as_slice(), [Effect::BrowserPluginStart { url, .. }] if url == html));
    assert_eq!(app.state.browser.url, html);
    assert_eq!(app.state.browser.address, html);
    assert_eq!(app.state.browser.history[0], html);
    assert!(matches!(app.browser_read(rss.into()).as_slice(), [Effect::BrowserPluginStart { url, .. }] if url == html));
    assert!(
        matches!(app.browser_read("https://example.com/feed.xml".into()).as_slice(), [Effect::BrowserPluginStart { url, .. }] if url == "https://example.com/feed.xml")
    );
}

#[test]
fn browser_address_cursor_places_and_drags_in_both_directions_without_reloading_text() {
    let (f, i) = (fonts(), info());
    for full in [false, true] {
        let mut app = App::new(Screen::BrowserAddress, W, H, PPD);
        app.state.browser.full_active = full;
        app.state.browser.address = "https://example.com/你好/path".into();
        app.state.browser.address_cursor = app.state.browser.address.len();
        app.frame(&f, &i);
        let t = app.theme();
        let top = if full {
            crate::browser::full_input_viewport(&t).y + crate::browser::full_input_viewport(&t).h
        } else {
            app.frame(&f, &i)
                .hits
                .iter()
                .find(|hit| hit.action == Action::BrowserAddress)
                .unwrap()
                .rect
                .y
        };
        let y = top + t.m.dp(28.0);
        let x = t.m.margin + t.m.inner;
        let original = app.state.browser.address.clone();
        app.touch(Touch::Down(x, y), &f, 1000);
        let first = app.state.browser.address_cursor;
        app.touch(Touch::Move(x + t.m.dp(100.0), y), &f, 1010);
        assert!(app.state.browser.address_cursor > first);
        app.touch(Touch::Move(x, y), &f, 1020);
        assert!(app.state.browser.address_cursor <= first);
        app.touch(Touch::Up, &f, 1030);
        assert_eq!(app.state.browser.address, original);
        app.state.browser.address_cursor = "https://example.com/你".len();
        app.type_key(Cap::Backspace, 1040);
        assert_eq!(app.state.browser.address, "https://example.com/好/path");
        app.type_key(Cap::Char('你'), 1050);
        assert_eq!(app.state.browser.address, original);
        app.type_key(Cap::Space, 1060);
        assert!(app.state.browser.address.contains("你 好"));
    }
}

#[test]
fn deleting_a_download_requires_confirmation_and_keeps_other_tasks_indexed() {
    let mut app = App::new(Screen::BrowserDownloads, W, H, PPD);
    app.state.data = Data::Open;
    app.browser_offered_download("https://example.com/one.zip".into(), "one.zip".into(), 0);
    app.browser_save_ended(Err("stopped as asked".into()), 1);
    app.browser_offered_download("https://example.com/two.zip".into(), "two.zip".into(), 2);
    assert!(app.act(Action::BrowserAskRemoveDownload(0), 3).is_empty());
    assert_eq!(app.state.browser.downloads.len(), 2);
    app.act(Action::BrowserCancelRemoveDownload, 4);
    assert_eq!(app.state.browser.remove_download, None);
    app.act(Action::BrowserAskRemoveDownload(0), 5);
    let effects = app.act(Action::BrowserConfirmRemoveDownload, 6);
    let [Effect::RemoveBrowserDownload { path }] = effects.as_slice() else {
        panic!("confirmed deletion produces one cleanup request")
    };
    assert_eq!(path, "/data/media/0/Download/Skyward/one.zip");
    assert!(
        app.browser_offered_download("https://example.com/three.zip".into(), "three.zip".into(), 7)
            .is_empty()
    );
    app.browser_download_removed(path, Ok(()), 8);
    assert_eq!(app.state.browser.downloads.len(), 1);
    assert_eq!(app.state.browser.saving, Some(0));
    assert_eq!(app.state.browser.downloads[0].name, "two.zip");
}

#[test]
fn the_plugin_is_not_finished_until_the_checksum_checked_worker_ends() {
    let mut app = App::new(Screen::BrowserFull, W, H, PPD);
    app.state.data = Data::Open;
    let effects = app.act(Action::BrowserPluginDownload, 0);
    assert!(matches!(effects.as_slice(), [Effect::BrowserPluginDownload { .. }]));
    app.plugin_saved(100, Some(100), 1);
    assert!(!app.state.browser.plugin_download.as_ref().unwrap().finished);
    app.plugin_downloaded(Err("checksum mismatch".into()));
    let download = app.state.browser.plugin_download.as_ref().unwrap();
    assert!(!download.finished && download.error.is_some());
    app.plugin_downloaded(Ok(()));
    let download = app.state.browser.plugin_download.as_ref().unwrap();
    assert!(download.finished && download.error.is_none());
}
fn fonts() -> Fonts {
    crate::font::bundled()
}

#[test]
fn keyboard_haptics_obey_the_vibration_switch() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::BrowserAddress, W, H, PPD);
    for enabled in [true, false] {
        app.state.vibrate = enabled;
        app.refresh();
        let (x, y) = spot(&mut app, &f, &i, Action::Type(Cap::Char('q')));
        let effects = app.touch(Touch::Down(x, y), &f, 1000);
        assert_eq!(effects.contains(&Effect::Haptic), enabled);
        app.touch(Touch::Up, &f, 1100);
    }
}
fn info() -> DeviceInfo {
    DeviceInfo {
        model: "Xiaomi 13 Ultra".into(),
        codename: "ishtar".into(),
        soc: None,
        cpu: Cpu {
            cores: 8,
            max_khz: Some(3_187_200),
        },
        ram_kib: 15_556_972,
        panel: Some((W, H)),
        screen: Some(6.73),
        os_version: None,
        android: None,
        locale: None,
        security_patch: None,
        kernel: "5.15.178".into(),
        storage: Storage {
            total: 496_550_785_024,
            free: None,
        },
        battery: None,
        slot: None,
        snapshot: SnapshotState::None,
        boot: Boot {
            unlocked: None,
            verified: None,
        },
    }
}
#[test]
fn a_page_drawn_a_band_at_a_time_is_the_page_drawn_whole() {
    let (f, i) = (fonts(), info());
    let state = State::default();
    let screens = [
        Screen::Decrypt,
        Screen::Home,
        Screen::Settings,
        Screen::Reboot,
        Screen::Wipe,
        Screen::Mount,
        Screen::Install,
        Screen::Choose(Place::Storage),
        Screen::Specs,
        Screen::ScreenOff,
        Screen::Search,
        Screen::Backup,
        Screen::Restore,
        Screen::Advanced,
        Screen::Avb,
        Screen::Files,
        Screen::Terminal,
        Screen::Language,
        Screen::TimeZone,
        Screen::Logs,
        Screen::Update,
        Screen::SkywardUpdate,
        Screen::License,
        Screen::FontLicense,
        Screen::Slot,
        Screen::Fastboot,
        Screen::Erase,
        Screen::CantStart,
        Screen::Unhandled,
    ];
    for t in [Theme::light(W, H, PPD), Theme::dark(W, H, PPD).pressed()] {
        for screen in screens {
            let cx = |band| Ctx {
                t: &t,
                f: &f,
                info: &i,
                state: &state,
                term: None,
                band,
            };
            let whole = pages::render(screen, &cx(None)).canvas;
            let row = whole.w as usize * 4;
            let differs = |c: &Canvas| c.data.chunks(row).zip(whole.data.chunks(row)).position(|(a, b)| a != b);
            let mut cuts: Vec<u32> = [0, 1, 377, 1201, 3000, 3001, 4444, u32::MAX].map(|k| k.min(whole.h)).to_vec();
            cuts.dedup();
            let mut joined: Option<Canvas> = None;
            for rows in cuts.windows(2) {
                let part = pages::render(
                    screen,
                    &cx(Some(Band {
                        rows: (rows[0], rows[1]),
                        cols: (0, W),
                    })),
                )
                .canvas;
                assert_eq!(
                    (part.h, part.held()),
                    (whole.h, rows[0]..rows[1]),
                    "{screen:?}: a band is laid out as the whole"
                );
                match &mut joined {
                    Some(c) => c.join(&part),
                    None => joined = Some(part),
                }
            }
            let joined = joined.expect("a page has rows");
            assert!(joined.whole(), "{screen:?}: the bands make the whole");
            assert_eq!(differs(&joined), None, "{screen:?}: the first row drawn otherwise in bands");
            let mut across = Canvas::unfilled(W, whole.h, 0..whole.h);
            for cols in [0, 1, 333, 720, W - 1, W].windows(2) {
                let part = pages::render(
                    screen,
                    &cx(Some(Band {
                        rows: (0, whole.h),
                        cols: (cols[0], cols[1]),
                    })),
                )
                .canvas;
                assert_eq!(part.cols(), cols[0]..cols[1], "{screen:?}: a part across holds its columns");
                across.replace(&part);
            }
            assert_eq!(differs(&across), None, "{screen:?}: the first row drawn otherwise across");
        }
    }
}
#[test]
fn a_page_at_rest_draws_the_rows_below_the_screen_a_band_at_a_time() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.compose(&mut Canvas::new(W, H), &f, &i, "12:00", 0);
    assert_eq!(
        app.frame.as_ref().unwrap().canvas.held(),
        0..H,
        "the page shown has the rows on screen drawn"
    );
    let mut bands = 1;
    while app.draw_ahead(&f, &i) {
        bands += 1;
    }
    let t = app.theme();
    let whole = pages::render(
        Screen::Settings,
        &Ctx {
            t: &t,
            f: &f,
            info: &i,
            state: &app.state,
            term: None,
            band: None,
        },
    )
    .canvas;
    let page = &app.frame.as_ref().unwrap().canvas;
    assert!(whole.h > H + H / 4, "a page well over a screen long, drawn in {bands} more bands");
    assert!(
        page.whole() && page.data == whole.data,
        "the page drawn a band at a time is the page drawn whole"
    );
}
#[test]
fn a_page_typed_on_draws_again_only_what_changed_and_shows_what_it_would_drawn_anew() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Search, W, H, PPD);
    let mut out = Canvas::new(W, H);
    app.compose(&mut out, &f, &i, "12:00", 0);
    let anew = |app: &App| {
        let t = app.theme();
        pages::render(
            Screen::Search,
            &Ctx {
                t: &t,
                f: &f,
                info: &i,
                state: &app.state,
                term: None,
                band: None,
            },
        )
    };
    for cap in [
        Cap::Char('b'),
        Cap::Char('f'),
        Cap::Space,
        Cap::Backspace,
        Cap::Shift,
        Cap::Char('q'),
    ] {
        let (x, y) = spot(&mut app, &f, &i, Action::Type(cap));
        app.touch(Touch::Down(x, y), &f, 0);
        assert!(
            app.frame.is_some() && !app.stale.is_empty() || cap == Cap::Shift,
            "{cap:?}: the key held is drawn again on its own"
        );
        app.compose(&mut out, &f, &i, "12:00", 0);
        assert!(
            app.frame.as_ref().unwrap().canvas.data == anew(&app).canvas.data,
            "{cap:?} held down"
        );
        app.touch(Touch::Up, &f, 0);
        app.compose(&mut out, &f, &i, "12:00", 0);
        let (drawn, fresh) = (app.frame.as_ref().unwrap(), anew(&app));
        assert!(drawn.canvas.data == fresh.canvas.data, "{cap:?} typed");
        assert_eq!(
            drawn.hits.iter().map(|h| h.action).collect::<Vec<_>>(),
            fresh.hits.iter().map(|h| h.action).collect::<Vec<_>>(),
            "{cap:?} typed"
        );
    }
    assert_eq!(app.state.search, "bfQ");
}
/// Where on screen the first control doing `action` sits.
/// Where on screen the control doing `action` is: among the page's
/// controls, on its pinned strip, or a row of its long list.
fn spot(app: &mut App, f: &Fonts, i: &DeviceInfo, action: Action) -> (f32, f32) {
    let (t, scroll) = (app.theme(), app.top().1);
    let fr = app.frame(f, i);
    if let Some(h) = fr.hits.iter().find(|h| h.action == action) {
        let (x, y) = h.rect.centre();
        return (x, y - scroll);
    }
    if let Some(p) = &fr.pinned
        && let Some(h) = p.hits.iter().find(|h| h.action == action)
    {
        let (x, y) = h.rect.centre();
        return (x, y + pinned_top(&t, p, scroll));
    }
    if let Some(d) = &fr.dock
        && let Some(h) = d.hits.iter().find(|h| h.action == action)
    {
        let (x, y) = h.rect.centre();
        return (x, y + H as f32 - d.canvas.h as f32);
    }
    let rows = fr.rows.as_ref().expect("control on the page");
    let answer = rows.action.expect("control on the page");
    let k = (0..rows.count()).find(|&k| answer(k) == action).expect("control on the page");
    let (x, y) = rows.rect(k, fr.canvas.w as f32).centre();
    (x, y - scroll)
}
fn tap(app: &mut App, f: &Fonts, i: &DeviceInfo, action: Action) -> Vec<Effect> {
    tap_at(app, f, i, action, 0)
}
fn tap_at(app: &mut App, f: &Fonts, i: &DeviceInfo, action: Action, now: u64) -> Vec<Effect> {
    let (x, y) = spot(app, f, i, action);
    let mut fx = app.touch(Touch::Down(x, y), f, now);
    fx.extend(app.touch(Touch::Up, f, now));
    if matches!(action, Action::Type(_)) {
        fx.retain(|fx| *fx != Effect::Haptic);
    }
    fx
}
/// Tap the `<`, which stays at the top of the screen.
fn tap_back(app: &mut App, f: &Fonts, i: &DeviceInfo, now: u64) {
    app.frame(f, i);
    let (x, y) = widgets::back_rect(&app.theme()).centre();
    app.touch(Touch::Down(x, y), f, now);
    app.touch(Touch::Up, f, now);
}
/// Tap the round button at the right end of the bar.
fn tap_button(app: &mut App, f: &Fonts, i: &DeviceInfo, now: u64) -> Vec<Effect> {
    app.frame(f, i);
    let (x, y) = widgets::button_rect(&app.theme()).centre();
    let mut fx = app.touch(Touch::Down(x, y), f, now);
    fx.extend(app.touch(Touch::Up, f, now));
    fx
}
#[test]
fn rows_open_pages_and_back_returns() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Settings));
    assert_eq!(app.screen(), Screen::Settings);
    tap_back(&mut app, &f, &i, 0);
    assert_eq!(app.screen(), Screen::Home);
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Home, "back on the first page stays there");
}
#[test]
fn the_back_control_stays_put_while_the_page_scrolls() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Settings));
    app.frame(&f, &i);
    app.touch(Touch::Down(700.0, 2400.0), &f, 0);
    app.touch(Touch::Move(700.0, 2200.0), &f, 0);
    app.touch(Touch::Move(700.0, 2100.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    assert_eq!(app.top().1, 300.0);
    tap_back(&mut app, &f, &i, 0);
    assert_eq!(app.screen(), Screen::Home);
}
/// `screen` opened from the home page, so there is a page to go back
/// to.
fn opened(screen: Screen, f: &Fonts, i: &DeviceInfo) -> App {
    let mut app = App::new(Screen::Home, W, H, PPD);
    tap(&mut app, f, i, Action::Open(screen));
    app
}
#[test]
fn home_opens_the_mount_page() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Mount)),
        vec![Effect::ReadMounts],
        "the page shows the partitions as they are now"
    );
    assert_eq!(app.screen(), Screen::Mount);
}
/// A battery at `level`, as `charge` has it, 35.2 °C, on a charger
/// unless it discharges.
fn battery_at(level: u8, charge: Charge) -> skyward_info::Battery {
    let plugged = charge != Charge::Discharging;
    skyward_info::Battery {
        level,
        charge,
        temp: Some(352),
        health: None,
        cycles: None,
        design: None,
        full: None,
        voltage: None,
        quick: false,
        plugged,
    }
}
#[test]
fn the_home_card_tells_of_a_battery_too_low_to_install_on() {
    let f = fonts();
    let under = |i: &DeviceInfo| {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.frame(&f, i)
            .hits
            .iter()
            .find(|h| h.action == Action::Open(Screen::Install))
            .expect("an install row")
            .rect
            .y
    };
    let plain = under(&info());
    assert!(
        under(&DeviceInfo {
            battery: Some(battery_at(19, Charge::Discharging)),
            ..info()
        }) > plain,
        "short of 20% unplugged"
    );
    assert_eq!(
        under(&DeviceInfo {
            battery: Some(battery_at(20, Charge::Discharging)),
            ..info()
        }),
        plain
    );
    assert!(
        under(&DeviceInfo {
            battery: Some(battery_at(14, Charge::Charging)),
            ..info()
        }) > plain,
        "short of 15% charging"
    );
    assert_eq!(
        under(&DeviceInfo {
            battery: Some(battery_at(16, Charge::Charging)),
            ..info()
        }),
        plain
    );
    let (title, note) = fmt::battery_low(Lang::Hans, 14, true);
    assert_eq!((title.as_str(), note.as_str()), ("电量仅剩 14%", "充电至 15% 以上才能安装"));
}
#[test]
fn the_home_card_tells_of_a_restart_into_the_other_slot_and_opens_the_slot_page() {
    let f = fonts();
    let i = DeviceInfo {
        slot: Some(Slot::B),
        ..info()
    };
    let mut app = App::new(Screen::Home, W, H, PPD);
    let slots = |active| BootSlots {
        active,
        bootable: [true, true],
        successful: [true, true],
    };
    app.set_boot_slots(Ok(slots(Slot::B)));
    assert!(
        app.frame(&f, &i).hits.iter().all(|h| h.action != Action::Open(Screen::Slot)),
        "the slot it runs from starts next"
    );
    app.set_boot_slots(Ok(slots(Slot::A)));
    assert_eq!(tap(&mut app, &f, &i, Action::Open(Screen::Slot)), vec![Effect::ReadSlots]);
    assert_eq!(app.screen(), Screen::Slot);
    assert_eq!(
        fmt::next_boot(Lang::Hans, Slot::A, Slot::B),
        ("重启后从 A 槽位启动".into(), "当前运行的是 B 槽位".into())
    );
}
#[test]
fn the_status_bar_carries_the_slot_the_data_partitions_lock_and_the_temperature() {
    let i = DeviceInfo {
        slot: Some(Slot::B),
        battery: Some(battery_at(64, Charge::Charging)),
        ..info()
    };
    let mut app = App::new(Screen::Home, W, H, PPD);
    let bar = app.bar("12:00", &i);
    assert_eq!(
        (bar.slot, bar.temp, bar.battery),
        (Some("B"), Some(352), Some((64, Some(Bolt::Charging)))),
        "the temperature to the tenth"
    );
    let quick = DeviceInfo {
        battery: Some(skyward_info::Battery {
            quick: true,
            ..battery_at(64, Charge::Charging)
        }),
        ..info()
    };
    assert_eq!(
        app.bar("12:00", &quick).battery,
        Some((64, Some(Bolt::Quick))),
        "a quick charger draws the stock bar's quick bolt"
    );
    let out = DeviceInfo {
        battery: Some(battery_at(64, Charge::Discharging)),
        ..info()
    };
    assert_eq!(app.bar("12:00", &out).battery, Some((64, None)), "no bolt with the cable out");
    let lock = |app: &mut App, data: Data| {
        app.state.data = data;
        app.bar("12:00", &i).data
    };
    assert_eq!(
        lock(
            &mut app,
            Data::Locked {
                user: 0,
                keys: Keys::Digits,
                checking: false
            }
        ),
        Some(false),
        "shut while its files are out of reach"
    );
    assert_eq!(lock(&mut app, Data::Open), Some(true));
    assert_eq!(
        lock(
            &mut app,
            Data::Locked {
                user: 999,
                keys: Keys::Digits,
                checking: false
            }
        ),
        Some(true),
        "the owner's files are open behind a second space's lock"
    );
    assert_eq!(lock(&mut app, Data::Erased), None, "nothing to open once wiped");
    let none = app.bar("12:00", &info());
    assert_eq!((none.slot, none.temp), (None, None), "a phone without slots or a fuel gauge");
}
/// A screen as tall as the home page's rows need, with a little room to
/// spare. Measured from the page rather than written down, so that a row
/// added to the page moves it with them.
fn snug(f: &Fonts, i: &DeviceInfo) -> u32 {
    let mut app = App::new(Screen::Home, W, H * 2, PPD);
    let fr = app.frame(f, i);
    let last = fr.hits.iter().map(|h| h.rect.y + h.rect.h).fold(0.0f32, f32::max);
    (last + 8.0).ceil() as u32
}
#[test]
fn a_home_page_grown_by_an_update_notice_scrolls_down_to_its_last_row() {
    let f = fonts();
    let plain = info();
    let snug = snug(&f, &plain);
    let mut app = App::new(Screen::Home, W, snug, PPD);
    let fr = app.frame(&f, &plain);
    let reboot = fr
        .hits
        .iter()
        .find(|h| h.action == Action::Open(Screen::Reboot))
        .expect("a reboot row")
        .rect;
    assert!(reboot.y + reboot.h <= snug as f32, "without the notice the rows fit the screen");
    let i = DeviceInfo {
        snapshot: SnapshotState::Snapshotted,
        ..info()
    };
    let mut app = App::new(Screen::Home, W, snug, PPD);
    let fr = app.frame(&f, &i);
    let reboot = fr
        .hits
        .iter()
        .find(|h| h.action == Action::Open(Screen::Reboot))
        .expect("a reboot row")
        .rect;
    assert!(reboot.y + reboot.h > snug as f32, "the notice pushes the last row below the screen");
    assert!(reboot.y + reboot.h <= fr.canvas.h as f32, "the page is drawn down to it");
    app.touch(Touch::Down(700.0, 2800.0), &f, 0);
    app.touch(Touch::Move(700.0, 2000.0), &f, 0);
    app.touch(Touch::Move(700.0, 1200.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    for ms in (0..3000).step_by(8) {
        app.tick(ms);
    }
    let (_, y) = spot(&mut app, &f, &i, Action::Open(Screen::Reboot));
    assert!(y < snug as f32, "scrolled up into view");
    tap_at(&mut app, &f, &i, Action::Open(Screen::Reboot), 3000);
    assert_eq!(app.screen(), Screen::Reboot);
}
#[test]
fn the_cursor_blinks_and_comes_up_whole_at_each_edit() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Search, &f, &i);
    let mut screen = Canvas::new(W, H);
    let mut shows = |app: &mut App, now: u64| {
        app.compose(&mut screen, &f, &i, "12:00", now);
        app.caret_shown
    };
    assert!(shows(&mut app, 1000), "it comes up whole on the page");
    assert_eq!(app.deadline(1200), Some(1500), "the loop wakes when it turns");
    assert!(!shows(&mut app, 1500), "half a second on it hides");
    assert!(shows(&mut app, 2000));
    assert!(!shows(&mut app, 2600));
    tap(&mut app, &f, &i, Action::Type(Cap::Char('b')));
    assert!(shows(&mut app, 2700), "an edit brings it back whole at once");
    assert!(!shows(&mut app, 3200));
    tap(&mut app, &f, &i, Action::CloseSearch);
    assert_eq!(app.screen(), Screen::Home, "the round button leaves the search");
    assert!(!shows(&mut app, 3300), "no cursor off the page");
    assert!(
        app.deadline(3300).is_none_or(|d| d > 10_000),
        "nor a wake for it, only for the screen going off"
    );
}
#[test]
fn the_search_bar_moves_up_into_the_search_page_and_back_down() {
    let (f, i) = (fonts(), info());
    let t = Theme::light(W, H, PPD);
    let m = t.m;
    let mut app = App::new(Screen::Home, W, H, PPD);
    let mut screen = Canvas::new(W, H);
    let x = m.dp(300.0) as u32;
    let top = |screen: &Canvas| {
        (m.dp(40.0) as u32..H)
            .find(|&y| screen.data[((y * W + x) * 4) as usize] == t.pal.search.r)
            .map(|y| y as f32)
    };
    let grey = |screen: &Canvas, y: f32| screen.data[((y as u32 * W + W / 2) * 4) as usize];
    let (home, field) = (m.content_top, widgets::search_layout(&t).0.y);
    let card = m.content_top + m.search_h + m.after_search + m.dp(20.0);
    app.compose(&mut screen, &f, &i, "12:00", 0);
    assert_eq!(grey(&screen, card), t.pal.card.r);
    tap_at(&mut app, &f, &i, Action::Open(Screen::Search), 1000);
    assert_eq!(app.screen(), Screen::Search);
    assert_eq!(app.deadline(1000), Some(0), "it moves frame by frame");
    app.compose(&mut screen, &f, &i, "12:00", 1067);
    let along = crate::motion::settle(67, MORPH_PERIOD);
    let expected = home + (field - home) * along;
    let at = top(&screen).expect("the bar is on screen");
    assert!(
        (at - expected).abs() <= 2.0,
        "up {along} of the way after 67 ms: at {at}, not {expected}"
    );
    assert_eq!(grey(&screen, card), t.pal.bg.r, "the home page's cards go at once");
    app.compose(&mut screen, &f, &i, "12:00", 2500);
    assert!(app.morph.is_none(), "settled, the search page shows as itself");
    assert_eq!(top(&screen), Some(field.ceil()));
    tap_at(&mut app, &f, &i, Action::CloseSearch, 3000);
    assert_eq!(app.screen(), Screen::Home);
    app.compose(&mut screen, &f, &i, "12:00", 3067);
    let expected = field + (home - field) * along;
    let at = top(&screen).expect("the bar is on screen");
    assert!(
        (at - expected).abs() <= 2.0,
        "down {along} of the way after 67 ms: at {at}, not {expected}"
    );
    assert_eq!(grey(&screen, card + (at - home)), t.pal.card.r, "the cards come down under the bar");
    app.compose(&mut screen, &f, &i, "12:00", 4500);
    assert!(app.morph.is_none());
    assert_eq!(top(&screen), Some(home.ceil()));
}
/// A screen too short for the home page's rows, which scrolls well past
/// the fold of its search bar.
const SHORT: u32 = 2600;
#[test]
fn the_home_search_bar_folds_away_under_its_still_title_before_the_title_goes() {
    let (f, i) = (fonts(), info());
    let t = Theme::light(W, SHORT, PPD);
    let m = t.m;
    let mut app = App::new(Screen::Home, W, SHORT, PPD);
    let fold = app.frame(&f, &i).fold.expect("the home page's search bar folds");
    assert!(
        (fold.span - (m.search_h + m.after_search)).abs() < 1e-3,
        "the bar and the gap under it"
    );
    let mut screen = Canvas::new(W, SHORT);
    let x = m.dp(300.0) as u32;
    let px = |screen: &Canvas, y: u32| screen.data[((y * W + x) * 4) as usize];
    let bar = |screen: &Canvas| (0..SHORT).filter(|&y| px(screen, y) == t.pal.search.r).collect::<Vec<_>>();
    let card = |screen: &Canvas| {
        (fold.bar.y as u32..SHORT)
            .find(|&y| px(screen, y) == t.pal.card.r)
            .expect("the card shows")
    };
    let (top, bottom) = ((m.title_top - m.dp(14.0)) as u32, fold.bar.y as u32);
    let title = |screen: &Canvas| screen.crop(0, top, W, bottom - top).data.clone();
    app.compose(&mut screen, &f, &i, "12:00", 0);
    let (whole, still, first) = (bar(&screen), title(&screen), card(&screen));
    assert_eq!(
        (whole[0], *whole.last().expect("the bar shows")),
        (fold.bar.y.ceil() as u32, (fold.bar.y + fold.bar.h).floor() as u32 - 1)
    );
    let half = (fold.bar.h / 2.0).round();
    scroll_to(&mut app, &f, half, 1000);
    app.compose(&mut screen, &f, &i, "12:00", 1800);
    let rows = bar(&screen);
    assert_eq!(rows[0], whole[0], "its top stays");
    assert_eq!(
        *rows.last().expect("half the bar shows"),
        (fold.bar.y + fold.bar.h - half).floor() as u32 - 1,
        "its foot rises with the page"
    );
    assert_eq!(card(&screen), first - half as u32, "the content follows the finger");
    assert!(title(&screen) == still, "the title holds still");
    scroll_to(&mut app, &f, fold.span, 3000);
    app.compose(&mut screen, &f, &i, "12:00", 3800);
    assert!(bar(&screen).is_empty(), "folded away");
    assert_eq!(card(&screen), first - fold.span.round() as u32, "the gap under it closed");
    assert!(title(&screen) == still, "the title holds still till then");
    scroll_to(&mut app, &f, fold.span + m.dp(10.0), 5000);
    app.compose(&mut screen, &f, &i, "12:00", 5800);
    assert!(title(&screen) != still, "then it goes up with the page");
    assert!(!app.title.on, "the bar's title waits for the large one to fade");
    scroll_to(&mut app, &f, fold.span + m.dp(TITLE_FADE_DP) + 2.0, 7000);
    assert!(app.title.on);
}
#[test]
fn a_home_page_let_go_with_its_search_bar_part_folded_folds_it_whole_or_not_at_all() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, SHORT, PPD);
    let span = app.frame(&f, &i).fold.expect("the home page's search bar folds").span;
    let settle = |app: &mut App, from: u64| {
        for ms in (from..from + 2000).step_by(8) {
            app.tick(ms);
        }
        app.top().1
    };
    scroll_to(&mut app, &f, span * (FOLD_SETTLE - 0.1), 0);
    assert_eq!(settle(&mut app, 800), 0.0, "short of the mark it unfolds");
    scroll_to(&mut app, &f, span * (FOLD_SETTLE + 0.1), 3000);
    assert_eq!(settle(&mut app, 3800), span, "past it it folds away");
    scroll_to(&mut app, &f, span + 50.0, 6000);
    assert_eq!(settle(&mut app, 6800), span + 50.0, "a page scrolled past the fold stays");
    scroll_to(&mut app, &f, span + 86.0, 8000);
    app.touch(Touch::Down(10.0, 1000.0), &f, 9000);
    for k in 1..=7u64 {
        app.touch(Touch::Move(10.0, 1000.0 + 8.0 * k as f32), &f, 9000 + 8 * k);
    }
    app.touch(Touch::Up, &f, 9056);
    let glide = app.fling.expect("it glides");
    let (rest, _) = glide.at(u64::MAX);
    assert!(rest > 0.0 && rest < span, "this one comes to rest in the fold: {rest}");
    let folded = if rest < span * FOLD_SETTLE { 0.0 } else { span };
    assert_eq!(settle(&mut app, 9056), folded, "it settles there the same way");
    scroll_to(&mut app, &f, 0.0, 12_000);
    app.touch(Touch::Down(10.0, 1000.0), &f, 13_000);
    for k in 1..=5u64 {
        app.touch(Touch::Move(10.0, 1000.0 - 30.0 * k as f32), &f, 13_000 + 8 * k);
    }
    for k in 1..=14u64 {
        app.touch(Touch::Move(10.0, 850.0 + 2.0 * k as f32), &f, 13_040 + 8 * k);
    }
    app.touch(Touch::Up, &f, 13_152);
    assert_eq!(app.top().1, 122.0);
    assert!(app.fling.is_none(), "no glide");
    assert_eq!(app.seek.map(|(rest, _)| rest), Some(span), "it was let go past the mark");
    assert_eq!(settle(&mut app, 13_152), span);
    scroll_to(&mut app, &f, 0.0, 16_000);
    app.touch(Touch::Down(10.0, 1000.0), &f, 17_000);
    for k in 1..=4u64 {
        app.touch(Touch::Move(10.0, 1000.0 - 30.0 * k as f32), &f, 17_000 + 8 * k);
    }
    app.touch(Touch::Up, &f, 17_032);
    let (end, _) = app.fling.expect("it glides").at(u64::MAX);
    assert!(end > span, "on past the fold: {end}");
}
#[test]
fn a_search_bar_folded_away_answers_no_touch() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let fold = app.frame(&f, &i).fold.expect("the home page's search bar folds");
    scroll_to(&mut app, &f, fold.span, 0);
    app.tick(3000);
    let (x, y) = fold.bar.centre();
    app.touch(Touch::Down(x, y - fold.span), &f, 3000);
    assert_eq!(app.pressing(), None, "nothing shows held");
    app.touch(Touch::Up, &f, 3000);
    assert_eq!(app.screen(), Screen::Home);
}
#[test]
fn the_keys_unfold_the_search_bar_to_light_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let span = app.frame(&f, &i).fold.expect("the home page's search bar folds").span;
    scroll_to(&mut app, &f, span, 0);
    app.tick(3000);
    press(&mut app, Key::VolumeDown, 3000);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::About))),
        "a bar folded away is not in view"
    );
    press(&mut app, Key::VolumeUp, 3000);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::Search))));
    for ms in (3000..5000).step_by(8) {
        app.tick(ms);
    }
    assert_eq!(app.top().1, 0.0, "the page went back up to unfold it");
    press(&mut app, Key::Power, 5000);
    assert_eq!(app.screen(), Screen::Search);
}
#[test]
fn a_search_bar_pulled_down_past_the_top_comes_down_whole_with_its_title() {
    let fold = Fold {
        bar: Rect::new(36.0, 503.0, 1128.0, 131.0),
        hint: "",
        span: 169.0,
    };
    assert_eq!(fold.shown(-30.0), Some(Rect::new(36.0, 533.0, 1128.0, 131.0)));
    assert_eq!(fold.lift(-30.0), -30.0);
    assert_eq!(fold.shown(31.0), Some(Rect::new(36.0, 503.0, 1128.0, 100.0)));
    assert_eq!(fold.shown(131.0), None);
    assert_eq!((fold.lift(150.0), fold.lift(200.0)), (0.0, 31.0));
}
/// The grey of the screen's pixel at (`x`, `y`).
fn grey_at(c: &Canvas, x: f32, y: f32) -> u8 {
    c.data[((y as u32 * c.w + x as u32) * 4) as usize]
}
/// The darkest grey of the screen within `r`.
fn darkest(c: &Canvas, r: Rect) -> u8 {
    let (x0, y0, x1, y1) = (r.x as u32, r.y as u32, (r.x + r.w) as u32, (r.y + r.h) as u32);
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| c.data[((y * c.w + x) * 4) as usize]))
        .min()
        .expect("a part of the screen")
}
/// A copy of the screen.
fn shot(c: &Canvas) -> Canvas {
    c.clone()
}
/// How a slide opening a page, or going `back`, stands `ms` into it:
/// where the page in front stands, how far left the page behind is, and
/// how much of its light the page behind keeps, in 256ths.
fn slid(back: bool, ms: u64) -> (u32, u32, u32) {
    let (x, _) = if back {
        Spring::new(0, -1000.0, 0.0, BACK_PERIOD)
    } else {
        Spring::new(0, 1000.0, -OPEN_SPEED * 1000.0, OPEN_PERIOD)
    }
    .at(ms);
    let along = if back { 1.0 + x / 1000.0 } else { x / 1000.0 };
    let front = (along * W as f32).round() as u32;
    let behind = (BEHIND_SHIFT * W as f32 * (1.0 - along)).round() as u32;
    let keep = ((1.0 - BEHIND_DIM * (1.0 - along)) * 256.0).round() as u32;
    (front, behind, keep)
}
/// Check row `y` of `mid`: the page `ahead` from `front` on, and left of
/// it the page `under`, `behind` pixels left of rest and keeping `keep`
/// 256ths of its light.
fn check_slide(mid: &Canvas, ahead: &Canvas, under: &Canvas, y: u32, (front, behind, keep): (u32, u32, u32)) {
    for x in 0..W {
        let at = |c: &Canvas, x: u32| c.data[((y * W + x) * 4) as usize] as u32;
        let want = if x >= front {
            at(ahead, x - front)
        } else {
            (at(under, x + behind) * keep) >> 8
        };
        let got = at(mid, x);
        assert!(
            got.abs_diff(want) <= 1,
            "column {x} of row {y}: {got}, not {want} (front {front}, behind {behind})"
        );
    }
}
#[test]
fn a_page_opened_slides_in_over_the_page_it_opens_from_and_back_off_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:00", 0);
    let home = shot(&c);
    tap_at(&mut app, &f, &i, Action::Open(Screen::Mount), 1000);
    assert_eq!(app.screen(), Screen::Mount);
    assert_eq!(app.deadline(1000), Some(0), "it slides frame by frame");
    app.compose(&mut c, &f, &i, "12:00", 1000);
    let first = shot(&c);
    app.compose(&mut c, &f, &i, "12:00", 1050);
    let mid = shot(&c);
    app.compose(&mut c, &f, &i, "12:00", 3000);
    assert!(app.turn.is_none(), "come to rest, the page left is let go");
    assert_ne!(app.deadline(3000), Some(0));
    let mount = shot(&c);
    for y in [H / 3, H / 2, H - 200] {
        check_slide(&first, &mount, &home, y, (W, 0, 256));
        check_slide(&mid, &mount, &home, y, slid(false, 50));
    }
    let (front, _, _) = slid(false, 50);
    assert!(
        (front as f32 / W as f32 - 0.728).abs() < 0.01,
        "about three quarters of the way in after 50 ms: {front}"
    );
    tap_back(&mut app, &f, &i, 4000);
    assert_eq!(app.screen(), Screen::Home);
    app.compose(&mut c, &f, &i, "12:00", 4000);
    let first = shot(&c);
    app.compose(&mut c, &f, &i, "12:00", 4060);
    let mid = shot(&c);
    app.compose(&mut c, &f, &i, "12:00", 6000);
    assert!(app.turn.is_none());
    let home = shot(&c);
    for y in [H / 3, H / 2, H - 200] {
        check_slide(&first, &mount, &home, y, (0, W / 4, ((1.0 - BEHIND_DIM) * 256.0).round() as u32));
        check_slide(&mid, &mount, &home, y, slid(true, 60));
    }
}
#[test]
fn a_page_sliding_has_its_leading_corners_rounded_as_the_screens_are() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.set_screen_corners(140.0);
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:00", 0);
    let home = shot(&c);
    tap_at(&mut app, &f, &i, Action::Open(Screen::Mount), 1000);
    app.compose(&mut c, &f, &i, "12:00", 1000);
    app.compose(&mut c, &f, &i, "12:00", 1050);
    let mid = shot(&c);
    app.compose(&mut c, &f, &i, "12:00", 3000);
    let mount = shot(&c);
    let (front, behind, keep) = slid(false, 50);
    let at = |c: &Canvas, x: u32, y: u32| c.data[((y * W + x) * 4) as usize] as u32;
    let y = H - 3;
    assert_eq!(
        at(&mid, front + 5, y),
        (at(&home, front + 5 + behind, y) * keep) >> 8,
        "the corner's tip shows the page behind"
    );
    assert_eq!(
        at(&mid, front + 140, y),
        at(&mount, 140, y),
        "past the corner's curve the page in front is whole"
    );
    assert_eq!(
        at(&mid, front + 5, H / 2),
        at(&mount, 5, H / 2),
        "between the corners its edge is straight"
    );
}
#[test]
fn a_finger_brings_a_sliding_page_to_rest_and_a_page_never_shown_gives_way_at_once() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:00", 0);
    tap_at(&mut app, &f, &i, Action::Open(Screen::Mount), 1000);
    app.compose(&mut c, &f, &i, "12:00", 1000);
    app.touch(Touch::Down(W as f32 / 2.0, H as f32 / 2.0), &f, 1030);
    assert!(app.turn.is_none(), "a finger brings it to rest");
    app.touch(Touch::Up, &f, 1060);
    app.refresh();
    tap_back(&mut app, &f, &i, 2000);
    assert_eq!(app.screen(), Screen::Home);
    assert!(app.turn.is_none());
}
#[test]
fn a_row_held_darkens_behind_its_words_a_moment_after_the_finger_lands() {
    let (f, i) = (fonts(), info());
    let t = Theme::light(W, H, PPD);
    let mut app = App::new(Screen::Home, W, H, PPD);
    let mut c = Canvas::new(W, H);
    let row = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Open(Screen::Backup))
        .expect("on the home page")
        .rect;
    let (x, y) = (W as f32 * 0.6, row.y + row.h / 2.0);
    app.compose(&mut c, &f, &i, "12:00", 0);
    let ink = darkest(&c, row);
    assert_eq!(grey_at(&c, x, y), t.pal.card.r);
    app.touch(Touch::Down(x, y), &f, 1000);
    let rise = 1000 + ROW_PRESS_DELAY_MS;
    assert_eq!(app.deadline(1000), Some(rise), "nothing moves until the row starts darkening");
    app.compose(&mut c, &f, &i, "12:00", 1000);
    assert!(
        app.row_art.is_none(),
        "a finger that may yet scroll does not wait for the row drawn held down"
    );
    app.compose(&mut c, &f, &i, "12:00", rise);
    assert_eq!(grey_at(&c, x, y), t.pal.card.r, "it has not started yet");
    app.compose(&mut c, &f, &i, "12:00", rise + 30);
    let partway = grey_at(&c, x, y);
    assert!(partway < 255 && partway > 225, "darkening after 30 ms: {partway}");
    app.compose(&mut c, &f, &i, "12:00", 2000);
    assert_eq!(grey_at(&c, x, y), 225, "as dark as the stock settings' row held down");
    assert_eq!(darkest(&c, row), ink, "its words keep their ink");
    assert_ne!(app.deadline(2000), Some(0), "held still, nothing moves");
    app.touch(Touch::Move(x, y + t.m.dp(30.0)), &f, 2100);
    assert_eq!(app.deadline(2100), Some(0));
    app.touch(Touch::Up, &f, 2200);
    app.tick(3000);
    assert!(app.row_press.is_none() && app.row_art.is_none(), "faded, it is let go");
    assert_eq!(app.screen(), Screen::Home, "a scroll opens nothing");
}
#[test]
fn a_row_scrolled_away_from_before_it_darkens_never_shows_held() {
    let (f, i) = (fonts(), info());
    let t = Theme::light(W, H, PPD);
    let mut app = App::new(Screen::Home, W, H, PPD);
    let row = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Open(Screen::Backup))
        .expect("on the home page")
        .rect;
    let (x, y) = (W as f32 * 0.6, row.y + row.h / 2.0);
    app.touch(Touch::Down(x, y), &f, 1000);
    app.touch(Touch::Move(x, y + t.m.dp(30.0)), &f, 1050);
    assert!(app.row_press.is_none());
}
#[test]
fn a_row_tapped_quickly_shows_held_from_the_lift_and_fades_back() {
    let (f, i) = (fonts(), info());
    let t = Theme::light(W, H, PPD);
    let mut app = App::new(Screen::Settings, W, H, PPD);
    let mut c = Canvas::new(W, H);
    let row = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Toggle(Switch::Vibrate))
        .expect("on the settings page")
        .rect;
    let (x, y) = (W as f32 * 0.6, row.y + row.h / 2.0);
    tap_at(&mut app, &f, &i, Action::Toggle(Switch::Vibrate), 1000);
    assert!(!app.state.vibrate, "the tap worked the switch");
    app.compose(&mut c, &f, &i, "12:00", 1040);
    let held = grey_at(&c, x, y);
    assert!(held < t.pal.card.r, "held from the lift: {held}");
    app.tick(1040 + QUICK_TAP_MS + 1000);
    assert!(app.row_press.is_none(), "and let go once faded");
    app.compose(&mut c, &f, &i, "12:00", 2200);
    assert_eq!(grey_at(&c, x, y), t.pal.card.r);
}
#[test]
fn the_back_control_and_the_search_buttons_darken_at_once_under_a_finger() {
    let (f, i) = (fonts(), info());
    let t = Theme::light(W, H, PPD);
    let mut c = Canvas::new(W, H);
    let mut app = opened(Screen::Mount, &f, &i);
    let (cx, cy, r) = widgets::back_disc(&t);
    let (bx, by) = widgets::back_rect(&t).centre();
    let around = Rect::new(cx - r, cy - r, r * 2.0, r * 2.0);
    app.compose(&mut c, &f, &i, "12:00", 5000);
    assert_eq!(grey_at(&c, cx + r * 0.6, cy), t.pal.bg.r);
    assert_eq!(darkest(&c, around), 0);
    app.touch(Touch::Down(bx, by), &f, 6000);
    app.compose(&mut c, &f, &i, "12:00", 6000);
    let disc = over(t.pal.button_press, t.pal.bg).r;
    assert_eq!(grey_at(&c, cx + r * 0.6, cy), disc, "darker at once");
    let arrow = darkest(&c, around);
    assert!(arrow.abs_diff(85) <= 2, "the stock `<` held down is 85, not {arrow}");
    app.touch(Touch::Move(bx, by + t.m.dp(40.0)), &f, 6050);
    app.compose(&mut c, &f, &i, "12:00", 6050);
    assert_eq!(grey_at(&c, cx + r * 0.6, cy), t.pal.bg.r);
    app.touch(Touch::Up, &f, 6100);
    assert_eq!(app.screen(), Screen::Mount);
    let mut app = App::new(Screen::Search, W, H, PPD);
    app.frame(&f, &i);
    let (_, button) = widgets::search_layout(&t);
    let (x, y) = button.centre();
    app.touch(Touch::Down(x, y), &f, 1000);
    app.compose(&mut c, &f, &i, "12:00", 1000);
    assert_eq!(grey_at(&c, x, y + button.h * 0.35), over(t.pal.button_press, t.pal.search).r);
    let cross = darkest(&c, button);
    assert!(cross.abs_diff(70) <= 2, "the stock cross held down is 70, not {cross}");
    let mut app = App::new(Screen::Home, W, H, PPD);
    let bar = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Open(Screen::Search))
        .expect("on the home page")
        .rect;
    let (x, y) = (bar.x + bar.w * 0.8, bar.y + bar.h / 2.0);
    app.touch(Touch::Down(x, y), &f, 1000);
    app.compose(&mut c, &f, &i, "12:00", 1000);
    assert_eq!(grey_at(&c, x, y), over(t.pal.button_press, t.pal.search).r);
}
#[test]
fn an_unreadable_partition_table_leaves_nothing_to_toggle() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Mount, &f, &i);
    app.set_mounts(Err("read the metadata in super: Input/output error (os error 5)".into()));
    assert!(app.frame(&f, &i).hits.is_empty());
}
#[test]
fn avb_switches_only_write_after_confirmation_and_never_enter_saved_settings() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    let saved = app.settings();
    assert_eq!(tap(&mut app, &f, &i, Action::Open(Screen::Avb)), vec![Effect::ReadAvb]);
    let state = avbctl::State {
        suffix: "_a".into(),
        flags: 0,
        unlocked: true,
        digest: [0; 32],
    };
    app.avb_read(Ok(state.clone()), None);
    assert!(tap(&mut app, &f, &i, Action::AvbToggle(avbctl::Check::Verification)).is_empty());
    assert_eq!(app.settings(), saved);
    assert_eq!(
        app.state.avb,
        Some(Ok(state.clone())),
        "a pending switch does not claim a disk change"
    );
    tap(&mut app, &f, &i, Action::CancelAvb);
    assert!(app.state.avb_pending.is_none());
    tap(&mut app, &f, &i, Action::AvbToggle(avbctl::Check::Verification));
    tap(&mut app, &f, &i, Action::AvbConfirmOpen);
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::AvbDisable),
        vec![Effect::SetAvb {
            expected: state,
            check: avbctl::Check::Verification,
            disabled: true
        }]
    );
    assert!(app.state.avb_pending.is_none());
}
#[test]
fn unknown_or_locked_avb_state_offers_no_write_controls() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Avb, W, H, PPD);
    for state in [
        Err("cannot read vbmeta".into()),
        Ok(avbctl::State {
            suffix: "_a".into(),
            flags: 3,
            unlocked: false,
            digest: [0; 32],
        }),
    ] {
        app.avb_read(state, None);
        assert!(app.frame(&f, &i).hits.is_empty());
        assert!(app.confirmed(Confirm::Avb, 0).is_empty());
    }
}
#[test]
fn partitions_mount_while_an_update_waits_to_merge() {
    let (f, mut i) = (fonts(), info());
    i.snapshot = SnapshotState::Snapshotted;
    let mut app = opened(Screen::Mount, &f, &i);
    app.set_mounts(Ok(vec![
        MountRow {
            base: "vendor".into(),
            point: "/vendor".into(),
            mounted: false,
            holders: Vec::new(),
        },
        MountRow {
            base: "odm".into(),
            point: "/odm".into(),
            mounted: true,
            holders: Vec::new(),
        },
    ]));
    let frame = app.frame(&f, &i);
    assert_eq!(
        frame.hits.iter().map(|h| h.action).collect::<Vec<_>>(),
        [Action::MountToggle(0), Action::MountToggle(1)]
    );
    assert_eq!(
        tap(&mut app, &f, &i, Action::MountToggle(0)),
        vec![Effect::Mount("vendor".into())],
        "through its snapshot"
    );
}
#[test]
fn an_update_waiting_in_the_slot_merges_with_a_swipe_and_shows_how_far_it_has_come() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    app.set_update(Standing::Waiting);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Update)),
        vec![Effect::ReadUpdate],
        "the page reads where the update stands"
    );
    assert_eq!(app.screen(), Screen::Update);
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Merge), vec![Effect::Merge]);
    assert_eq!(app.state.update, Standing::Merging(0.0), "the page turns to the merge at once");
    assert!(app.frame(&f, &i).hits.is_empty(), "nothing to swipe while it merges");
    app.set_update(Standing::Merging(0.5));
    assert!(app.frame(&f, &i).hits.is_empty());
    app.set_update(Standing::Merged);
    assert!(app.frame(&f, &i).hits.is_empty(), "merged, nothing is left to do here");
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Advanced);
    app.set_update(Standing::Installed(Slot::A));
    tap(&mut app, &f, &i, Action::Open(Screen::Update));
    assert!(
        app.frame(&f, &i).hits.is_empty(),
        "an update in the other slot merges once that slot has started"
    );
}
#[test]
fn the_home_pages_word_on_a_waiting_update_opens_its_page() {
    let (f, mut i) = (fonts(), info());
    i.snapshot = SnapshotState::Snapshotted;
    let mut app = App::new(Screen::Home, W, H, PPD);
    assert_eq!(tap(&mut app, &f, &i, Action::Open(Screen::Update)), vec![Effect::ReadUpdate]);
    assert_eq!(app.screen(), Screen::Update);
}
#[test]
fn the_mount_page_toggles_each_partition() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Mount, &f, &i);
    app.set_mounts(Ok(vec![
        MountRow {
            base: "system".into(),
            point: "/mnt/system".into(),
            mounted: true,
            holders: Vec::new(),
        },
        MountRow {
            base: "vendor".into(),
            point: "/vendor".into(),
            mounted: false,
            holders: Vec::new(),
        },
    ]));
    let frame = app.frame(&f, &i);
    assert!(
        frame.hits.iter().any(|h| h.action == Action::MountToggle(0)),
        "the first partition has a toggle"
    );
    assert!(
        frame.hits.iter().any(|h| h.action == Action::MountToggle(1)),
        "the second partition has a toggle"
    );
    assert!(
        !frame.hits.iter().any(|h| matches!(h.action, Action::Toggle(_))),
        "partitions mount read-only"
    );
    assert_eq!(app.act(Action::MountToggle(0), 0), vec![Effect::Unmount("system".into())]);
    assert_eq!(app.act(Action::MountToggle(1), 0), vec![Effect::Mount("vendor".into())]);
}
#[test]
fn a_partition_in_use_cannot_be_unmounted() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Mount, &f, &i);
    let held = vec!["touch_report".to_string(), "touchsensor".to_string()];
    app.set_mounts(Ok(vec![
        MountRow {
            base: "odm".into(),
            point: "/odm".into(),
            mounted: true,
            holders: held,
        },
        MountRow {
            base: "vendor".into(),
            point: "/vendor".into(),
            mounted: true,
            holders: Vec::new(),
        },
    ]));
    let frame = app.frame(&f, &i);
    assert!(
        matches!(frame.hits[0].action, Action::Unavailable(_)),
        "the programs using odm keep it"
    );
    assert_eq!(frame.hits[1].action, Action::MountToggle(1));
    let busy = frame.hits[0].action;
    assert!(tap(&mut app, &f, &i, busy).is_empty());
    app.tick(1000);
    assert_eq!(app.deadline(1000), Some(TOAST_MS), "it says why");
}
#[test]
fn a_failed_mount_stays_on_the_page_until_the_next_try() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Mount, &f, &i);
    app.set_mounts(Ok(vec![MountRow {
        base: "vendor".into(),
        point: "/vendor".into(),
        mounted: false,
        holders: Vec::new(),
    }]));
    let failure = MountFailure {
        mounting: true,
        base: "vendor".into(),
        detail: "mount vendor_b: no filesystem".into(),
    };
    app.mount_failed(failure.clone());
    assert_eq!(app.state.mount_failure, Some(failure.clone()));
    assert_eq!(app.act(Action::MountToggle(0), 0), vec![Effect::Mount("vendor".into())]);
    assert_eq!(app.state.mount_failure, None, "trying again clears it");
    app.mount_failed(failure);
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Home);
    assert_eq!(app.state.mount_failure, None, "leaving the page clears it");
}
#[test]
fn the_mount_page_notes_when_there_is_nothing_to_mount() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Mount, &f, &i);
    let frame = app.frame(&f, &i);
    assert!(frame.hits.is_empty(), "no partition rows, and nothing else to touch");
}
#[test]
fn swipe_from_the_left_edge_goes_back() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Reboot, &f, &i);
    app.touch(Touch::Down(10.0, 1600.0), &f, 0);
    app.touch(Touch::Move(90.0, 1605.0), &f, 0);
    app.touch(Touch::Move(320.0, 1610.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    assert_eq!(app.screen(), Screen::Home);
}
#[test]
fn a_short_edge_swipe_does_not_go_back() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Reboot, &f, &i);
    app.touch(Touch::Down(10.0, 1600.0), &f, 0);
    app.touch(Touch::Move(80.0, 1600.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    assert_eq!(app.screen(), Screen::Reboot);
}
#[test]
fn the_first_page_has_no_edge_swipe_back() {
    let f = fonts();
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.touch(Touch::Down(10.0, 1600.0), &f, 0);
    app.touch(Touch::Move(80.0, 1600.0), &f, 8);
    assert!(app.touch(Touch::Move(400.0, 1600.0), &f, 16).is_empty(), "no tap of the motor");
    assert!(!matches!(app.drag, Drag::Edge { .. }), "no cue");
    app.touch(Touch::Up, &f, 24);
    assert_eq!(app.screen(), Screen::Home);
    assert_eq!(app.deadline(24), app.idle_due(), "nothing left on screen to settle");
}
#[test]
fn the_back_cue_taps_once_as_it_arms_and_sinks_when_let_go() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Reboot, &f, &i);
    let ready = app.theme().m.dp(BACK_SWIPE_DP);
    app.touch(Touch::Down(10.0, 1600.0), &f, 0);
    assert!(app.touch(Touch::Move(80.0, 1600.0), &f, 8).is_empty());
    assert_eq!(app.touch(Touch::Move(20.0 + ready, 1600.0), &f, 16), vec![Effect::Haptic]);
    assert!(
        app.touch(Touch::Move(60.0 + ready, 1600.0), &f, 24).is_empty(),
        "one tap while armed"
    );
    app.touch(Touch::Move(40.0, 1600.0), &f, 32);
    app.touch(Touch::Up, &f, 40);
    assert_eq!(app.screen(), Screen::Reboot, "pulled back in, it does not go back");
    assert_eq!(app.deadline(40), Some(0), "the cue is sinking back");
    app.tick(1000);
    assert_eq!(app.deadline(1000), app.idle_due());
}
#[test]
fn the_back_cue_stays_quiet_with_touch_vibration_off() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Reboot, &f, &i);
    app.state.vibrate = false;
    app.touch(Touch::Down(10.0, 1600.0), &f, 0);
    app.touch(Touch::Move(80.0, 1600.0), &f, 8);
    assert!(app.touch(Touch::Move(400.0, 1600.0), &f, 16).is_empty());
}
#[test]
fn the_back_cue_keeps_to_the_height_the_swipe_began_at() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Reboot, &f, &i);
    let at = 1600.0 - app.theme().m.dp(CUE_LIFT_DP);
    app.touch(Touch::Down(10.0, 1600.0), &f, 0);
    app.touch(Touch::Move(80.0, 1610.0), &f, 8);
    app.touch(Touch::Move(200.0, 1300.0), &f, 16);
    app.touch(Touch::Move(150.0, 1900.0), &f, 24);
    let Drag::Edge { cy, .. } = app.drag else {
        panic!("still swiping in from the edge")
    };
    assert_eq!(cy, at, "the finger wandered up and down");
    app.touch(Touch::Move(40.0, 2100.0), &f, 32);
    app.touch(Touch::Up, &f, 40);
    assert_eq!(app.cue.as_ref().map(|c| c.1), Some(at), "it sinks back where it was");
}
#[test]
fn pulling_past_the_top_resists_and_springs_back() {
    let f = fonts();
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    app.touch(Touch::Down(700.0, 1000.0), &f, 0);
    app.touch(Touch::Move(700.0, 1300.0), &f, 100);
    app.touch(Touch::Move(700.0, 1800.0), &f, 600);
    let shown = app.overscroll(600);
    assert!(shown < 0.0 && shown > -700.0, "the page follows less than the finger: {shown}");
    assert_eq!(app.top().1, 0.0);
    app.touch(Touch::Move(700.0, 1400.0), &f, 1000);
    assert!(app.overscroll(1000) > shown);
    app.touch(Touch::Up, &f, 1400);
    assert!(app.overscroll(1400) < 0.0);
    assert_eq!(app.deadline(1400), Some(0));
    app.tick(1500);
    assert!(app.overscroll(1500) > app.overscroll(1400), "on its way back");
    app.tick(3000);
    assert_eq!(app.overscroll(3000), 0.0);
    assert_eq!(app.deadline(3000), app.idle_due());
}
#[test]
fn a_page_pulled_past_its_end_can_be_caught() {
    let f = fonts();
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    app.touch(Touch::Down(700.0, 1000.0), &f, 0);
    app.touch(Touch::Move(700.0, 1500.0), &f, 500);
    app.touch(Touch::Up, &f, 1000);
    app.tick(1050);
    let held = app.overscroll(1050);
    app.touch(Touch::Down(700.0, 1200.0), &f, 1050);
    app.tick(1500);
    assert!((app.overscroll(1500) - held).abs() < 1.0, "the finger holds it where it was");
    app.touch(Touch::Up, &f, 1600);
    app.tick(4000);
    assert_eq!(app.overscroll(4000), 0.0);
}
#[test]
fn the_always_on_switch_shows_only_on_a_panel_that_dozes_and_turns_it() {
    let (f, i) = (fonts(), info());
    let offered = |app: &mut App| app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Toggle(Switch::AlwaysOn));
    let mut app = App::new(Screen::Settings, W, H, PPD);
    assert!(
        !offered(&mut app),
        "a panel with no doze of its own has nothing to keep on the glass"
    );
    app.state.panel_dozes = true;
    app.refresh();
    assert!(offered(&mut app), "the switch is among the page's controls");
    assert!(!app.state.always_on, "the face is off until it is turned on");
    app.act(Action::Toggle(Switch::AlwaysOn), 1000);
    assert!(app.state.always_on, "the switch turns it on");
    app.act(Action::Toggle(Switch::AlwaysOn), 2000);
    assert!(!app.state.always_on, "and off again");
}
#[test]
fn the_avb_page_reads_before_it_shows_and_only_writes_after_the_swipe() {
    use skyward_install::commands::avbctl;
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    assert_eq!(app.act(Action::Open(Screen::Avb), 0), vec![Effect::ReadAvb]);
    let state = avbctl::State {
        suffix: "_a".into(),
        flags: 0,
        unlocked: true,
        digest: [0; 32],
    };
    app.avb_read(Ok(state.clone()), None);
    let saved = app.settings();
    assert!(app.act(Action::AvbToggle(avbctl::Check::Verification), 1000).is_empty());
    assert_eq!(app.settings(), saved, "a pending change is not a saved setting");
    assert_eq!(app.state.avb, Some(Ok(state.clone())), "and does not claim a disk change");
    assert_eq!(app.state.avb_pending, Some((state.clone(), avbctl::Check::Verification, true)));
    app.act(Action::CancelAvb, 2000);
    assert!(app.state.avb_pending.is_none(), "the request can be taken back");
    app.act(Action::AvbToggle(avbctl::Check::Verification), 3000);
    app.frame(&f, &i);
    assert!(
        !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Confirm(Confirm::Avb)),
        "a disable is never one swipe away"
    );
    app.act(Action::AvbConfirmOpen, 3500);
    assert!(app.state.avb_confirming, "the row opens the second step");
    assert_eq!(
        app.state.avb_pending,
        Some((state.clone(), avbctl::Check::Verification, true)),
        "and the request is only a request still"
    );
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::AvbDisable),
        vec![Effect::SetAvb {
            expected: state,
            check: avbctl::Check::Verification,
            disabled: true
        }],
        "the second step's swipe is the one that writes it"
    );
    assert!(app.state.avb_pending.is_none(), "and the request is spent");
    assert!(!app.state.avb_confirming, "and the second step is over");
}
#[test]
fn turning_a_check_on_keeps_the_single_swipe_because_it_only_checks_more() {
    use skyward_install::commands::avbctl;
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Avb, W, H, PPD);
    let state = avbctl::State {
        suffix: "_a".into(),
        flags: 0b11,
        unlocked: true,
        digest: [0; 32],
    };
    app.avb_read(Ok(state.clone()), None);
    assert!(app.act(Action::AvbToggle(avbctl::Check::Verification), 1000).is_empty());
    assert_eq!(app.state.avb_pending, Some((state.clone(), avbctl::Check::Verification, false)));
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::Avb),
        vec![Effect::SetAvb {
            expected: state,
            check: avbctl::Check::Verification,
            disabled: false
        }],
        "turning one on is the one direction that writes from a single swipe"
    );
    assert!(!app.state.avb_confirming, "and it never asks twice");
}
#[test]
fn an_avb_state_that_could_not_be_read_or_is_locked_offers_no_write() {
    use skyward_install::commands::avbctl;
    let locked = avbctl::State {
        suffix: "_a".into(),
        flags: 0,
        unlocked: false,
        digest: [0; 32],
    };
    for unreadable in [Err("cannot read vbmeta".to_string()), Ok(locked)] {
        let mut app = App::new(Screen::Avb, W, H, PPD);
        app.avb_read(unreadable, None);
        app.act(Action::AvbToggle(avbctl::Check::Verity), 1000);
        assert!(app.state.avb_pending.is_none(), "a state the page cannot act on reaches no write");
        app.act(Action::Confirm(Confirm::Avb), 2000);
        assert!(app.state.avb_pending.is_none());
    }
}
#[test]
fn installing_a_rom_honours_the_avb_switch_and_it_is_saved() {
    let mut app = App::new(Screen::Install, W, H, PPD);
    assert!(!app.state.disable_avb, "a ROM is installed without turning the checks off");
    assert!(app.act(Action::Toggle(Switch::DisableAvb), 1000).is_empty());
    assert!(app.state.disable_avb);
    let saved = crate::settings::Settings::of(&app.state);
    let mut next = State::default();
    saved.apply(&mut next);
    assert!(next.disable_avb, "and the choice is kept for the next start");
}
#[test]
fn turning_remembering_off_forgets_the_credential_it_held() {
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.state.remember_password = true;
    app.state.remembered = Some("1234".into());
    assert_eq!(
        app.act(Action::Toggle(Switch::RememberPassword), 1000),
        vec![Effect::RememberPassword(None)]
    );
    assert!(app.state.remembered.is_none(), "the credential goes with the switch");
    assert!(!app.state.remember_password);
}
#[test]
fn the_two_tap_switch_shows_only_where_the_panel_offers_it_and_starts_off() {
    let (f, i) = (fonts(), info());
    let offered = |app: &mut App| app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Toggle(Switch::DoubleTap));
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.state.double_tap_supported = false;
    app.state.double_tap = false;
    assert!(
        !offered(&mut app),
        "no panel reports fingers with it dark, so there is nothing to offer"
    );
    app.state.panel_taps = true;
    app.refresh();
    assert!(offered(&mut app), "the switch is among the page's controls");
    assert!(!app.state.double_tap, "the two-tap wake is off until it is turned on");
    app.act(Action::Toggle(Switch::DoubleTap), 1000);
    assert!(app.state.double_tap, "the switch turns it on");
    app.act(Action::Toggle(Switch::DoubleTap), 2000);
    assert!(!app.state.double_tap, "and off again");
}
#[test]
fn scrolling_back_down_past_the_top_pulls_from_the_top() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.frame(&f, &i);
    app.touch(Touch::Down(700.0, 2000.0), &f, 0);
    app.touch(Touch::Move(700.0, 1700.0), &f, 100);
    assert_eq!(app.top().1, 300.0);
    app.touch(Touch::Move(700.0, 2100.0), &f, 500);
    assert_eq!(app.top().1, 0.0);
    assert!(app.overscroll(500) < 0.0, "the last 100 px pull the page down");
}
#[test]
fn dragging_scrolls_and_does_not_tap() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    let (x, y) = spot(&mut app, &f, &i, Action::Toggle(Switch::Vibrate));
    assert!(y < H as f32, "the switch is on the first screen");
    app.touch(Touch::Down(x, y), &f, 0);
    app.touch(Touch::Move(x, y - 100.0), &f, 0);
    app.touch(Touch::Move(x, y - 300.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    assert!(app.state.vibrate, "the switch was not flipped");
    assert_eq!(app.top().1, 300.0);
}
#[test]
fn the_file_transfer_switch_has_the_runtime_follow_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    assert!(app.state.file_transfer, "on until turned off");
    let (_, y) = spot(&mut app, &f, &i, Action::Toggle(Switch::FileTransfer));
    if y > H as f32 * 0.9 {
        scroll_to(&mut app, &f, y - H as f32 * 0.8, 0);
    }
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::Toggle(Switch::FileTransfer), 2000),
        [Effect::FileTransfer(false)]
    );
    assert!(!app.state.file_transfer);
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::Toggle(Switch::FileTransfer), 3000),
        [Effect::FileTransfer(true)]
    );
    app.file_transfer_failed(false, "无法开启文件传输", 4000);
    assert!(!app.state.file_transfer, "the switch shows how it stands");
}
/// Flick the page up from `start`: the finger travels `step` px every
/// 8 ms five times and lifts 8 ms after its last move. Returns when it
/// lifted.
fn flick(app: &mut App, f: &Fonts, start: u64, step: f32) -> u64 {
    app.touch(Touch::Down(700.0, 2400.0), f, start);
    for k in 1..=5u64 {
        app.touch(Touch::Move(700.0, 2400.0 - step * k as f32), f, start + 8 * k);
    }
    let lift = start + 48;
    app.touch(Touch::Up, f, lift);
    lift
}
#[test]
fn a_flick_glides_on_and_comes_to_rest() {
    let f = fonts();
    let battery = skyward_info::Battery {
        level: 80,
        charge: Charge::Discharging,
        temp: Some(300),
        health: Some("Good".into()),
        cycles: Some(169),
        design: Some(5_500_000),
        full: Some(5_095_000),
        voltage: Some(4_200_000),
        quick: false,
        plugged: false,
    };
    let i = DeviceInfo {
        os_version: Some("OS3.0.305.0.WMACNXM".into()),
        android: Some("16".into()),
        battery: Some(battery),
        ..info()
    };
    let mut app = App::new(Screen::Specs, W, H, PPD);
    app.frame(&f, &i);
    let lift = flick(&mut app, &f, 1000, 20.0);
    let lifted = app.top().1;
    assert_eq!(lifted, 100.0);
    assert_eq!(app.deadline(lift), Some(0), "the page moves on after the lift");
    app.tick(lift + 50);
    assert!(app.top().1 > lifted);
    for ms in (100..5000).step_by(8) {
        app.tick(lift + ms);
    }
    assert_eq!(app.deadline(lift + 5000), app.idle_due(), "the glide comes to rest");
    let rest = app.top().1;
    assert!(rest > lifted + 300.0 && rest < app.max_scroll(), "{rest}");
}
#[test]
fn a_page_drawn_again_as_work_goes_on_keeps_its_scroll() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.backup_started(false, Place::Storage, true);
    app.backup_plan((0..20).map(|k| (format!("part{k}"), 1_000_000)).collect());
    let at = |done| BackupAt {
        item: 0,
        done,
        of: 1_000_000,
        checking: false,
    };
    app.backup_at(at(0));
    app.frame(&f, &i);
    app.touch(Touch::Down(700.0, 2400.0), &f, 0);
    for k in 1..=4u64 {
        app.touch(Touch::Move(700.0, 2400.0 - 200.0 * k as f32), &f, 8 * k);
    }
    let dragged = app.top().1;
    assert!(dragged > 0.0);
    app.backup_at(at(500_000));
    app.touch(Touch::Move(700.0, 1400.0), &f, 40);
    assert_eq!(
        (app.top().1, app.pull),
        (dragged + 200.0, 0.0),
        "the page goes on from where it was"
    );
    app.touch(Touch::Up, &f, 48);
    let lifted = app.top().1;
    app.backup_at(at(600_000));
    app.tick(98);
    assert!(app.top().1 > lifted, "a glide goes on through the page being drawn again");
    assert_eq!(app.overscroll(98), 0.0, "and does not spring back from the top");
}
#[test]
fn a_glide_into_the_end_swings_past_it_and_back() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.frame(&f, &i);
    let lift = flick(&mut app, &f, 1000, 80.0);
    let mut furthest: f32 = 0.0;
    for ms in (8..2000).step_by(8) {
        app.tick(lift + ms);
        furthest = furthest.max(app.overscroll(lift + ms));
    }
    assert_eq!(app.top().1, app.max_scroll());
    assert!(furthest > app.theme().m.dp(8.0), "it ran on past the end: {furthest}");
    assert_eq!(app.overscroll(lift + 2000), 0.0);
    assert_eq!(app.deadline(lift + 2000), app.idle_due());
}
#[test]
fn a_finger_that_stops_before_lifting_leaves_the_page_still() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.frame(&f, &i);
    app.touch(Touch::Down(700.0, 2400.0), &f, 1000);
    app.touch(Touch::Move(700.0, 2200.0), &f, 1008);
    app.touch(Touch::Move(700.0, 2000.0), &f, 1016);
    app.touch(Touch::Up, &f, 1100);
    assert!(app.fling.is_none());
    assert_eq!(app.top().1, 400.0);
}
#[test]
fn touching_a_gliding_page_stops_it_without_pressing() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.frame(&f, &i);
    let lift = flick(&mut app, &f, 1000, 20.0);
    app.tick(lift + 30);
    assert!(app.fling.is_some(), "the page is gliding");
    let held = app.top().1;
    let (x, y) = spot(&mut app, &f, &i, Action::Toggle(Switch::Vibrate));
    assert!(y > 0.0 && y < H as f32, "the switch is on screen");
    app.touch(Touch::Down(x, y), &f, lift + 40);
    app.tick(lift + 500);
    assert_eq!(app.top().1, held, "the page stopped under the finger");
    app.touch(Touch::Up, &f, lift + 520);
    assert!(app.state.vibrate, "the switch was not flipped");
    assert!(app.fling.is_none());
}
#[test]
fn an_edge_swipe_goes_back_over_a_gliding_page() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Settings, &f, &i);
    app.frame(&f, &i);
    let lift = flick(&mut app, &f, 1000, 20.0);
    app.tick(lift + 30);
    app.touch(Touch::Down(10.0, 1600.0), &f, lift + 40);
    app.touch(Touch::Move(90.0, 1605.0), &f, lift + 48);
    let under = app.top().1;
    app.tick(lift + 64);
    assert!(app.top().1 > under, "the page glides on under the swipe");
    app.touch(Touch::Move(400.0, 1610.0), &f, lift + 72);
    app.touch(Touch::Up, &f, lift + 80);
    assert_eq!(app.screen(), Screen::Home);
}
#[test]
fn a_finger_dragging_from_the_edge_catches_a_gliding_page() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Settings, &f, &i);
    app.frame(&f, &i);
    let lift = flick(&mut app, &f, 1000, 20.0);
    app.tick(lift + 30);
    app.touch(Touch::Down(10.0, 1600.0), &f, lift + 40);
    app.tick(lift + 48);
    assert!(app.fling.is_some(), "landing at the edge leaves the glide going");
    app.touch(Touch::Move(12.0, 1500.0), &f, lift + 56);
    assert!(app.fling.is_none(), "dragging the page catches it");
    let held = app.top().1;
    app.touch(Touch::Move(12.0, 1450.0), &f, lift + 64);
    assert_eq!(app.top().1, held + 50.0, "and it follows the finger from there");
}
#[test]
fn a_button_stops_a_gliding_page() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.frame(&f, &i);
    let lift = flick(&mut app, &f, 1000, 20.0);
    assert!(app.fling.is_some(), "the page is gliding");
    app.key(Key::Back, lift + 20);
    assert!(app.fling.is_none());
}
/// Drag the page to scroll `to`, resting before lifting so it does not
/// glide, and let the time pass. The finger first goes 100 px, past the
/// touch slop, and comes back to the mark. It runs down the page's
/// margin, clear of the sliders, which a finger landing on moves
/// instead of the page.
fn scroll_to(app: &mut App, f: &Fonts, to: f32, now: u64) {
    let (from, x) = (app.top().1, app.theme().m.margin / 2.0);
    app.touch(Touch::Down(x, 2400.0), f, now);
    app.touch(Touch::Move(x, 2300.0), f, now + 200);
    app.touch(Touch::Move(x, 2400.0 - (to - from)), f, now + 400);
    app.touch(Touch::Up, f, now + 800);
    app.tick(now + 800);
    assert!((app.top().1 - to).abs() < 0.01, "{} {to}", app.top().1);
}
#[test]
fn the_bar_title_comes_after_the_large_title_fades_and_the_bar_once_content_slides_under() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    let fr = app.frame(&f, &i);
    let content_top = fr.content_top;
    let m = app.theme().m;
    scroll_to(&mut app, &f, m.dp(TITLE_FADE_DP) - 2.0, 0);
    assert!(!app.title.on && !app.bar.on);
    scroll_to(&mut app, &f, m.dp(TITLE_FADE_DP) + 2.0, 1000);
    assert!(app.title.on, "the large title is gone");
    assert!(!app.bar.on, "the content is not under the bar yet");
    assert!(app.title.level(1900) < 1.0 && app.title.level(1900) > 0.0, "it eases in");
    assert_eq!(app.title.level(3000), 1.0);
    let under = content_top - widgets::bar_bottom(&app.theme()) - m.dp(20.0);
    scroll_to(&mut app, &f, under + 10.0, 4000);
    assert!(app.bar.on);
    assert_eq!(app.bar.level(4800 + 100), 1.0, "the bar comes in 100 ms");
    scroll_to(&mut app, &f, 0.0, 6000);
    assert!(!app.title.on && !app.bar.on);
}
#[test]
fn the_about_page_brings_title_and_bar_together_once_the_version_fades() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::About, W, H, PPD);
    app.frame(&f, &i);
    let t = app.theme();
    let gone = t.m.dp(ABOUT_LOGO_DP) - widgets::bar_bottom(&t);
    scroll_to(&mut app, &f, gone - 4.0, 0);
    assert!(!app.title.on && !app.bar.on);
    scroll_to(&mut app, &f, gone + 4.0, 1000);
    assert!(app.title.on && app.bar.on);
    assert!((app.bar.level(1800 + 75) - 0.5).abs() < 0.01, "a straight 150 ms fade");
}
#[test]
fn a_scrolled_page_gone_back_to_has_its_bar_from_its_first_frame() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Settings));
    app.frame(&f, &i);
    let end = app.max_scroll();
    scroll_to(&mut app, &f, end, 0);
    app.tick(2000);
    assert_eq!(
        (app.title.level(2000), app.bar.level(2000)),
        (1.0, 1.0),
        "scrolled down, the bar is up"
    );
    tap_at(&mut app, &f, &i, Action::Open(Screen::Language), 3000);
    assert_eq!(app.screen(), Screen::Language);
    tap_back(&mut app, &f, &i, 4000);
    assert_eq!(app.screen(), Screen::Settings);
    let mut c = Canvas::new(W, H);
    app.tick(4008);
    app.compose(&mut c, &f, &i, "12:00", 4008);
    assert_eq!(app.top().1, end, "the page is where it was left");
    assert_eq!(
        (app.title.level(4008), app.bar.level(4008)),
        (1.0, 1.0),
        "its bar is up from the first frame"
    );
}
#[test]
fn theme_previews_switch_the_theme() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    tap(&mut app, &f, &i, Action::Theme { dark: true });
    assert!(app.state.dark);
    assert!(app.theme().dark);
    tap(&mut app, &f, &i, Action::Theme { dark: false });
    assert!(!app.state.dark);
}
#[test]
fn dragging_the_track_sets_brightness() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Slider(Slider::Brightness))
        .unwrap()
        .rect;
    let y = track.centre().1 - app.top().1;
    assert!(app.touch(Touch::Down(track.x + track.w * 0.25, y), &f, 0).is_empty());
    assert_eq!(app.panel(0), Some(0.25));
    assert!(app.touch(Touch::Move(track.x + track.w * 0.75, y + 40.0), &f, 0).is_empty());
    assert_eq!(app.panel(0), Some(0.75));
    assert!(app.touch(Touch::Up, &f, 0).is_empty(), "the panel shows the brightness");
    assert_eq!(app.state.brightness, 0.75);
}
#[test]
fn the_strength_slider_taps_the_motor_as_strongly_as_it_is_let_go() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Slider(Slider::Vibration))
        .unwrap()
        .rect;
    let y = track.centre().1 - app.top().1;
    assert!(app.touch(Touch::Down(track.x + track.w * 0.25, y), &f, 0).is_empty());
    assert!(
        app.touch(Touch::Move(track.x + track.w * 0.8, y + 40.0), &f, 0).is_empty(),
        "nothing taps while the finger moves"
    );
    assert_eq!(app.touch(Touch::Up, &f, 0), vec![Effect::Haptic], "let go, the motor taps");
    assert!((app.state.vibration - 0.8).abs() < 1e-5);
    assert!(app.touch(Touch::Down(track.x - 50.0, y), &f, 0).is_empty());
    assert!(app.touch(Touch::Up, &f, 0).is_empty(), "touched beside the track, the slider stays");
    app.state.vibrate = false;
    app.touch(Touch::Down(track.x, y), &f, 0);
    assert!(
        app.touch(Touch::Up, &f, 0).is_empty(),
        "with touch vibration off, the motor stays still"
    );
    assert_eq!(app.state.vibration, 0.0, "the strength still moves, down to the slider's left end");
}
#[test]
fn swiping_the_knob_reboots_to_the_chosen_target() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    tap(&mut app, &f, &i, Action::RebootTarget(3));
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(Confirm::Reboot))
        .unwrap()
        .rect;
    let y = track.centre().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 0);
    app.touch(Touch::Move(track.x + track.w / 2.0, y), &f, 0);
    app.touch(Touch::Move(track.x + track.w, y), &f, 0);
    let fx = app.touch(Touch::Up, &f, 0);
    assert_eq!(fx, vec![Effect::Reboot(Target::Bootloader)]);
    assert_eq!(app.state.confirm, 0.0, "the knob springs back");
}
#[test]
fn letting_go_halfway_does_not_reboot() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(Confirm::Reboot))
        .unwrap()
        .rect;
    let y = track.centre().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 0);
    app.touch(Touch::Move(track.x + track.w / 2.0, y), &f, 0);
    assert!(app.touch(Touch::Up, &f, 0).is_empty());
}
#[test]
fn a_control_that_cannot_work_explains_itself() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Mount, W, H, PPD);
    app.set_mounts(Ok(vec![MountRow {
        base: "odm".into(),
        point: "/odm".into(),
        mounted: true,
        holders: vec!["qseecomd".into()],
    }]));
    tap(&mut app, &f, &i, Action::Unavailable("无法卸载正在使用的分区"));
    assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("无法卸载正在使用的分区"));
    app.tick(1000);
    assert_eq!(app.deadline(1000), Some(TOAST_MS));
    app.tick(TOAST_MS);
    assert_eq!(app.deadline(TOAST_MS), app.idle_due());
}
#[test]
fn the_language_chosen_is_spoken_on_every_page() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Settings, &f, &i);
    tap(&mut app, &f, &i, Action::Open(Screen::Language));
    assert_eq!(app.frame(&f, &i).title, "语言");
    tap(&mut app, &f, &i, Action::Language(Lang::En));
    assert_eq!((app.state.lang, app.screen()), (Lang::En, Screen::Language), "the page stays");
    assert_eq!(app.frame(&f, &i).title, "Language");
    tap(&mut app, &f, &i, Action::Language(Lang::Hant));
    assert_eq!(app.frame(&f, &i).title, "語言");
    tap_back(&mut app, &f, &i, 0);
    app.tick(1000);
    assert_eq!(
        (app.screen(), app.frame(&f, &i).title),
        (Screen::Settings, "設定"),
        "and so does the page gone back to"
    );
    app.set_mounts(Ok(vec![MountRow {
        base: "odm".into(),
        point: "/odm".into(),
        mounted: true,
        holders: vec!["qseecomd".into()],
    }]));
    tap(&mut app, &f, &i, Action::Open(Screen::Language));
    tap(&mut app, &f, &i, Action::Language(Lang::En));
    tap_back(&mut app, &f, &i, 0);
    app.tick(2000);
    tap_back(&mut app, &f, &i, 2000);
    app.tick(3000);
    tap(&mut app, &f, &i, Action::Open(Screen::Mount));
    tap(&mut app, &f, &i, Action::Unavailable("A partition in use can't be unmounted"));
    assert_eq!(
        app.toast.as_ref().map(|t| t.0.as_str()),
        Some("A partition in use can't be unmounted"),
        "as do its messages"
    );
}
#[test]
fn a_language_chosen_is_kept_and_spoken_over_the_one_the_system_names() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Settings, &f, &i);
    app.take_settings(None, Some(Lang::Hant));
    assert_eq!(
        (app.state.lang, app.settings().language),
        (Lang::Hant, None),
        "the system's, until one is chosen"
    );
    tap(&mut app, &f, &i, Action::Open(Screen::Language));
    tap(&mut app, &f, &i, Action::Language(Lang::En));
    let kept = app.settings();
    assert_eq!(kept.language.as_deref(), Some("en"));
    let mut next = App::new(Screen::Home, W, H, PPD);
    next.take_settings(Some(&kept), Some(Lang::Hans));
    assert_eq!(next.state.lang, Lang::En, "the next start speaks the language chosen");
    assert_eq!(next.settings(), kept, "and keeps it chosen");
}
#[test]
fn with_no_language_chosen_a_start_speaks_the_system_s_or_else_the_one_last_spoken() {
    let spoke_english = Settings {
        spoken: Some("en".into()),
        ..Settings::default()
    };
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.take_settings(Some(&spoke_english), Some(Lang::Hant));
    assert_eq!(app.state.lang, Lang::Hant, "the one the system names");
    let mut fastboot = App::new(Screen::Fastboot, W, H, PPD);
    fastboot.take_settings(Some(&spoke_english), None);
    assert_eq!(fastboot.state.lang, Lang::En, "a start that names none, as fastboot's");
    let mut first = App::new(Screen::Fastboot, W, H, PPD);
    first.take_settings(None, None);
    assert_eq!(first.state.lang, State::default().lang, "and with nothing kept, the pages' own");
}
#[test]
fn the_other_pages_choices_are_kept_too() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Settings, &f, &i);
    tap(&mut app, &f, &i, Action::Toggle(Switch::Vibrate));
    app.state.check_signature = true;
    app.state.backup_to = Place::UsbDrive;
    app.state.logs_to = 1;
    let kept = app.settings();
    let mut next = App::new(Screen::Home, W, H, PPD);
    next.take_settings(Some(&kept), None);
    assert!(!next.state.vibrate && next.state.check_signature);
    assert_eq!((next.state.backup_to, next.state.logs_to), (Place::UsbDrive, 1));
}
/// The installing page, opened from the install page, with the
/// installer started and waiting for the computer.
fn sideloading(f: &Fonts, i: &DeviceInfo) -> App {
    let mut app = opened(Screen::Install, f, i);
    assert_eq!(tap(&mut app, f, i, Action::Sideload), vec![Effect::Sideload]);
    assert_eq!(app.screen(), Screen::Installing);
    assert_eq!(app.state.install, Install::Starting);
    app.install(Event::Waiting, 0);
    app
}
#[test]
fn leaving_while_waiting_cancels_and_the_end_clears_it() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    assert_eq!(app.state.install, Install::Waiting);
    app.frame(&f, &i);
    app.touch(Touch::Down(10.0, 1600.0), &f, 0);
    app.touch(Touch::Move(90.0, 1600.0), &f, 8);
    app.touch(Touch::Move(400.0, 1600.0), &f, 16);
    assert_eq!(app.touch(Touch::Up, &f, 24), vec![Effect::Cancel]);
    assert_eq!(app.screen(), Screen::Install);
    assert_eq!(app.state.install, Install::Ending);
    app.install(Event::Checking { size: 4096 }, 0);
    assert_eq!(app.state.install, Install::Ending);
    tap(&mut app, &f, &i, Action::Sideload);
    assert_eq!(app.screen(), Screen::Install, "no second installer while the first is ending");
    app.install(Event::Finished(Outcome::Cancelled), 0);
    assert_eq!(app.state.install, Install::Idle);
}
#[test]
fn an_image_goes_where_it_is_sent_once_swiped() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Checking { size: 201_326_592 }, 0);
    let kind = ImageKind::Boot {
        kernel: true,
        ramdisk: skyward_install::Ramdisk::Recovery,
    };
    app.install(
        Event::Image {
            kind,
            size: 201_326_592,
            partitions: vec!["recovery", "boot"],
        },
        0,
    );
    tap_at(&mut app, &f, &i, Action::FlashPartition(1), 1000);
    tap_at(&mut app, &f, &i, Action::FlashSlots(Pick::Both), 1000);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(Confirm::Flash))
        .unwrap()
        .rect;
    let y = track.centre().1 - app.top().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 1000);
    app.touch(Touch::Move(track.x + track.w, y), &f, 1000);
    assert_eq!(
        app.touch(Touch::Up, &f, 1000),
        vec![Effect::Flash {
            partition: "boot",
            pick: Pick::Both
        }]
    );
    assert_eq!(
        app.state.install,
        Install::Flashing {
            partition: "boot",
            stage: None
        }
    );
    app.install(Event::Progress(Stage::Flashing { done: 10, total: 40 }), 1000);
    assert_eq!(
        app.state.install,
        Install::Flashing {
            partition: "boot",
            stage: Some(Stage::Flashing { done: 10, total: 40 })
        }
    );
    app.key(Key::Back, 1000);
    assert_eq!(app.screen(), Screen::Installing, "a partition being written keeps the page");
    app.install(
        Event::Finished(Outcome::Flashed {
            partition: "boot",
            slots: vec![Slot::A, Slot::B],
        }),
        1000,
    );
    assert!(
        matches!(app.state.install, Install::Finished { damage: None, .. }),
        "a flash that worked broke nothing"
    );
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), 2000),
        vec![Effect::Reboot(Target::System)]
    );
    app.key(Key::Back, 2000);
    assert_eq!(app.screen(), Screen::Install);
    assert_eq!(app.state.install, Install::Idle);
}
#[test]
fn a_super_image_is_written_whole_and_asks_for_no_slot() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Checking { size: 6 << 30 }, 0);
    app.install(
        Event::Image {
            kind: ImageKind::Super,
            size: 6 << 30,
            partitions: vec!["super"],
        },
        0,
    );
    let frame = app.frame(&f, &i);
    let actions: Vec<Action> = frame.hits.iter().map(|h| h.action).collect();
    assert!(actions.contains(&Action::FlashPartition(0)), "the one partition it is written to");
    assert!(
        !actions.iter().any(|a| matches!(a, Action::FlashSlots(_))),
        "a super image has no slot to pick"
    );
    assert!(
        actions.contains(&Action::Confirm(Confirm::Flash)),
        "one swipe, as for any other flash"
    );
    tap_at(&mut app, &f, &i, Action::FlashPartition(0), 1000);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(Confirm::Flash))
        .unwrap()
        .rect;
    let y = track.centre().1 - app.top().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 1000);
    app.touch(Touch::Move(track.x + track.w, y), &f, 1000);
    assert_eq!(
        app.touch(Touch::Up, &f, 1000),
        vec![Effect::Flash {
            partition: "super",
            pick: Pick::Current
        }]
    );
    assert_eq!(
        app.state.install,
        Install::Flashing {
            partition: "super",
            stage: None
        }
    );
    app.install(
        Event::Finished(Outcome::Flashed {
            partition: "super",
            slots: Vec::new(),
        }),
        1000,
    );
    assert!(
        matches!(app.state.install, Install::Finished { damage: None, .. }),
        "a flash that worked broke nothing"
    );
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), 2000),
        vec![Effect::Reboot(Target::System)],
        "the note says to restart"
    );
}
#[test]
fn an_update_shows_its_steps_and_cannot_be_left() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Checking { size: 7_103_043_868 }, 0);
    app.install(
        Event::Installing {
            version: Some("OS3.0.306.0.WMACNXM".into()),
        },
        0,
    );
    app.install(Event::Overwriting, 0);
    app.install(
        Event::Progress(Stage::Writing {
            done: 1 << 30,
            total: 7 << 30,
        }),
        0,
    );
    assert!(matches!(
        app.state.install,
        Install::Updating {
            overwriting: true,
            stage: Some(_),
            ..
        }
    ));
    let shown = app.frame(&f, &i).hits.len();
    assert_eq!(shown, 0, "nothing on the page to tap while installing");
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Installing);
    assert_eq!(app.deadline(0), Some(TOAST_MS), "it says why");
    app.install(Event::Finished(Outcome::Failed(Failure::Engine(9))), 0);
    assert!(matches!(
        app.state.install,
        Install::Finished {
            damage: Some(Damage::System),
            ..
        }
    ));
    assert_eq!(tap_at(&mut app, &f, &i, Action::Sideload, 1000), vec![Effect::Sideload]);
    assert_eq!(app.state.install, Install::Starting);
    assert_eq!(app.screen(), Screen::Installing);
}
#[test]
fn a_refused_package_or_an_update_that_kept_the_system_broke_nothing() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Checking { size: 10 }, 0);
    app.install(Event::Finished(Outcome::Refused(Refusal::Unknown)), 0);
    assert_eq!(
        app.state.install,
        Install::Finished {
            outcome: Outcome::Refused(Refusal::Unknown),
            damage: None
        }
    );
    tap_at(&mut app, &f, &i, Action::Sideload, 1000);
    app.install(Event::Waiting, 1000);
    app.install(Event::Installing { version: None }, 1000);
    app.install(Event::Finished(Outcome::Failed(Failure::Engine(21))), 1000);
    assert!(matches!(app.state.install, Install::Finished { damage: None, .. }));
}
#[test]
fn an_image_that_failed_part_way_names_its_partition() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(
        Event::Image {
            kind: ImageKind::Dtbo,
            size: 1 << 20,
            partitions: vec!["dtbo"],
        },
        0,
    );
    app.state.install = Install::Flashing {
        partition: "dtbo",
        stage: None,
    };
    app.install(
        Event::Progress(Stage::Flashing {
            done: 1 << 20,
            total: 4 << 20,
        }),
        0,
    );
    app.install(Event::Finished(Outcome::Failed(Failure::Disconnected)), 0);
    assert!(matches!(
        app.state.install,
        Install::Finished {
            damage: Some(Damage::Partition("dtbo")),
            ..
        }
    ));
}
#[test]
fn fastboot_is_chosen_and_restarted_to_like_any_target() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    let fastboot = TARGETS.iter().position(|t| *t == Target::Fastboot).unwrap();
    tap(&mut app, &f, &i, Action::RebootTarget(fastboot));
    assert_eq!(app.state.reboot, fastboot);
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Reboot), vec![Effect::Reboot(Target::Fastboot)]);
}
/// Press `key` and let it go.
fn press(app: &mut App, key: Key, now: u64) -> Vec<Effect> {
    let mut fx = app.key(key, now);
    fx.extend(app.key_up(key, now));
    fx
}
/// The control the keys are on, on the page drawn.
fn lit(app: &mut App, f: &Fonts, i: &DeviceInfo) -> Option<Stop> {
    app.frame(f, i);
    app.lit().map(|s| s.stop)
}
/// Move the light down the page until it is on `stop`.
fn light_on(app: &mut App, f: &Fonts, i: &DeviceInfo, stop: Stop) {
    for _ in 0..100 {
        press(app, Key::VolumeDown, 0);
        if app.keys && lit(app, f, i) == Some(stop) {
            return;
        }
    }
    panic!("the keys never came to {stop:?}");
}
#[test]
fn the_keys_reach_the_last_row_of_a_home_page_grown_by_an_update_notice() {
    let f = fonts();
    let snug = snug(&f, &info());
    let i = DeviceInfo {
        snapshot: SnapshotState::Snapshotted,
        ..info()
    };
    let mut app = App::new(Screen::Home, W, snug, PPD);
    let (_, y) = spot(&mut app, &f, &i, Action::Open(Screen::Reboot));
    assert!(y > snug as f32, "below the screen to start with");
    light_on(&mut app, &f, &i, Stop::Hit(Action::Open(Screen::Reboot)));
    for ms in (0..3000).step_by(8) {
        app.tick(ms);
    }
    let (_, y) = spot(&mut app, &f, &i, Action::Open(Screen::Reboot));
    assert!(y < snug as f32, "the page scrolled the lit row into view");
    press(&mut app, Key::Power, 3000);
    assert_eq!(app.screen(), Screen::Reboot);
}
#[test]
fn the_fastboot_page_is_worked_with_the_keys() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Fastboot, W, H, PPD);
    let choice = |k: usize| Some(Stop::Hit(FASTBOOT_CHOICES[k].1));
    assert_eq!(lit(&mut app, &f, &i), choice(0), "the first choice is lit as the page opens");
    assert!(press(&mut app, Key::VolumeDown, 0).is_empty());
    assert_eq!(lit(&mut app, &f, &i), choice(1), "down moves to the next choice");
    assert!(app.key(Key::Power, 0).is_empty(), "power takes it as it comes up");
    assert_eq!(app.key_up(Key::Power, 0), vec![Effect::EnterRecovery]);
    assert!(app.display_on(), "power leaves the panel on here");
    press(&mut app, Key::VolumeUp, 0);
    press(&mut app, Key::VolumeUp, 0);
    assert_eq!(
        lit(&mut app, &f, &i),
        choice(FASTBOOT_CHOICES.len() - 1),
        "up from the first goes round to the last"
    );
    assert_eq!(press(&mut app, Key::Power, 0), vec![Effect::Reboot(Target::PowerOff)]);
    assert_eq!(
        tap(&mut app, &f, &i, Action::RebootTo(Target::Bootloader)),
        vec![Effect::Reboot(Target::Bootloader)],
        "a touch takes a choice too"
    );
    assert!(press(&mut app, Key::Back, 0).is_empty());
    assert_eq!(app.screen(), Screen::Fastboot, "there is nothing to go back to");
}
#[test]
fn the_recovery_comes_up_from_fastboot_at_the_home_page() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Fastboot, W, H, PPD);
    app.frame(&f, &i);
    app.enter_recovery();
    assert_eq!(app.screen(), Screen::Home);
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Home, "fastboot is not behind it");
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::Search))),
        "the keys go on working the pages"
    );
    app.touch(Touch::Down(W as f32 / 2.0, 200.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    assert_eq!(press(&mut app, Key::Power, 0), vec![Effect::Ticks]);
    assert!(!app.display_on(), "with the light put out, power turns the panel off");
}
#[test]
fn without_a_touchscreen_the_first_control_is_lit_at_once() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.frame(&f, &i);
    assert_eq!(app.light(0.0), None, "a page opens to the touch");
    app.touchscreen_missing();
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::Search))));
    assert!(app.light(0.0).is_some(), "lit before any key is pressed");
    assert!(app.take_dirty(), "and shown");
}
#[test]
fn the_volume_keys_light_the_controls_down_the_page_and_up_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.frame(&f, &i);
    assert!(!app.keys, "a page opens to the touch");
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::Search))),
        "the first press lights the first control in view"
    );
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::About))));
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::Install))));
    press(&mut app, Key::VolumeUp, 0);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::About))));
    press(&mut app, Key::VolumeUp, 0);
    press(&mut app, Key::VolumeUp, 0);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::Reboot))),
        "up from the first goes round to the last"
    );
    app.tick(5000);
    let reboot = app.lit().expect("lit");
    assert!(app.shows(&reboot, app.top().1), "scrolled into view");
    press(&mut app, Key::VolumeDown, 6000);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::Search))),
        "down from the last goes round to the first"
    );
    app.tick(10_000);
    assert_eq!(app.top().1, 0.0, "at the top of the page again");
}
#[test]
fn the_power_key_works_the_control_lit_as_it_comes_up() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.frame(&f, &i);
    light_on(&mut app, &f, &i, Stop::Hit(Action::Open(Screen::Settings)));
    assert!(app.key(Key::Power, 0).is_empty());
    assert_eq!(app.screen(), Screen::Home, "nothing happens as the key goes down");
    app.key_up(Key::Power, 0);
    assert_eq!(app.screen(), Screen::Settings, "the row opens its page as the key comes up");
    let first = lit(&mut app, &f, &i);
    assert!(
        first.is_some() && first != Some(Stop::Back),
        "the page's first control is lit, not the `<`: {first:?}"
    );
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Home);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::Settings))),
        "back on the page, the keys are where they left it"
    );
    light_on(&mut app, &f, &i, Stop::Hit(Action::Open(Screen::Settings)));
    press(&mut app, Key::Power, 0);
    assert_eq!(lit(&mut app, &f, &i), first);
    press(&mut app, Key::VolumeUp, 0);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Back),
        "up from the page's first control comes to the `<`"
    );
    press(&mut app, Key::Power, 0);
    assert_eq!(app.screen(), Screen::Home, "the `<` goes back");
}
#[test]
fn past_the_last_control_the_keys_scroll_on_half_a_screen_a_press() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Installing, W, H, PPD);
    app.state.install = Install::Finished {
        outcome: Outcome::Ran { wipe_cache: false },
        damage: None,
    };
    app.state.script_output = (0..300).map(|k| format!("- line {k}")).collect();
    app.frame(&f, &i);
    let (max, (top, bottom)) = (app.max_scroll(), app.band(0.0));
    let half = (bottom - top) / 2.0;
    assert!(max > half * 4.0, "the script's output runs on below the controls: {max}");
    let last = *app.spots().expect("drawn").last().expect("controls on the page");
    light_on(&mut app, &f, &i, last.stop);
    let mut at = app.resting();
    let mut presses = 0;
    while at < max {
        press(&mut app, Key::VolumeDown, 0);
        assert_eq!(app.resting(), (at + half).min(max), "half a screen a press");
        at = app.resting();
        presses += 1;
        assert!(presses < 40);
        if !app.shows(&last, at) {
            assert_eq!(lit(&mut app, &f, &i), None, "with no control in view, nothing is lit");
        }
    }
    press(&mut app, Key::VolumeDown, 0);
    let first = app.spots().expect("drawn")[0].stop;
    assert_eq!(
        (lit(&mut app, &f, &i), app.resting()),
        (Some(first), 0.0),
        "past the end the light goes round to the top"
    );
    press(&mut app, Key::VolumeUp, 0);
    assert_eq!(lit(&mut app, &f, &i), Some(last.stop), "and up from it round to the last control");
    assert!(app.shows(&last, app.resting()), "which shows");
    let resting = app.resting();
    app.tick(5000);
    assert_eq!(app.top().1, resting, "where the page eased to");
}
#[test]
fn the_power_key_held_slides_a_swipe_control_and_confirms_it_let_go() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    app.frame(&f, &i);
    light_on(&mut app, &f, &i, Stop::Hit(Action::Confirm(Confirm::Reboot)));
    assert!(press(&mut app, Key::Power, 0).is_empty());
    assert_eq!(
        app.toast.as_ref().map(|t| t.0.as_str()),
        Some("请按住电源键"),
        "a short press says how"
    );
    app.key(Key::Power, 1000);
    assert_eq!(app.deadline(1000), Some(1000 + CHORD_MS), "it waits for volume down first");
    app.tick(1000 + CHORD_MS);
    assert_eq!(app.deadline(1000 + CHORD_MS), Some(0), "then the knob moves while the key is held");
    app.tick(1000 + SLIDE_MS / 2);
    assert!(
        (app.state.confirm - 0.5).abs() < 0.01,
        "half way after half the time: {}",
        app.state.confirm
    );
    assert!(
        app.key_up(Key::Power, 1000 + SLIDE_MS / 2).is_empty(),
        "let go short of the end, nothing happens"
    );
    assert_eq!(app.state.confirm, 0.0, "and the knob goes back");
    app.key(Key::Power, 3000);
    app.tick(3000 + SLIDE_MS * 2);
    assert_eq!(app.state.confirm, 1.0, "the knob stops at the end");
    assert_eq!(
        app.key_up(Key::Power, 3000 + SLIDE_MS * 2),
        vec![Effect::Reboot(TARGETS[0])],
        "let go there, it confirms"
    );
    assert_eq!(app.state.confirm, 0.0);
    app.key(Key::Power, 6000);
    app.tick(6000 + SLIDE_MS);
    app.touch(Touch::Down(W as f32 / 2.0, 200.0), &f, 6000 + SLIDE_MS);
    assert_eq!(app.state.confirm, 0.0, "a touch lets the knob go");
    assert!(
        app.key_up(Key::Power, 6000 + SLIDE_MS).is_empty(),
        "and the key comes up to nothing"
    );
}
#[test]
fn the_brightness_slider_takes_the_volume_keys_until_the_power_key_lets_it_go() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.frame(&f, &i);
    light_on(&mut app, &f, &i, Stop::Hit(Action::Slider(Slider::Brightness)));
    assert_eq!(app.state.brightness, SLIDER_START, "it starts a step from its left end");
    press(&mut app, Key::Power, 0);
    assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("按音量键调节亮度，按电源键完成"));
    assert!(press(&mut app, Key::VolumeUp, 0).is_empty(), "the panel shows the brightness");
    assert!((app.state.brightness - (SLIDER_START + SLIDER_STEP)).abs() < 1e-5, "up turns it up");
    press(&mut app, Key::VolumeDown, 0);
    assert!((app.state.brightness - SLIDER_START).abs() < 1e-5, "down turns it down");
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(app.state.brightness, MIN_BRIGHTNESS, "no lower than the slider's floor");
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Slider(Slider::Brightness))),
        "the light stays on the slider"
    );
    press(&mut app, Key::Power, 0);
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(app.state.brightness, MIN_BRIGHTNESS, "let go, the slider keeps its level");
    assert_ne!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Slider(Slider::Brightness))),
        "and the keys move the light again"
    );
}
#[test]
fn the_strength_slider_taps_the_motor_at_each_step_of_the_keys() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.frame(&f, &i);
    light_on(&mut app, &f, &i, Stop::Hit(Action::Slider(Slider::Vibration)));
    press(&mut app, Key::Power, 0);
    assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("按音量键调节振动强度，按电源键完成"));
    assert_eq!(
        press(&mut app, Key::VolumeDown, 0),
        vec![Effect::Haptic],
        "each step taps at the strength reached"
    );
    assert_eq!(app.state.vibration, 0.0, "it starts a step from its left end, the lightest");
    for _ in 0..20 {
        press(&mut app, Key::VolumeUp, 0);
    }
    assert_eq!(app.state.vibration, 1.0, "up to its right end, the strongest");
    press(&mut app, Key::Power, 0);
    assert!(press(&mut app, Key::VolumeDown, 0).is_empty(), "let go, the keys move the light");
    assert_eq!(app.state.vibration, 1.0);
}
#[test]
fn the_keys_join_a_patterns_dots_and_end_it_on_the_last_one_joined() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Decrypt, W, H, PPD);
    app.data_waiting(Some((0, Keys::Pattern)), Vec::new());
    app.frame(&f, &i);
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Dot(0)), "the grid's first dot is lit first");
    assert!(press(&mut app, Key::Power, 0).is_empty());
    assert_eq!(app.state.pattern, [0]);
    assert_eq!(
        app.toast.as_ref().map(|t| t.0.as_str()),
        Some("连接完成后，在最后一个点上再按一次电源键")
    );
    press(&mut app, Key::Power, 0);
    assert!(app.state.pattern.is_empty(), "ended at one dot, the pattern is wiped");
    assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("请至少连接 4 个点"));
    press(&mut app, Key::Power, 0);
    press(&mut app, Key::VolumeDown, 0);
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Dot(2)));
    press(&mut app, Key::Power, 0);
    assert_eq!(app.state.pattern, [0, 1, 2], "the dot passed straight over is joined on the way");
    for _ in 0..6 {
        press(&mut app, Key::VolumeDown, 0);
    }
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Dot(8)));
    press(&mut app, Key::Power, 0);
    assert_eq!(app.state.pattern, [0, 1, 2, 5, 8]);
    press(&mut app, Key::VolumeUp, 0);
    press(&mut app, Key::VolumeUp, 0);
    press(&mut app, Key::Power, 0);
    assert_eq!(
        app.state.pattern,
        [0, 1, 2, 5, 8, 7, 6],
        "two along the row joins the one between first"
    );
    press(&mut app, Key::VolumeDown, 0);
    assert!(
        press(&mut app, Key::Power, 0).is_empty(),
        "a dot joined already is not joined again"
    );
    press(&mut app, Key::VolumeUp, 0);
    assert_eq!(
        press(&mut app, Key::Power, 0),
        vec![Effect::Unlock {
            user: 0,
            code: "1236987".into()
        }],
        "the last one joined ends it"
    );
}
#[test]
fn the_keys_type_a_screen_locks_code_on_its_keypad() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Decrypt, W, H, PPD);
    app.data_waiting(Some((0, Keys::Digits)), Vec::new());
    app.frame(&f, &i);
    for ch in ['2', '5', '8', '0'] {
        light_on(&mut app, &f, &i, Stop::Hit(Action::Type(Cap::Char(ch))));
        press(&mut app, Key::Power, 0);
    }
    assert_eq!(app.state.code, "2580");
    light_on(&mut app, &f, &i, Stop::Hit(Action::Type(Cap::Enter)));
    assert_eq!(
        press(&mut app, Key::Power, 0),
        vec![Effect::Unlock {
            user: 0,
            code: "2580".into()
        }]
    );
}
#[test]
fn the_power_key_held_on_an_entry_opens_what_can_be_done_with_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Files));
    app.set_files(
        "/".into(),
        Ok(vec![entry("vendor", folder(12)), entry("init.rc", FileKind::File)]),
        LISTED,
    );
    app.frame(&f, &i);
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::OpenEntry(0))));
    assert!(app.key(Key::Power, 1000).is_empty());
    assert_eq!(app.deadline(1000), Some(1000 + CHORD_MS), "it waits for volume down first");
    assert!(app.tick(1000 + CHORD_MS).is_empty());
    assert_eq!(
        app.deadline(1000 + CHORD_MS),
        Some(1000 + LONG_PRESS_MS),
        "the press is timed from when it went down"
    );
    assert!(app.tick(1000 + LONG_PRESS_MS - 1).is_empty());
    assert_eq!(
        app.tick(1000 + LONG_PRESS_MS),
        vec![Effect::Haptic],
        "held long enough, the folder's page opens"
    );
    assert_eq!(app.screen(), Screen::FileInfo);
    assert_eq!(app.state.file.as_ref().map(|e| e.name.as_str()), Some("vendor"));
    assert!(
        app.key_up(Key::Power, 1000 + LONG_PRESS_MS + 100).is_empty(),
        "and the key comes up to nothing"
    );
    app.key(Key::Back, 2000);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::OpenEntry(0))));
    assert_eq!(
        press(&mut app, Key::Power, 3000),
        vec![Effect::ReadDir("/vendor".into())],
        "a short press goes into the folder"
    );
}
#[test]
fn a_touch_puts_the_light_out_and_a_volume_key_brings_it_back() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.frame(&f, &i);
    light_on(&mut app, &f, &i, Stop::Hit(Action::Open(Screen::About)));
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:00", 0);
    assert!(app.light(0.0).is_some(), "lit while the keys work the page");
    app.touch(Touch::Down(W as f32 / 2.0, 200.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    assert!(app.light(0.0).is_none(), "a touch puts the light out");
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::About))),
        "the keys come back where they were"
    );
    app.touch(Touch::Down(W as f32 / 2.0, 200.0), &f, 0);
    app.touch(Touch::Up, &f, 0);
    press(&mut app, Key::Power, 0);
    assert!(!app.display_on(), "with the light out, power turns the panel off");
    press(&mut app, Key::VolumeDown, 1000);
    assert!(!app.display_on(), "a volume key leaves the panel dark");
    press(&mut app, Key::Power, 1100);
    assert!(app.display_on(), "the power key lights it");
    assert!(!app.keys, "and does only that");
}
#[test]
fn a_volume_key_held_moves_the_light_on_and_on() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.frame(&f, &i);
    app.key(Key::VolumeDown, 0);
    assert_eq!(app.deadline(0), Some(CHORD_MS), "it waits for the power key first");
    app.tick(CHORD_MS);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::Search))));
    assert_eq!(
        app.deadline(CHORD_MS),
        Some(REPEAT_DELAY_MS),
        "and repeats as from when it went down"
    );
    app.tick(REPEAT_DELAY_MS - 1);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::Search))),
        "not held long enough yet"
    );
    app.tick(REPEAT_DELAY_MS);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::About))));
    app.tick(REPEAT_DELAY_MS + REPEAT_MS);
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::Install))));
    app.key_up(Key::VolumeDown, REPEAT_DELAY_MS + REPEAT_MS + 10);
    app.tick(REPEAT_DELAY_MS + REPEAT_MS * 5);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::Open(Screen::Install))),
        "let go, it stops"
    );
}
#[test]
fn climbing_out_of_a_folder_the_keys_come_back_to_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Files));
    let folders: Vec<FileRow> = (0..300).map(|k| entry(&format!("dir{k:03}"), folder(k))).collect();
    app.set_files("/".into(), Ok(folders.clone()), LISTED);
    app.frame(&f, &i);
    press(&mut app, Key::VolumeDown, 0);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::OpenEntry(0))),
        "the first entry, below the path"
    );
    for _ in 0..40 {
        press(&mut app, Key::VolumeDown, 0);
    }
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::OpenEntry(40))));
    app.tick(5000);
    let row = app.lit().expect("lit");
    assert!(app.shows(&row, app.top().1), "scrolled into view");
    assert_eq!(press(&mut app, Key::Power, 5000), vec![Effect::ReadDir("/dir040".into())]);
    app.set_files("/dir040".into(), Ok(vec![entry("a", FileKind::File)]), LISTED);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::OpenEntry(0))),
        "a folder opened lights its first entry"
    );
    assert_eq!(app.key(Key::Back, 6000), vec![Effect::ReadDir("/".into())]);
    app.set_files("/".into(), Ok(folders), LISTED);
    assert_eq!(
        lit(&mut app, &f, &i),
        Some(Stop::Hit(Action::OpenEntry(40))),
        "back up, the folder left is lit"
    );
}
#[test]
fn the_cant_start_page_retries_looks_around_or_erases() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::CantStart, W, H, PPD);
    app.set_asked(Asked {
        reason: Some("RescueParty".into()),
        ..Asked::default()
    });
    assert_eq!(
        tap(&mut app, &f, &i, Action::RebootTo(Target::System)),
        vec![Effect::Reboot(Target::System)]
    );
    assert_eq!(swipe(&mut app, &f, &i, Confirm::FactoryReset), vec![Effect::FactoryReset]);
    assert_eq!(tap(&mut app, &f, &i, Action::Dismiss), vec![Effect::Dismiss]);
    assert_eq!(app.screen(), Screen::Home);
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Home, "the system's question is not behind it");
}
#[test]
fn erasing_goes_part_by_part_and_offers_a_way_out_once_it_stops() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::CantStart, W, H, PPD);
    app.erase_started(true, true, vec![Part::Data, Part::Metadata]);
    assert_eq!(app.screen(), Screen::Erase);
    assert!(app.frame(&f, &i).hits.is_empty(), "nothing is to be done while erasing");
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Erase, "there is nothing to go back to");
    app.erase_at(Part::Metadata);
    assert_eq!(app.state.erase.done, 1);
    press(&mut app, Key::Power, 0);
    assert!(!app.display_on());
    app.erase_ended(
        Err(EraseStop::Failed(Part::Metadata, "make_f2fs ended with exit status: 1".into())),
        1000,
    );
    assert!(app.display_on(), "what comes next is the user's to choose");
    assert_eq!(app.state.data, Data::Erased, "the data partition went before /metadata failed");
    let later = 2000;
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::RebootTo(Target::PowerOff), later),
        vec![Effect::Reboot(Target::PowerOff)]
    );
    assert_eq!(tap_at(&mut app, &f, &i, Action::Dismiss, later), vec![Effect::Dismiss]);
    assert_eq!(app.screen(), Screen::Home);
}
#[test]
fn erasing_that_ends_well_leaves_nothing_to_choose() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.erase_started(false, true, vec![Part::Cache]);
    app.erase_ended(Ok(()), 0);
    assert_eq!(app.state.erase.done, 1, "every part is erased");
    assert!(app.frame(&f, &i).hits.is_empty(), "the runtime starts the system at once");
    assert_eq!(app.state.data, Data::Closed, "the data partition was not part of it");
}
#[test]
fn the_wipe_page_erases_everything_or_the_parts_chosen() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Wipe, &f, &i);
    let chosen = |app: &App| {
        WIPES
            .iter()
            .zip(app.state.wipe)
            .filter(|(_, on)| *on)
            .map(|(&w, _)| w)
            .collect::<Vec<_>>()
    };
    assert!(swipe(&mut app, &f, &i, Confirm::Wipe).is_empty(), "nothing is chosen at first");
    for part in [
        Wipe::Updates,
        Wipe::Logs,
        Wipe::Art,
        Wipe::SystemCaches,
        Wipe::AppCaches,
        Wipe::Media,
        Wipe::AppData,
    ] {
        tap(&mut app, &f, &i, Action::WipeItem(part));
    }
    let parts = vec![
        Wipe::AppData,
        Wipe::Media,
        Wipe::AppCaches,
        Wipe::SystemCaches,
        Wipe::Art,
        Wipe::Logs,
        Wipe::Updates,
    ];
    assert_eq!(chosen(&app), parts, "every part the page lists can be chosen with the others");
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::Wipe),
        vec![Effect::Wipe(parts)],
        "in the order listed"
    );
    tap(&mut app, &f, &i, Action::WipeItem(Wipe::All));
    assert_eq!(chosen(&app), [Wipe::All], "everything takes the parts in");
    tap(&mut app, &f, &i, Action::WipeItem(Wipe::Art));
    assert_eq!(chosen(&app), [Wipe::Art], "a part on its own leaves everything out");
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Wipe), vec![Effect::Wipe(vec![Wipe::Art])]);
    tap(&mut app, &f, &i, Action::WipeItem(Wipe::Art));
    assert!(chosen(&app).is_empty(), "tapped again, a part is left out");
    tap(&mut app, &f, &i, Action::WipeItem(Wipe::All));
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Wipe), vec![Effect::Wipe(vec![Wipe::All])]);
}
#[test]
fn the_cache_partition_is_offered_only_on_a_phone_that_has_one() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Wipe, &f, &i);
    let offered = |app: &mut App| app.frame(&f, &i).hits.iter().any(|h| h.action == Action::WipeItem(Wipe::Cache));
    assert!(!offered(&mut app));
    app.state.cache_partition = true;
    app.refresh();
    assert!(offered(&mut app));
    tap(&mut app, &f, &i, Action::WipeItem(Wipe::Cache));
    tap(&mut app, &f, &i, Action::WipeItem(Wipe::Media));
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::Wipe),
        vec![Effect::Wipe(vec![Wipe::Media, Wipe::Cache])],
        "in the order listed"
    );
}
#[test]
fn erasing_the_user_chose_tells_its_progress_and_waits_for_them_at_the_end() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.erase_started(false, false, vec![Part::Media, Part::Art]);
    app.erase_progress(30, 120);
    assert_eq!(app.state.erase.progress, Some((30, 120)));
    app.erase_at(Part::Art);
    assert_eq!(app.state.erase.progress, None, "the next part starts from nothing");
    press(&mut app, Key::Power, 0);
    assert!(!app.display_on());
    app.erase_ended(Ok(()), 1000);
    assert!(app.display_on(), "how it ended waits to be read");
    assert_eq!(app.state.data, Data::Closed, "the data partition itself stays");
    let later = 2000;
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), later),
        vec![Effect::Reboot(Target::System)]
    );
    assert_eq!(tap_at(&mut app, &f, &i, Action::Dismiss, later), vec![Effect::Dismiss]);
    assert_eq!(app.screen(), Screen::Home);
}
#[test]
fn parts_waiting_on_a_locked_user_erase_nothing_and_send_the_user_home_to_decrypt() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.data = Data::Locked {
        user: 0,
        keys: Keys::Digits,
        checking: false,
    };
    app.erase_started(false, false, vec![Part::AppCaches, Part::Logs]);
    app.erase_ended(Err(EraseStop::Locked(vec![0])), 0);
    assert_eq!(app.state.erase.done, 0, "nothing was erased");
    assert_eq!(
        app.state.data,
        Data::Locked {
            user: 0,
            keys: Keys::Digits,
            checking: false
        },
        "the partition stays as it was"
    );
    assert_eq!(tap_at(&mut app, &f, &i, Action::Dismiss, 1000), vec![Effect::Dismiss]);
    assert_eq!(app.screen(), Screen::Home);
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::Open(Screen::Decrypt), 1000),
        Vec::new(),
        "the screen lock is asked for at once"
    );
    assert_eq!(app.screen(), Screen::Decrypt);
}
#[test]
fn a_data_partition_erased_whole_is_not_opened_again() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.data = Data::Open;
    app.erase_started(true, false, vec![Part::Data, Part::Metadata]);
    app.erase_ended(Ok(()), 0);
    assert_eq!(app.state.data, Data::Erased);
    assert_eq!(tap_at(&mut app, &f, &i, Action::Dismiss, 1000), vec![Effect::Dismiss]);
    assert!(
        tap_at(&mut app, &f, &i, Action::Open(Screen::Decrypt), 1000).is_empty(),
        "nothing opens until the system makes it afresh"
    );
    assert_eq!(app.screen(), Screen::Decrypt);
}
#[test]
fn an_update_that_erases_all_user_data_waits_to_be_agreed_to() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Checking { size: 7_103_043_868 }, 0);
    app.install(
        Event::Ready {
            version: Some("OS3.0.306.0.WMACNXM".into()),
            wipe: true,
        },
        0,
    );
    assert_eq!(
        app.state.install,
        Install::Confirming {
            version: Some("OS3.0.306.0.WMACNXM".into()),
            wipe: true
        }
    );
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(Confirm::InstallWipe))
        .unwrap()
        .rect;
    let y = track.centre().1 - app.top().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 1000);
    app.touch(Touch::Move(track.x + track.w, y), &f, 1000);
    assert_eq!(app.touch(Touch::Up, &f, 1000), vec![Effect::Install]);
    assert_eq!(
        app.state.install,
        Install::Updating {
            version: Some("OS3.0.306.0.WMACNXM".into()),
            stage: None,
            overwriting: false
        }
    );
    app.key(Key::Back, 1000);
    assert_eq!(app.screen(), Screen::Installing, "an update being written keeps the page");
    let mut app = sideloading(&f, &i);
    app.install(Event::Ready { version: None, wipe: true }, 0);
    assert_eq!(
        app.key(Key::Back, 1000),
        vec![Effect::Cancel],
        "turning it down cancels before anything is written"
    );
    assert_eq!(app.state.install, Install::Ending);
    assert_eq!(app.screen(), Screen::Install);
}
#[test]
fn a_zip_runs_its_install_script_once_agreed_to_and_shows_what_it_says() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Install, &f, &i);
    app.data_waiting(None, Vec::new());
    tap(&mut app, &f, &i, Action::Open(Screen::Choose(Place::Storage)));
    app.set_place(Place::Storage, String::new(), Ok(vec![entry("rom.zip", FileKind::File)]), LISTED);
    assert_eq!(
        tap(&mut app, &f, &i, Action::ChooseEntry(0)),
        vec![Effect::InstallFile(Place::Storage, "rom.zip".into())]
    );
    assert_eq!(app.state.package_name.as_deref(), Some("rom.zip"), "the question names the file");
    app.install(Event::Script, 0);
    assert_eq!(app.state.install, Install::ConfirmingScript);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(Confirm::RunScript))
        .unwrap()
        .rect;
    let y = track.centre().1 - app.top().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 1000);
    app.touch(Touch::Move(track.x + track.w, y), &f, 1000);
    assert_eq!(app.touch(Touch::Up, &f, 1000), vec![Effect::Install]);
    assert_eq!(
        app.state.install,
        Install::Scripting {
            stage: None,
            running: false
        }
    );
    app.install(Event::Running, 1000);
    for line in ["- Flashing boot", " ", "- Done"] {
        app.install(Event::Print(line.into()), 1000);
    }
    app.install(Event::Progress(Stage::Script { thousandths: 500 }), 1000);
    assert_eq!(
        app.state.install,
        Install::Scripting {
            stage: Some(Stage::Script { thousandths: 500 }),
            running: true
        }
    );
    let fr = app.frame(&f, &i);
    assert!(fr.hits.is_empty(), "nothing on the page to tap while the script runs");
    assert_eq!(fr.rows.as_ref().map(Rows::count), Some(3), "a row to each line it showed");
    app.key(Key::Back, 1000);
    assert_eq!(app.screen(), Screen::Installing, "a script running keeps the page");
    assert_eq!(app.deadline(1000), Some(1000 + TOAST_MS), "it says why");
    app.install(Event::Finished(Outcome::Ran { wipe_cache: false }), 1000);
    assert_eq!(
        app.state.install,
        Install::Finished {
            outcome: Outcome::Ran { wipe_cache: false },
            damage: None
        }
    );
    let fr = app.frame(&f, &i);
    let rows = fr.rows.as_ref().expect("what it showed stays on the page");
    assert_eq!(rows.count(), 3);
    assert!(
        fr.hit(W as f32 / 2.0, rows.rect(1, W as f32).centre().1).is_none(),
        "its lines answer no touch"
    );
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), 2000),
        vec![Effect::Reboot(Target::System)]
    );
}
#[test]
fn a_script_turned_down_never_runs_and_one_that_failed_may_have_written_anywhere() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Script, 0);
    assert_eq!(
        app.key(Key::Back, 1000),
        vec![Effect::Cancel],
        "turning it down cancels before it runs"
    );
    assert_eq!((app.screen(), &app.state.install), (Screen::Install, &Install::Ending));
    let mut app = sideloading(&f, &i);
    app.install(Event::Script, 0);
    app.state.install = Install::Scripting {
        stage: None,
        running: false,
    };
    let full = "extract META-INF/com/google/android/update-binary: No space left on device (os error 28)";
    app.install(Event::Finished(Outcome::Failed(Failure::Error(full.into()))), 0);
    assert!(
        matches!(app.state.install, Install::Finished { damage: None, .. }),
        "a script that never ran wrote nothing"
    );
    tap_at(&mut app, &f, &i, Action::Sideload, 1000);
    app.install(Event::Waiting, 1000);
    app.install(Event::Script, 1000);
    app.state.install = Install::Scripting {
        stage: None,
        running: false,
    };
    app.install(Event::Running, 1000);
    app.install(Event::Print("! Could not write vendor_b".into()), 1000);
    app.install(Event::Finished(Outcome::Failed(Failure::Script(1))), 1000);
    assert_eq!(
        app.state.install,
        Install::Finished {
            outcome: Outcome::Failed(Failure::Script(1)),
            damage: Some(Damage::Script)
        }
    );
    assert_eq!(
        app.state.script_output,
        ["! Could not write vendor_b"],
        "what it showed before it failed stays"
    );
    assert_eq!(tap_at(&mut app, &f, &i, Action::Sideload, 2000), vec![Effect::Sideload]);
    assert!(app.state.script_output.is_empty(), "sending again starts afresh");
}
#[test]
fn a_script_page_keeps_the_latest_lines_its_script_showed() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Script, 0);
    app.install(Event::Running, 0);
    for k in 0..SCRIPT_LINES + 2 {
        app.install(Event::Print(format!("line {k}")), 0);
    }
    assert_eq!(app.state.script_output.len(), SCRIPT_LINES);
    assert_eq!(app.state.script_output[0], "line 2");
    assert_eq!(app.frame(&f, &i).rows.as_ref().map(Rows::count), Some(SCRIPT_LINES));
}
#[test]
fn the_install_page_offers_to_keep_skyward_where_the_phone_has_recovery_partitions() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Install, &f, &i);
    let offered = |app: &mut App| {
        app.frame(&f, &i)
            .hits
            .iter()
            .any(|h| h.action == Action::Toggle(Switch::KeepRecovery))
    };
    assert!(!offered(&mut app), "a phone whose recovery lives in boot");
    app.state.recovery_partitions = true;
    app.refresh();
    assert!(offered(&mut app));
    assert!(app.state.keep_recovery, "on until turned off");
    tap(&mut app, &f, &i, Action::Toggle(Switch::KeepRecovery));
    assert!(!app.state.keep_recovery);
}
#[test]
fn the_install_page_offers_to_check_a_zips_signature_off_until_turned_on() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Install, &f, &i);
    assert!(
        app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Toggle(Switch::Signature)),
        "on any phone"
    );
    assert!(!app.state.check_signature);
    tap(&mut app, &f, &i, Action::Toggle(Switch::Signature));
    assert!(app.state.check_signature);
}
#[test]
fn a_signature_being_checked_shows_how_far_and_leaving_stops_it() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Checking { size: 4096 }, 0);
    app.key(Key::Back, 1000);
    assert_eq!(app.screen(), Screen::Installing, "a package being looked at keeps the page");
    let checking = Stage::Signature { done: 1024, total: 4000 };
    app.install(Event::Progress(checking.clone()), 1000);
    assert_eq!(
        app.state.install,
        Install::Checking {
            size: 4096,
            stage: Some(checking)
        }
    );
    assert!(app.frame(&f, &i).hits.is_empty(), "nothing on the page to tap while it is checked");
    assert_eq!(
        app.key(Key::Back, 2000),
        vec![Effect::Cancel],
        "checking only reads, so leaving stops it"
    );
    assert_eq!((app.screen(), &app.state.install), (Screen::Install, &Install::Ending));
    app.install(Event::Finished(Outcome::Cancelled), 2000);
    assert_eq!(app.state.install, Install::Idle);
}
#[test]
fn how_skyward_went_back_stays_until_the_next_install() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Script, 0);
    app.install(Event::Running, 0);
    let writing = Stage::Reflashing {
        done: 1 << 20,
        total: 4 << 20,
    };
    app.install(Event::Progress(writing.clone()), 0);
    assert_eq!(
        app.state.install,
        Install::Scripting {
            stage: Some(writing),
            running: true
        }
    );
    let why = "write /dev/block/by-name/recovery_a: Input/output error (os error 5)";
    app.install(Event::Reflashed(Reflash::Failed(why.into())), 0);
    app.install(Event::Finished(Outcome::Failed(Failure::Script(1))), 0);
    assert_eq!(app.state.reflash, Some(Reflash::Failed(why.into())));
    tap_at(&mut app, &f, &i, Action::Sideload, 1000);
    assert_eq!(app.state.reflash, None, "sending again starts afresh");
    app.install(Event::Waiting, 1000);
    app.install(Event::Installing { version: None }, 1000);
    app.install(Event::Progress(Stage::Reflashing { done: 1, total: 2 }), 1000);
    assert!(
        matches!(
            app.state.install,
            Install::Updating {
                stage: Some(Stage::Reflashing { .. }),
                ..
            }
        ),
        "an update writes it back too"
    );
    app.install(Event::Reflashed(Reflash::Written(vec![Slot::A])), 1000);
    app.install(
        Event::Finished(Outcome::Installed {
            slot: Slot::A,
            wipe: false,
        }),
        1000,
    );
    assert_eq!(app.state.reflash, Some(Reflash::Written(vec![Slot::A])));
}
#[test]
fn work_the_recovery_does_not_do_is_shown_with_where_to_go() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Unhandled, W, H, PPD);
    app.set_asked(Asked {
        unhandled: Some("--update_package=/data/ota_package/update.zip".into()),
        ..Asked::default()
    });
    assert_eq!(
        tap(&mut app, &f, &i, Action::RebootTo(Target::System)),
        vec![Effect::Reboot(Target::System)]
    );
    assert_eq!(tap(&mut app, &f, &i, Action::Dismiss), vec![Effect::Dismiss]);
    assert_eq!(app.screen(), Screen::Home);
}
/// Drag the knob of a swipe-to-confirm track all the way across.
fn swipe(app: &mut App, f: &Fonts, i: &DeviceInfo, which: Confirm) -> Vec<Effect> {
    let track = app
        .frame(f, i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(which))
        .expect("the track is on the page")
        .rect;
    let y = track.centre().1 - app.top().1;
    let mut fx = app.touch(Touch::Down(track.x + track.h / 2.0, y), f, 0);
    fx.extend(app.touch(Touch::Move(track.x + track.w / 2.0, y), f, 0));
    fx.extend(app.touch(Touch::Move(track.x + track.w, y), f, 0));
    fx.extend(app.touch(Touch::Up, f, 0));
    fx
}
#[test]
fn every_unfinished_entry_now_opens_its_page() {
    let (f, i) = (
        fonts(),
        DeviceInfo {
            slot: Some(Slot::B),
            ..info()
        },
    );
    let routes: [(Screen, Screen); 16] = [
        (Screen::Home, Screen::Search),
        (Screen::Home, Screen::Backup),
        (Screen::Home, Screen::Restore),
        (Screen::Home, Screen::Advanced),
        (Screen::Advanced, Screen::Files),
        (Screen::Advanced, Screen::Terminal),
        (Screen::Advanced, Screen::Relabel),
        (Screen::Advanced, Screen::Filesystems),
        (Screen::Advanced, Screen::Logs),
        (Screen::Advanced, Screen::Update),
        (Screen::Advanced, Screen::Avb),
        (Screen::Settings, Screen::Language),
        (Screen::Settings, Screen::TimeZone),
        (Screen::About, Screen::Logs),
        (Screen::Specs, Screen::License),
        (Screen::Specs, Screen::FontLicense),
    ];
    for (from, to) in routes {
        let mut app = App::new(from, W, H, PPD);
        tap(&mut app, &f, &i, Action::Open(to));
        assert_eq!(app.screen(), to, "from {from:?}");
    }
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Slot));
    assert_eq!(app.screen(), Screen::Slot);
}
#[test]
fn tools_have_one_primary_entry_and_updates_own_the_mirror_setting() {
    let (f, i) = (fonts(), info());
    let mut settings = App::new(Screen::Settings, W, H, PPD);
    let actions: Vec<_> = settings.frame(&f, &i).hits.iter().map(|hit| hit.action).collect();
    assert!(!actions.contains(&Action::Open(Screen::Terminal)));
    assert!(!actions.contains(&Action::Toggle(Switch::Mirror)));
    let mut advanced = App::new(Screen::Advanced, W, H, PPD);
    let actions: Vec<_> = advanced.frame(&f, &i).hits.iter().map(|hit| hit.action).collect();
    assert!(!actions.contains(&Action::OpenControlCenter));
    for screen in [
        Screen::Files,
        Screen::Browser,
        Screen::Terminal,
        Screen::Update,
        Screen::RootModules,
        Screen::Partitions,
        Screen::Avb,
        Screen::Filesystems,
        Screen::Relabel,
        Screen::Logs,
    ] {
        assert_eq!(
            actions.iter().filter(|&&action| action == Action::Open(screen)).count(),
            1,
            "{screen:?}"
        );
    }
    let mut update = App::new(Screen::SkywardUpdate, W, H, PPD);
    let enabled = update.state.mirror;
    let effects = tap(&mut update, &f, &i, Action::Toggle(Switch::Mirror));
    assert_eq!(update.state.mirror, !enabled);
    assert_eq!(effects, [Effect::SetMirror(!enabled)]);
}

#[test]
fn browser_menu_has_one_mode_entry_and_keeps_toolbar_features_reachable() {
    let (f, i) = (fonts(), info());
    for full in [false, true] {
        let mut app = App::new(Screen::BrowserMenu, W, H, PPD);
        app.state.browser.full_active = full;
        let actions: Vec<_> = app.frame(&f, &i).hits.iter().map(|hit| hit.action).collect();
        assert!(actions.contains(&Action::BrowserTabs));
        assert!(actions.contains(&Action::BrowserHome));
        assert!(actions.contains(&Action::BrowserBookmark));
        for retired in [
            Action::BrowserFull,
            Action::BrowserNewTab,
            Action::BrowserHistoryList,
            Action::BrowserBookmarksList,
            Action::BrowserDownloads,
        ] {
            assert!(!actions.contains(&retired), "duplicate menu entry: {retired:?}");
        }
        assert_eq!(actions.contains(&Action::BrowserFullLoad), !full);
        assert_eq!(actions.contains(&Action::BrowserBasic), full);
        app.act(Action::BrowserTabs, 0);
        assert!(app.frame(&f, &i).hits.iter().any(|hit| hit.action == Action::BrowserNewTab));
        app.act(Action::BrowserHome, 1);
        let dock = app.frame(&f, &i).dock.as_ref().unwrap();
        for action in [Action::BrowserHistoryList, Action::BrowserBookmarksList, Action::BrowserDownloads] {
            assert_eq!(dock.hits.iter().filter(|hit| hit.action == action).count(), 1);
        }
    }
}

#[test]
fn install_safety_options_remain_available_after_copy_cleanup() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Install, W, H, PPD);
    app.state.recovery_partitions = true;
    let actions: Vec<_> = app.frame(&f, &i).hits.iter().map(|hit| hit.action).collect();
    for action in [
        Action::Toggle(Switch::Signature),
        Action::Toggle(Switch::KeepRecovery),
        Action::Toggle(Switch::DisableAvb),
    ] {
        assert_eq!(actions.iter().filter(|&&shown| shown == action).count(), 1);
    }
    assert!(app.state.keep_recovery);
}

#[test]
fn logs_are_exported_as_ticked_where_they_can_go() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Logs, W, H, PPD);
    let both = Effect::ExportLogs {
        recovery: true,
        kernel: true,
        storage: false,
    };
    assert_eq!(
        tap(&mut app, &f, &i, Action::ExportLogs),
        vec![both],
        "both, to the temporary folder, at first"
    );
    tap(&mut app, &f, &i, Action::LogPart(0));
    tap(&mut app, &f, &i, Action::LogsTo(1));
    assert!(
        tap(&mut app, &f, &i, Action::ExportLogs).is_empty(),
        "internal storage waits for the data partition"
    );
    app.tick(1000);
    assert_eq!(app.deadline(1000), Some(TOAST_MS), "and says so");
    app.data_waiting(None, Vec::new());
    assert_eq!(
        tap(&mut app, &f, &i, Action::ExportLogs),
        vec![Effect::ExportLogs {
            recovery: false,
            kernel: true,
            storage: true
        }]
    );
    tap(&mut app, &f, &i, Action::LogPart(1));
    assert!(
        tap(&mut app, &f, &i, Action::ExportLogs).is_empty(),
        "nothing ticked, nothing saved"
    );
}
#[test]
fn a_time_zone_chosen_is_the_one_kept() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::TimeZone));
    assert_eq!(app.state.time_zone, 0, "China until another is chosen");
    let london = TIME_ZONES
        .iter()
        .position(|&(_, _, name)| name == "Europe/London")
        .expect("London is listed");
    tap(&mut app, &f, &i, Action::TimeZone(london));
    assert_eq!(app.state.time_zone, london);
    tap(&mut app, &f, &i, Action::TimeZone(0));
    assert_eq!(app.state.time_zone, 0, "and China again");
}
#[test]
fn a_backup_takes_what_is_ticked_to_where_it_is_sent() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Backup, &f, &i);
    app.state.efs_backup_available = true;
    assert_eq!(app.state.backup, [true, true, false, true]);
    assert!(
        swipe(&mut app, &f, &i, Confirm::Backup).is_empty(),
        "internal storage waits for the data partition"
    );
    assert_eq!(app.deadline(0), Some(TOAST_MS), "and says so");
    app.data_waiting(Some((10, Keys::Digits)), Vec::new());
    let first = Effect::Backup {
        parts: [true, true, false, true],
        place: Place::Storage,
        compress: false,
        verify: true,
    };
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Backup), vec![first]);
    tap(&mut app, &f, &i, Action::BackupPart(2));
    assert!(
        swipe(&mut app, &f, &i, Confirm::Backup).is_empty(),
        "the data partition is backed up only with every user's files open"
    );
    app.data_waiting(None, Vec::new());
    tap(&mut app, &f, &i, Action::BackupPart(0));
    assert_eq!(app.state.backup, [false, true, true, true]);
    tap(&mut app, &f, &i, Action::BackupTo(Place::UsbDrive));
    tap(&mut app, &f, &i, Action::Toggle(Switch::Compress));
    tap(&mut app, &f, &i, Action::Toggle(Switch::Verify));
    assert!(swipe(&mut app, &f, &i, Confirm::Backup).is_empty(), "no drive is plugged in");
    app.set_usb_drive(Some(UsbDrive::Shown));
    let second = Effect::Backup {
        parts: [false, true, true, true],
        place: Place::UsbDrive,
        compress: true,
        verify: false,
    };
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Backup), vec![second]);
    for k in 1..4 {
        tap(&mut app, &f, &i, Action::BackupPart(k));
    }
    assert!(
        swipe(&mut app, &f, &i, Confirm::Backup).is_empty(),
        "nothing ticked, nothing backed up"
    );
}
#[test]
fn unavailable_efs_cannot_be_selected_or_silently_backed_up() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Backup, &f, &i);
    app.state.efs_backup_available = false;
    app.state.data = Data::Open;
    app.state.backup = [false; 4];
    assert!(
        app.frame(&f, &i).hits.iter().all(|hit| hit.action != Action::BackupPart(3)),
        "an unconfigured EFS option is not shown"
    );
    app.act(Action::BackupPart(3), 0);
    assert!(!app.state.backup[3]);
    app.state.backup[3] = true;
    assert!(swipe(&mut app, &f, &i, Confirm::Backup).is_empty());
}
/// A backup found in `place`, made at `made`, holding `parts`.
fn found(place: Place, made: i64, parts: [bool; 4]) -> BackupEntry {
    let held = Held {
        made,
        size: 6_200_000_000,
        parts,
        device: "ishtar".into(),
        system: Some("OS3.0.305.0.WMACNXM".into()),
        slot: Some(Slot::B),
    };
    BackupEntry {
        place,
        name: fmt::stamp(made, 0),
        held: Ok(held),
        image: None,
    }
}
#[test]
fn backups_found_are_listed_newest_first_and_those_unread_last() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Restore)),
        vec![Effect::FindBackups {
            storage: false,
            usb: false
        }]
    );
    let unread = BackupEntry {
        place: Place::Storage,
        name: "20261021-150000".into(),
        held: Err("read backup.toml: not found".into()),
        image: None,
    };
    let (old, new) = (
        found(Place::UsbDrive, 1_792_000_000, [true; 4]),
        found(Place::Storage, 1_792_348_260, [true; 4]),
    );
    app.set_backups(vec![unread.clone(), old.clone(), new.clone()]);
    assert_eq!(app.state.backups, [new, old, unread]);
    let opens = app
        .frame(&f, &i)
        .hits
        .iter()
        .filter(|h| matches!(h.action, Action::OpenBackup(_)))
        .count();
    assert_eq!(opens, 3, "each opens, one that cannot be read to be deleted");
}
#[test]
fn a_backup_found_opens_to_choose_what_to_restore() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Restore, &f, &i);
    assert!(app.frame(&f, &i).hits.is_empty(), "nothing found, nothing to open");
    app.set_backups(vec![found(Place::Storage, 1_792_348_260, [true, true, false, true])]);
    tap(&mut app, &f, &i, Action::OpenBackup(0));
    assert_eq!(app.screen(), Screen::RestoreBackup);
    let parts: Vec<Action> = app
        .frame(&f, &i)
        .hits
        .iter()
        .map(|h| h.action)
        .filter(|a| matches!(a, Action::RestorePart(_)))
        .collect();
    assert_eq!(
        parts,
        [Action::RestorePart(0), Action::RestorePart(1), Action::RestorePart(3)],
        "only what the backup holds"
    );
    tap(&mut app, &f, &i, Action::RestorePart(1));
    assert_eq!(app.state.restore.as_ref().map(|r| r.1), Some([true, false, false, true]));
    let name = fmt::stamp(1_792_348_260, 0);
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::Restore),
        vec![Effect::Restore {
            place: Place::Storage,
            name: name.clone(),
            parts: [true, false, false, true]
        }]
    );
    for k in [0, 3] {
        tap(&mut app, &f, &i, Action::RestorePart(k));
    }
    assert!(
        swipe(&mut app, &f, &i, Confirm::Restore).is_empty(),
        "nothing ticked, nothing restored"
    );
    assert_eq!(
        app.key(Key::Back, 0),
        vec![Effect::FindBackups {
            storage: false,
            usb: false
        }],
        "back on the list, it is looked for again"
    );
    assert_eq!((app.screen(), app.state.restore.clone()), (Screen::Restore, None));
}
#[test]
fn a_data_backup_is_restored_only_with_every_users_files_open() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Restore, &f, &i);
    app.set_backups(vec![found(Place::Storage, 1_792_348_260, [false, false, true, false])]);
    tap(&mut app, &f, &i, Action::OpenBackup(0));
    assert!(swipe(&mut app, &f, &i, Confirm::Restore).is_empty());
    assert_eq!(app.deadline(0), Some(TOAST_MS), "the page says to decrypt first");
    app.data_waiting(None, Vec::new());
    let fx = swipe(&mut app, &f, &i, Confirm::Restore);
    assert!(
        matches!(
            &fx[..],
            [Effect::Restore {
                parts: [false, false, true, false],
                ..
            }]
        ),
        "{fx:?}"
    );
}
#[test]
fn a_backup_made_on_another_phone_is_not_restored_here() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Restore, &f, &i);
    let mut other = found(Place::Storage, 1_792_348_260, [true; 4]);
    other.held.as_mut().unwrap().device = "aurora".into();
    let unread = BackupEntry {
        place: Place::Storage,
        name: "20261021-150000".into(),
        held: Err("read backup.toml: not found".into()),
        image: None,
    };
    app.set_backups(vec![other, unread]);
    for k in 0..2 {
        tap(&mut app, &f, &i, Action::OpenBackup(k));
        let actions: Vec<Action> = app.frame(&f, &i).hits.iter().map(|h| h.action).collect();
        assert!(
            !actions.contains(&Action::Confirm(Confirm::Restore)) && actions.contains(&Action::AskDelete),
            "{k}: {actions:?}"
        );
        app.key(Key::Back, 0);
    }
}
#[test]
fn a_backup_is_deleted_once_a_swipe_confirms_it() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Restore, &f, &i);
    app.data_waiting(None, Vec::new());
    let entry = found(Place::Storage, 1_792_348_260, [true; 4]);
    app.set_backups(vec![entry.clone()]);
    tap(&mut app, &f, &i, Action::OpenBackup(0));
    assert!(
        app.frame(&f, &i)
            .hits
            .iter()
            .all(|h| h.action != Action::Confirm(Confirm::DeleteBackup)),
        "asked for first"
    );
    tap(&mut app, &f, &i, Action::AskDelete);
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::DeleteBackup),
        vec![Effect::DeleteBackup {
            place: Place::Storage,
            name: entry.name
        }]
    );
    assert!(app.backup_deleted(Err("Permission denied".into()), 0).is_empty());
    assert_eq!(app.screen(), Screen::RestoreBackup, "not deleted, the page stays and says why");
    assert_eq!(
        app.backup_deleted(Ok(()), 0),
        vec![Effect::FindBackups { storage: true, usb: false }]
    );
    assert_eq!((app.screen(), app.state.restore.clone()), (Screen::Restore, None));
}
#[test]
fn a_backup_on_a_drive_pulled_out_leaves_its_page() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.set_usb_drive(Some(UsbDrive::Shown));
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Restore)),
        vec![Effect::FindBackups { storage: false, usb: true }]
    );
    app.set_backups(vec![found(Place::UsbDrive, 1_792_348_260, [true; 4])]);
    tap(&mut app, &f, &i, Action::OpenBackup(0));
    assert_eq!(
        app.set_usb_drive(None),
        vec![Effect::FindBackups {
            storage: false,
            usb: false
        }]
    );
    assert_eq!((app.screen(), app.state.restore.clone()), (Screen::Restore, None));
    assert_eq!(
        app.set_usb_drive(Some(UsbDrive::Shown)),
        vec![Effect::FindBackups { storage: false, usb: true }],
        "one plugged in is looked in"
    );
}
#[test]
fn backing_up_shows_how_far_it_has_come_and_stops_when_asked() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Backup, &f, &i);
    app.data_waiting(None, Vec::new());
    app.backup_started(false, Place::Storage, true);
    assert_eq!(app.screen(), Screen::BackupJob);
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::BackupJob, "the page stays while the backup goes on");
    app.backup_plan(vec![("boot".into(), 200), ("system".into(), 800)]);
    app.backup_at(BackupAt {
        item: 1,
        done: 400,
        of: 800,
        checking: false,
    });
    assert_eq!(tap(&mut app, &f, &i, Action::StopBackup), vec![Effect::StopBackup]);
    assert!(app.state.backup_job.as_ref().unwrap().stopping);
    assert!(
        app.frame(&f, &i).hits.iter().all(|h| h.action != Action::StopBackup),
        "only one stop is asked for"
    );
    app.backup_ended(BackupEnd::Stopped, 0);
    tap_at(&mut app, &f, &i, Action::Back, 1000);
    assert_eq!((app.screen(), app.state.backup_job.clone()), (Screen::Backup, None));
}
#[test]
fn a_restore_checks_its_backup_first_and_goes_on_to_its_end_once_writing() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Restore, &f, &i);
    app.set_backups(vec![found(Place::Storage, 1_792_348_260, [true; 4])]);
    tap(&mut app, &f, &i, Action::OpenBackup(0));
    app.backup_started(true, Place::Storage, true);
    app.backup_plan(vec![("boot".into(), 200), ("data".into(), 800)]);
    app.backup_at(BackupAt {
        item: 0,
        done: 100,
        of: 200,
        checking: true,
    });
    let stops = |app: &mut App| app.frame(&f, &i).hits.iter().any(|h| h.action == Action::StopBackup);
    assert!(stops(&mut app), "nothing is written while the backup is checked");
    app.backup_at(BackupAt {
        item: 0,
        done: 0,
        of: 400,
        checking: false,
    });
    assert!(!stops(&mut app), "what has begun to be written is not stopped");
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::BackupJob);
    app.backup_ended(
        BackupEnd::Restored {
            unmade: vec!["/data/user/10".into()],
        },
        0,
    );
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), 1000),
        vec![Effect::Reboot(Target::System)]
    );
}
#[test]
fn the_slot_page_switches_to_the_slot_picked() {
    let (f, i) = (
        fonts(),
        DeviceInfo {
            slot: Some(Slot::B),
            ..info()
        },
    );
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Slot)),
        vec![Effect::ReadSlots],
        "read as the page opens"
    );
    let held = BootSlots {
        active: Slot::B,
        bootable: [false, true],
        successful: [true, true],
    };
    app.set_boot_slots(Ok(held));
    assert!(swipe(&mut app, &f, &i, Confirm::Slot).is_empty());
    assert_eq!(app.deadline(0), Some(TOAST_MS), "B starts next already");
    tap(&mut app, &f, &i, Action::PickSlot(Slot::A));
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Slot), vec![Effect::SetSlot(Slot::A)]);
    let switched = BootSlots {
        active: Slot::A,
        bootable: [true, true],
        ..held
    };
    app.slot_switched(Ok(switched), 0);
    assert_eq!(
        (app.state.boot_slots.clone(), app.state.next_slot),
        (Some(Ok(switched)), None),
        "the pick is done with"
    );
    tap(&mut app, &f, &i, Action::PickSlot(Slot::B));
    app.key(Key::Back, 0);
    assert_eq!(app.state.next_slot, None, "a pick left unswitched is put down");
}
#[test]
fn the_slot_stays_as_an_update_in_flight_set_it() {
    let (f, i) = (
        fonts(),
        DeviceInfo {
            slot: Some(Slot::B),
            snapshot: SnapshotState::Snapshotted,
            ..info()
        },
    );
    let mut app = App::new(Screen::Slot, W, H, PPD);
    app.set_boot_slots(Ok(BootSlots {
        active: Slot::A,
        bootable: [true, true],
        successful: [false, true],
    }));
    assert!(
        app.frame(&f, &i).hits.iter().all(|h| !matches!(h.action, Action::Confirm(_))),
        "nothing to swipe"
    );
}
/// The systems with a second one installed, the main one starting.
fn two_systems() -> Overview {
    Overview {
        installed: skyward_systems::Named {
            name: "HyperOS 3".into(),
            version: Some("OS3.0.308.0.WMACNXM".into()),
        },
        other: Some(skyward_systems::Other {
            id: "ubuntu".into(),
            name: "Ubuntu Touch".into(),
            version: "24.04".into(),
            size: 16 << 30,
        }),
        other_starts: false,
        changed: false,
        held: skyward_systems::Held::Recorded,
    }
}
/// The systems page opened from the home page of a phone that can hold
/// a second system, with the systems read.
fn systems_page(f: &Fonts, i: &DeviceInfo, systems: Overview) -> App {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.dual_boot = true;
    assert_eq!(
        tap(&mut app, f, i, Action::Open(Screen::Systems)),
        vec![Effect::ReadSystems],
        "read as the page opens"
    );
    app.set_systems(Ok(systems));
    app
}
fn swipes(app: &mut App, f: &Fonts, i: &DeviceInfo) -> bool {
    app.frame(f, i).hits.iter().any(|h| matches!(h.action, Action::Confirm(_)))
}
#[test]
fn the_home_page_offers_the_systems_where_the_phone_can_hold_a_second() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    assert!(
        app.frame(&f, &i).hits.iter().all(|h| h.action != Action::Open(Screen::Systems)),
        "no init_boot partitions in both slots"
    );
    let app = systems_page(&f, &i, two_systems());
    assert_eq!(app.screen(), Screen::Systems);
    let named = |systems| {
        let mut app = App::new(Screen::Home, W, H, PPD);
        (app.state.dual_boot, app.state.systems) = (true, Some(Ok(systems)));
        let t = app.theme();
        let cx = Ctx {
            t: &t,
            f: &f,
            info: &i,
            state: &app.state,
            term: None,
            band: None,
        };
        pages::render(Screen::Home, &cx).canvas.data.clone()
    };
    assert!(
        named(two_systems())
            != named(Overview {
                other_starts: true,
                ..two_systems()
            }),
        "the home page names the one that starts"
    );
}
#[test]
fn the_systems_page_switches_to_the_system_picked() {
    let (f, i) = (fonts(), info());
    let mut app = systems_page(&f, &i, two_systems());
    assert!(!swipes(&mut app, &f, &i), "the main system starts already");
    tap(&mut app, &f, &i, Action::PickSystem(Who::Other));
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::SwitchSystem),
        vec![Effect::SwitchSystem(Who::Other)]
    );
    assert_eq!(
        (app.state.systems_work, app.state.system_pick),
        (Some(SystemsWork::Switching(Who::Other)), None)
    );
    assert!(
        app.frame(&f, &i).hits.is_empty(),
        "nothing to tap while the boot partitions are written"
    );
    app.tick(1000);
    app.key(Key::Back, 1000);
    assert_eq!(
        (app.screen(), app.deadline(1000)),
        (Screen::Systems, Some(1000 + TOAST_MS)),
        "the page stays until they are, and says why"
    );
    app.systems_work_failed("the store's header does not check", 0);
    assert_eq!(app.state.systems_work, None);
    app.set_systems(Ok(Overview {
        other_starts: true,
        ..two_systems()
    }));
    tap(&mut app, &f, &i, Action::PickSystem(Who::Installed));
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::SwitchSystem),
        vec![Effect::SwitchSystem(Who::Installed)]
    );
    app.set_systems(Ok(two_systems()));
    tap(&mut app, &f, &i, Action::PickSystem(Who::Other));
    app.key(Key::Back, 0);
    assert_eq!(
        (app.screen(), app.state.system_pick),
        (Screen::Home, None),
        "a pick left unswitched is put down"
    );
}
#[test]
fn the_second_system_waits_for_an_update_in_flight_and_the_main_one_does_not() {
    let (f, i) = (
        fonts(),
        DeviceInfo {
            snapshot: SnapshotState::Snapshotted,
            ..info()
        },
    );
    let mut app = systems_page(&f, &i, two_systems());
    tap(&mut app, &f, &i, Action::PickSystem(Who::Other));
    assert!(!swipes(&mut app, &f, &i), "nothing to swipe");
    app.set_systems(Ok(Overview {
        other_starts: true,
        ..two_systems()
    }));
    tap(&mut app, &f, &i, Action::PickSystem(Who::Installed));
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::SwitchSystem),
        vec![Effect::SwitchSystem(Who::Installed)],
        "back to the main system to merge it"
    );
}
#[test]
fn externally_rewritten_boot_requires_an_explicit_main_system_confirmation() {
    let (f, i) = (fonts(), info());
    let rewritten = Overview {
        held: skyward_systems::Held::Rewritten,
        other_starts: true,
        ..two_systems()
    };
    let mut app = systems_page(&f, &i, rewritten);
    assert!(!swipes(&mut app, &f, &i));
    tap(&mut app, &f, &i, Action::PickSystem(Who::Installed));
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::SwitchSystem),
        vec![Effect::SwitchSystem(Who::Installed)]
    );
}
#[test]
fn an_unrecorded_second_system_offers_no_saved_main_boot_write() {
    let (f, i) = (fonts(), info());
    let unknown = Overview {
        held: skyward_systems::Held::Unrecorded,
        other: None,
        ..two_systems()
    };
    let mut app = systems_page(&f, &i, unknown);
    tap(&mut app, &f, &i, Action::PickSystem(Who::Installed));
    assert!(!swipes(&mut app, &f, &i));
}
#[test]
fn the_second_system_is_removed_once_asked_and_swiped() {
    let (f, i) = (fonts(), info());
    let mut app = systems_page(&f, &i, two_systems());
    assert!(
        app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Open(Screen::Decrypt)),
        "the data partition is opened first"
    );
    app.state.data = Data::Open;
    app.refresh();
    tap(&mut app, &f, &i, Action::AskDelete);
    assert_eq!(swipe(&mut app, &f, &i, Confirm::RemoveSystem), vec![Effect::RemoveSystem]);
    assert_eq!(
        (app.state.systems_work, app.state.asking_delete),
        (Some(SystemsWork::Removing), false)
    );
    app.set_systems(Ok(Overview {
        other: None,
        ..two_systems()
    }));
    assert_eq!(app.state.systems_work, None);
    assert!(
        app.frame(&f, &i).hits.iter().all(|h| h.action != Action::AskDelete),
        "nothing left to remove"
    );
    tap(&mut app, &f, &i, Action::Open(Screen::Install));
    assert_eq!(app.screen(), Screen::Install, "the empty place opens the install page");
}
#[test]
fn a_system_installs_beside_the_main_one_in_the_room_chosen() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    app.install(Event::Checking { size: 3 << 30 }, 0);
    app.install(
        Event::System {
            name: "Ubuntu Touch".into(),
            version: "24.04".into(),
            sizes: vec![3 << 30, 8 << 30, 16 << 30],
        },
        0,
    );
    tap_at(&mut app, &f, &i, Action::SystemSize(1), 1000);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(Confirm::InstallSystem))
        .unwrap()
        .rect;
    let y = track.centre().1 - app.top().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 1000);
    app.touch(Touch::Move(track.x + track.w, y), &f, 1000);
    assert_eq!(app.touch(Touch::Up, &f, 1000), vec![Effect::InstallSystem { size: 8 << 30 }]);
    assert_eq!(
        app.state.install,
        Install::Systeming {
            name: "Ubuntu Touch".into(),
            stage: None
        }
    );
    app.install(
        Event::Progress(Stage::Root {
            done: 1 << 30,
            total: 6 << 30,
        }),
        1000,
    );
    assert_eq!(
        app.state.install,
        Install::Systeming {
            name: "Ubuntu Touch".into(),
            stage: Some(Stage::Root {
                done: 1 << 30,
                total: 6 << 30
            })
        }
    );
    app.key(Key::Back, 1000);
    assert_eq!(app.screen(), Screen::Installing, "the root being written keeps the page");
    app.install(
        Event::Finished(Outcome::SystemInstalled {
            name: "Ubuntu Touch".into(),
        }),
        1000,
    );
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::Open(Screen::Systems), 2000),
        vec![Effect::ReadSystems]
    );
    assert_eq!(app.screen(), Screen::Systems);
}
#[test]
fn a_switch_asked_for_shows_before_the_systems_are_read() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.dual_boot = true;
    app.switch_asked(Who::Installed);
    assert_eq!(
        (app.screen(), app.state.systems_work),
        (Screen::Systems, Some(SystemsWork::Switching(Who::Installed)))
    );
    assert!(app.frame(&f, &i).hits.is_empty());
}
#[test]
fn a_stopped_second_system_says_why_on_its_page_and_from_the_home_card() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.dual_boot = true;
    app.system_stopped("no partition userdata showed within 15s".into());
    assert_eq!(app.screen(), Screen::Systems);
    app.set_systems(Ok(Overview {
        other_starts: true,
        ..two_systems()
    }));
    let page = app.frame(&f, &i);
    assert!(
        page.hits.iter().any(|h| h.action == Action::PickSystem(Who::Installed)),
        "the main system is there to switch back to"
    );
    assert!(
        !page.hits.iter().any(|h| h.action == Action::Confirm(Confirm::SwitchSystem)),
        "nothing is switched until one is picked"
    );
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Home);
    let opens = app
        .frame(&f, &i)
        .hits
        .iter()
        .filter(|h| h.action == Action::Open(Screen::Systems))
        .count();
    assert_eq!(opens, 2, "the card says so above the entry to the systems");
}
/// Whether the page shows a key of the keypad or keyboard to tap.
fn keys_shown(app: &mut App, f: &Fonts, i: &DeviceInfo) -> bool {
    app.frame(f, i).hits.iter().any(|h| matches!(h.action, Action::Type(_)))
}
#[test]
fn the_decrypt_page_opens_the_partition_and_then_asks_with_the_users_own_control() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Decrypt)),
        vec![Effect::OpenData],
        "no swipe first"
    );
    assert_eq!(app.state.data, Data::Opening);
    assert!(
        !keys_shown(&mut app, &f, &i) && app.frame(&f, &i).pattern.is_none(),
        "nothing to answer yet"
    );
    app.data_waiting(Some((0, Keys::Pattern)), Vec::new());
    assert!(app.frame(&f, &i).pattern.is_some(), "the owner set a pattern, so the grid shows");
    app.data_waiting(Some((0, Keys::Digits)), Vec::new());
    assert!(keys_shown(&mut app, &f, &i), "for a code, the keypad");
    app.key(Key::Back, 0);
    assert!(
        tap(&mut app, &f, &i, Action::Open(Screen::Decrypt)).is_empty(),
        "open already, it is not opened again"
    );
    let mut first = App::new(Screen::Decrypt, W, H, PPD);
    assert_eq!(
        first.arrive(),
        vec![Effect::OpenData],
        "a recovery that starts on the page opens it as well"
    );
}
#[test]
fn leaving_the_decrypt_page_goes_back_to_where_it_was_opened_from() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Decrypt, &f, &i);
    tap(&mut app, &f, &i, Action::Skip);
    assert_eq!(
        (app.screen(), app.stack.len()),
        (Screen::Home, 1),
        "back on the home page, with no other behind it"
    );
    let mut first = App::new(Screen::Decrypt, W, H, PPD);
    first.arrive();
    tap(&mut first, &f, &i, Action::Skip);
    assert_eq!(
        (first.screen(), first.stack.len()),
        (Screen::Home, 1),
        "a recovery that started on the page goes on to the home page"
    );
}
#[test]
fn the_code_typed_on_the_keypad_is_sent_once_and_checked_before_more_is_taken() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Decrypt, W, H, PPD);
    app.data_waiting(Some((0, Keys::Digits)), Vec::new());
    for ch in "1234".chars() {
        tap(&mut app, &f, &i, Action::Type(Cap::Char(ch)));
    }
    assert_eq!(app.state.code, "1234");
    tap(&mut app, &f, &i, Action::Type(Cap::Backspace));
    assert_eq!(app.state.code, "123");
    let fx = tap(&mut app, &f, &i, Action::Type(Cap::Enter));
    assert_eq!(
        fx,
        vec![Effect::Unlock {
            user: 0,
            code: "123".into()
        }]
    );
    assert_eq!(app.state.code, "", "what was typed is gone once it is sent");
    assert_eq!(
        app.state.data,
        Data::Locked {
            user: 0,
            keys: Keys::Digits,
            checking: true
        }
    );
    assert!(!keys_shown(&mut app, &f, &i), "nothing more is taken while it is checked");
    app.unlock_failed("the secure world does not recognise that credential".into(), 0);
    assert!(keys_shown(&mut app, &f, &i), "turned down, it is asked for again");
    assert_eq!(app.deadline(0), Some(TOAST_MS), "with why");
    app.data_waiting(None, Vec::new());
    assert_eq!(app.state.data, Data::Open);
}
#[test]
fn nothing_typed_is_not_sent() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Decrypt, W, H, PPD);
    app.data_waiting(Some((0, Keys::Digits)), Vec::new());
    assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
    assert_eq!(app.deadline(0), Some(TOAST_MS), "the page says to type one first");
}
#[test]
fn a_pattern_is_sent_as_the_numbers_of_the_dots_it_joins() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Decrypt, W, H, PPD);
    app.data_waiting(Some((0, Keys::Pattern)), Vec::new());
    let (cx, cy, span) = app.frame(&f, &i).pattern.expect("the grid is on the page");
    let dots = widgets::pattern_dots(cx, cy, span);
    let scroll = app.top().1;
    app.touch(Touch::Down(dots[0].0, dots[0].1 - scroll), &f, 0);
    for &which in &[1usize, 4, 7] {
        app.touch(Touch::Move(dots[which].0, dots[which].1 - scroll), &f, 0);
    }
    let fx = app.touch(Touch::Up, &f, 0);
    assert_eq!(
        fx,
        vec![Effect::Unlock {
            user: 0,
            code: "1258".into()
        }],
        "the dots counted from one across the rows"
    );
    assert!(app.state.pattern.is_empty(), "the grid is cleared once it is sent");
    assert!(
        app.frame(&f, &i).hits.iter().all(|h| h.action != Action::Pattern),
        "nothing more is drawn while it is checked"
    );
}
#[test]
fn a_pattern_too_short_to_be_one_is_not_sent() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Decrypt, W, H, PPD);
    app.data_waiting(Some((0, Keys::Pattern)), Vec::new());
    let (cx, cy, span) = app.frame(&f, &i).pattern.expect("the grid is on the page");
    let dots = widgets::pattern_dots(cx, cy, span);
    let scroll = app.top().1;
    app.touch(Touch::Down(dots[0].0, dots[0].1 - scroll), &f, 0);
    app.touch(Touch::Move(dots[1].0, dots[1].1 - scroll), &f, 0);
    let fx = app.touch(Touch::Up, &f, 0);
    assert!(fx.is_empty());
    assert_eq!(app.deadline(0), Some(TOAST_MS), "the page says how many dots it takes");
}
#[test]
fn a_partition_that_did_not_open_says_why_and_is_tried_again() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Decrypt, W, H, PPD);
    app.arrive();
    app.data_failed("KeyMint is not running".into());
    assert_eq!(tap(&mut app, &f, &i, Action::OpenData), vec![Effect::OpenData]);
    assert_eq!(app.state.data, Data::Opening);
}
/// Type `text` on the keyboard of the page showing, one tap a key, in
/// the letters layer.
fn type_text(app: &mut App, f: &Fonts, i: &DeviceInfo, text: &str) {
    for ch in text.chars() {
        tap(app, f, i, Action::Type(Cap::Char(ch)));
    }
}
#[test]
fn search_finds_by_pinyin_typed_on_its_own_keyboard() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Search, &f, &i);
    type_text(&mut app, &f, &i, "bf");
    assert_eq!(app.state.search, "bf");
    let opens: Vec<Action> = app
        .frame(&f, &i)
        .hits
        .iter()
        .map(|h| h.action)
        .filter(|a| matches!(a, Action::Open(_)))
        .collect();
    assert_eq!(opens, [Action::Open(Screen::Backup)]);
    tap(&mut app, &f, &i, Action::Type(Cap::Backspace));
    assert_eq!(app.state.search, "b");
    tap(&mut app, &f, &i, Action::ClearSearch);
    assert_eq!(app.state.search, "");
    type_text(&mut app, &f, &i, "xpsj");
    tap(&mut app, &f, &i, Action::Type(Cap::Enter));
    assert_eq!(app.screen(), Screen::ScreenOff, "return opens the first result");
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Search);
}
#[test]
fn shift_makes_one_capital_and_the_layers_switch() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Search, &f, &i);
    tap(&mut app, &f, &i, Action::Type(Cap::Shift));
    type_text(&mut app, &f, &i, "ab");
    assert_eq!(app.state.search, "Ab");
    tap(&mut app, &f, &i, Action::Type(Cap::Show(Layer::Digits)));
    type_text(&mut app, &f, &i, "1");
    tap(&mut app, &f, &i, Action::Type(Cap::Show(Layer::Symbols)));
    type_text(&mut app, &f, &i, "|");
    tap(&mut app, &f, &i, Action::Type(Cap::Space));
    assert_eq!(app.state.search, "Ab1| ");
    tap(&mut app, &f, &i, Action::Type(Cap::Show(Layer::Letters)));
    assert_eq!(app.state.layer, Layer::Letters);
}
#[test]
fn a_key_shows_held_down_while_touched() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Search, &f, &i);
    let (x, y) = spot(&mut app, &f, &i, Action::Type(Cap::Char('q')));
    app.touch(Touch::Down(x, y), &f, 0);
    assert_eq!(app.state.key_held(), Some(Cap::Char('q')));
    app.touch(Touch::Up, &f, 0);
    assert_eq!(app.state.held, None);
    assert_eq!(app.state.search, "q");
}
#[test]
fn the_terminal_sends_every_key_to_its_shell() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Terminal)),
        vec![Effect::StartShell("/".into())],
        "the page asks for a shell the first time"
    );
    let (rows, cols) = app.term_size(&f);
    let min_rows = if cfg!(feature = "pinyin-input") { 18 } else { 20 };
    assert!(
        rows >= min_rows && cols >= 40,
        "a screen of {rows} rows by {cols}, reserving composition space"
    );
    app.term_started(rows, cols);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Type(Cap::Char('l'))),
        vec![Effect::TermInput(b"l".to_vec())]
    );
    assert_eq!(
        tap(&mut app, &f, &i, Action::Type(Cap::Enter)),
        vec![Effect::TermInput(b"\r".to_vec())]
    );
    assert!(tap(&mut app, &f, &i, Action::TermKey(TermKey::Ctrl)).is_empty());
    assert_eq!(
        tap(&mut app, &f, &i, Action::Type(Cap::Char('c'))),
        vec![Effect::TermInput(vec![0x03])]
    );
    assert_eq!(
        tap(&mut app, &f, &i, Action::TermKey(TermKey::Up)),
        vec![Effect::TermInput(b"\x1b[A".to_vec())]
    );
    app.term_output(b"ishtar:/ # ");
    tap_back(&mut app, &f, &i, 0);
    assert!(
        tap(&mut app, &f, &i, Action::Open(Screen::Terminal)).is_empty(),
        "the same shell goes on"
    );
    app.term_ended(TermEnd::Exited(0));
    assert!(
        tap(&mut app, &f, &i, Action::Type(Cap::Char('l'))).is_empty(),
        "nothing runs to take the keys"
    );
    assert_eq!(
        tap(&mut app, &f, &i, Action::Type(Cap::Enter)),
        vec![Effect::StartShell("/".into())]
    );
}
#[test]
fn the_file_managers_button_opens_the_terminal_in_the_folder_shown() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    app.set_files("/sdcard/Download".into(), Ok(vec![entry("a.txt", FileKind::File)]), 0);
    assert_eq!(app.frame(&f, &i).button, Some((widgets::Glyph::Terminal, Action::TerminalHere)));
    assert_eq!(
        tap_button(&mut app, &f, &i, 0),
        vec![Effect::StartShell("/sdcard/Download".into())],
        "a shell starts there"
    );
    assert_eq!(app.screen(), Screen::Terminal);
    let (rows, cols) = app.term_size(&f);
    app.term_started(rows, cols);
    app.term_ended(TermEnd::Exited(0));
    assert_eq!(
        tap(&mut app, &f, &i, Action::Type(Cap::Enter)),
        vec![Effect::StartShell("/sdcard/Download".into())],
        "Enter starts it again there"
    );
    app.term_started(rows, cols);
    tap_back(&mut app, &f, &i, 0);
    app.set_files("/data/adb".into(), Ok(Vec::new()), 0);
    assert_eq!(
        tap_button(&mut app, &f, &i, 0),
        vec![Effect::ShellTo("/data/adb".into())],
        "the shell running goes there"
    );
    assert_eq!(app.screen(), Screen::Terminal);
}
#[test]
fn the_file_page_searches_below_its_folder_and_one_button_cycles_the_order() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    app.set_files(
        "/sdcard".into(),
        Ok(vec![
            entry("alpha.txt", FileKind::File),
            entry("beta.zip", FileKind::File),
            entry("DCIM", folder(2)),
        ]),
        LISTED,
    );
    assert_eq!(
        app.state.files.shown_entries().len(),
        3,
        "every entry is shown before anything is typed"
    );
    let frame = app.frame(&f, &i);
    assert!(frame.pinned.is_some(), "the search box is pinned under the bar");
    assert!(frame.dock.is_none(), "no keyboard is drawn until the box is touched");
    assert!(tap(&mut app, &f, &i, Action::FilesSearch).is_empty(), "the box only opens the keys");
    let frame = app.frame(&f, &i);
    let dock = frame.dock.as_ref().expect("the keyboard is drawn in the strip");
    assert!(!dock.hits.is_empty(), "the keys are reachable");
    assert_eq!(app.screen(), Screen::Files, "the keyboard does not leave the page");
    let asked = tap_at(&mut app, &f, &i, Action::Type(Cap::Char('b')), 1010);
    assert_eq!(
        asked,
        vec![Effect::FindFiles {
            root: "/sdcard".into(),
            query: "b".into(),
            generation: app.state.find_gen,
            sort: SortKey::Name
        }]
    );
    assert_eq!(app.state.files_query, "b");
    assert!(app.state.finding, "the page says it is looking");
    assert_eq!(
        app.state.files.shown_entries().len(),
        3,
        "the folder's own list stays whole behind the box"
    );
    assert_eq!(
        app.frame(&f, &i).rows.as_ref().map(|r| r.count()),
        None,
        "nothing is listed until the answer comes"
    );
    app.found_files(
        app.state.find_gen,
        Ok(vec![Found {
            folder: "/sdcard/DCIM/Camera".into(),
            entry: entry("beach.jpg", FileKind::File),
        }]),
    );
    assert!(!app.state.finding);
    assert_eq!(
        app.frame(&f, &i).rows.as_ref().map(|r| r.count()),
        Some(1),
        "the entry found below the folder is the list"
    );
    assert_eq!(app.state.sort, SortKey::Name);
    for expected in [SortKey::Modified, SortKey::Kind, SortKey::Name] {
        let asked = tap_at(&mut app, &f, &i, Action::CycleSort, 1015);
        assert_eq!(
            asked,
            vec![Effect::FindFiles {
                root: "/sdcard".into(),
                query: "b".into(),
                generation: app.state.find_gen,
                sort: expected
            }],
            "the tree is walked again in the new order"
        );
        assert_eq!(app.state.sort, expected);
    }
    assert!(
        tap_at(&mut app, &f, &i, Action::ClearFilesQuery, 1020).is_empty(),
        "clearing is not a listing"
    );
    assert!(app.state.found.is_none(), "what was found is gone with the text");
    assert_eq!(app.state.files.shown_entries().len(), 3, "every entry comes back");
    assert_eq!(app.frame(&f, &i).rows.as_ref().map(|r| r.count()), Some(3));
    assert!(app.act(Action::CloseFilesSearch, 1025).is_empty());
    assert!(app.frame(&f, &i).dock.is_none(), "the keyboard goes away");
    app.act(Action::Type(Cap::Char('a')), 1030);
    assert_eq!(app.state.files_query, "a");
    app.set_files("/sdcard/DCIM".into(), Ok(vec![entry("photo.jpg", FileKind::File)]), LISTED);
    assert_eq!(app.state.files_query, "", "a folder opened is searched from nothing");
    assert_eq!(app.state.files.shown_entries().len(), 1);
}
#[test]
fn what_a_search_found_opens_where_it_is_and_not_where_the_page_is() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    app.set_files("/sdcard".into(), Ok(vec![entry("DCIM", folder(2))]), LISTED);
    tap(&mut app, &f, &i, Action::FilesSearch);
    tap_at(&mut app, &f, &i, Action::Type(Cap::Char('b')), 1000);
    app.found_files(
        app.state.find_gen,
        Ok(vec![
            Found {
                folder: "/sdcard/DCIM/Camera".into(),
                entry: entry("beach.jpg", FileKind::File),
            },
            Found {
                folder: "/sdcard/DCIM/Camera".into(),
                entry: entry("backup", folder(3)),
            },
        ]),
    );
    assert!(app.act(Action::OpenFound(0), 1005).is_empty());
    assert_eq!(app.screen(), Screen::FileInfo);
    assert_eq!(app.state.file.as_ref().map(|e| e.name.as_str()), Some("beach.jpg"));
    assert_eq!(
        app.act(Action::EditEntry, 1006),
        vec![Effect::OpenText("/sdcard/DCIM/Camera/beach.jpg".into())]
    );
    app.act(Action::Back, 1007);
    assert_eq!(app.screen(), Screen::Files);
    assert_eq!(
        app.act(Action::OpenFound(1), 1011),
        vec![Effect::ReadDir("/sdcard/DCIM/Camera".into())]
    );
    assert_eq!(app.state.files_query, "");
    assert!(app.state.found.is_none());
}
#[test]
fn an_answer_to_a_query_typed_over_is_dropped() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    app.set_files("/sdcard".into(), Ok(vec![entry("alpha.txt", FileKind::File)]), LISTED);
    tap(&mut app, &f, &i, Action::FilesSearch);
    tap_at(&mut app, &f, &i, Action::Type(Cap::Char('a')), 1000);
    let for_a = app.state.find_gen;
    tap_at(&mut app, &f, &i, Action::Type(Cap::Char('l')), 1001);
    let for_al = app.state.find_gen;
    app.found_files(
        for_a,
        Ok(vec![Found {
            folder: "/sdcard".into(),
            entry: entry("alpha.txt", FileKind::File),
        }]),
    );
    assert!(app.state.found.is_none(), "an answer to a query typed over is not the list");
    app.found_files(
        for_al,
        Ok(vec![Found {
            folder: "/sdcard/x".into(),
            entry: entry("alfa.txt", FileKind::File),
        }]),
    );
    assert!(app.state.found.is_some(), "the answer to what the box says now is");
}
#[test]
fn the_bars_round_button_darkens_under_a_finger_on_the_frames_drawn() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    app.set_files("/data/adb".into(), Ok(vec![entry("a.txt", FileKind::File)]), LISTED);
    let t = app.theme();
    let (cx, cy, _) = widgets::button_disc(&t);
    let k = (((cy - t.m.dp(5.0)) as u32 * W + (cx + t.m.dp(5.0)) as u32) * 4) as usize;
    let mut c = Canvas::new(W, H);
    let mut pixel = |app: &mut App, now: u64| {
        app.tick(now);
        app.compose(&mut c, &f, &i, "12:00", now);
        c.data[k..k + 4].to_vec()
    };
    let (x, y) = widgets::button_rect(&t).centre();
    let rest = pixel(&mut app, 0);
    app.touch(Touch::Down(x, y), &f, 100);
    assert_ne!(pixel(&mut app, 100), rest, "held down, its disc darkens");
    assert_eq!(app.touch(Touch::Up, &f, 150), vec![Effect::StartShell("/data/adb".into())]);
    for now in (150..1000).step_by(8) {
        pixel(&mut app, now);
    }
    assert_eq!(app.screen(), Screen::Terminal, "the terminal slides in");
    app.text_opened(&f, "/data/adb/a.txt".into(), Ok(Some(b"a".to_vec())), 1000);
    app.state.editor.as_mut().expect("the editor page shows a file").insert('b');
    app.refresh();
    for now in (1000..1500).step_by(8) {
        pixel(&mut app, now);
    }
    app.touch(Touch::Down(x, y), &f, 1500);
    pixel(&mut app, 1500);
    assert_eq!(
        app.touch(Touch::Up, &f, 1550),
        vec![Effect::SaveText {
            path: "/data/adb/a.txt".into(),
            text: "ba".into()
        }]
    );
}
#[test]
fn a_drag_on_the_terminal_scrolls_back_through_what_the_shell_wrote() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Terminal, W, H, PPD);
    let (rows, cols) = app.term_size(&f);
    app.term_started(rows, cols);
    let many: String = (0..100).map(|n| format!("line {n}\r\n")).collect();
    app.term_output(many.as_bytes());
    let (card, pitch) = app.frame(&f, &i).scrollback.expect("the terminal's screen scrolls back");
    let (x, y) = (card.x + card.w / 2.0, card.y + card.h / 3.0);
    app.touch(Touch::Down(x, y), &f, 0);
    app.touch(Touch::Move(x, y + pitch * 5.5), &f, 10);
    app.touch(Touch::Up, &f, 20);
    assert_eq!(
        app.term.as_ref().expect("the shell started").screen().scrollback(),
        5,
        "down shows five older rows"
    );
    assert_eq!(app.top().1, 0.0, "the page itself stays put");
}
fn entry(name: &str, kind: FileKind) -> FileRow {
    FileRow {
        name: name.into(),
        kind,
        size: 4096,
        modified: 1_792_348_260,
        mode: 0o040_755,
        uid: 0,
        gid: 0,
    }
}
/// When the tests list folders: a day after the entries changed.
const LISTED: i64 = 1_792_348_260 + 86_400;
fn folder(items: usize) -> FileKind {
    FileKind::Folder { items: Some(items) }
}
#[test]
fn the_file_manager_walks_folders_and_back_up_to_the_root() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Files)),
        vec![Effect::ReadDir("/".into())]
    );
    let link = FileKind::Link {
        target: "/storage/self/primary".into(),
        folder: true,
    };
    app.set_files(
        "/".into(),
        Ok(vec![
            entry("sdcard", link),
            entry("vendor", folder(12)),
            entry("init.rc", FileKind::File),
        ]),
        LISTED,
    );
    assert_eq!(tap(&mut app, &f, &i, Action::OpenEntry(1)), vec![Effect::ReadDir("/vendor".into())]);
    assert_eq!(
        tap(&mut app, &f, &i, Action::OpenEntry(0)),
        vec![Effect::ReadDir("/sdcard".into())],
        "a link to a folder opens like one"
    );
    app.set_files("/vendor".into(), Ok(vec![entry("etc", folder(3))]), LISTED);
    assert_eq!(app.key(Key::Back, 0), vec![Effect::ReadDir("/".into())], "back climbs a folder");
    assert_eq!(app.screen(), Screen::Files);
    app.set_files(
        "/".into(),
        Ok(vec![entry("vmlinux", FileKind::File), entry("init.rc", FileKind::File)]),
        LISTED,
    );
    assert_eq!(
        tap(&mut app, &f, &i, Action::OpenEntry(1)),
        vec![Effect::OpenText("/init.rc".into())],
        "a text file opens to edit"
    );
    assert_eq!(app.screen(), Screen::Files);
    tap(&mut app, &f, &i, Action::OpenEntry(0));
    assert_eq!(app.screen(), Screen::FileInfo);
    assert_eq!(app.state.file.as_ref().map(|e| e.name.as_str()), Some("vmlinux"));
    app.key(Key::Back, 0);
    assert_eq!(app.state.file, None);
    assert!(app.key(Key::Back, 0).is_empty(), "at the root, back leaves");
    assert_eq!(app.screen(), Screen::Advanced);
}
#[test]
fn a_file_in_internal_storage_is_chosen_once_the_storage_is_open() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Install, &f, &i);
    assert_eq!(tap(&mut app, &f, &i, Action::Open(Screen::Decrypt)), vec![Effect::OpenData]);
    app.data_waiting(None, Vec::new());
    tap(&mut app, &f, &i, Action::Skip);
    assert_eq!(app.screen(), Screen::Install);
    let storage = Screen::Choose(Place::Storage);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(storage)),
        vec![Effect::ReadPlace(Place::Storage, String::new())]
    );
    let listed = vec![
        entry(".thumbnails", folder(9)),
        entry("Download", folder(3)),
        entry("OTA.ZIP", FileKind::File),
        entry("boot.img", FileKind::File),
        entry("notes.txt", FileKind::File),
        entry(".old.zip", FileKind::File),
        entry("fifo", FileKind::Special),
    ];
    app.set_place(Place::Storage, String::new(), Ok(listed), LISTED);
    let shown: Vec<&str> = app
        .state
        .storage
        .entries
        .as_ref()
        .expect("listed")
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(
        shown,
        ["Download", "OTA.ZIP", "boot.img"],
        "folders and what can be installed, hidden ones left out"
    );
    assert_eq!(
        tap(&mut app, &f, &i, Action::ChooseEntry(0)),
        vec![Effect::ReadPlace(Place::Storage, "Download".into())]
    );
    app.set_place(
        Place::Storage,
        "Download".into(),
        Ok(vec![entry("ota.zip", FileKind::File)]),
        LISTED,
    );
    assert_eq!(
        tap(&mut app, &f, &i, Action::ChooseEntry(0)),
        vec![Effect::InstallFile(Place::Storage, "Download/ota.zip".into())]
    );
    assert_eq!((app.screen(), app.state.source), (Screen::Installing, Source::File(Place::Storage)));
    assert_eq!(
        app.state.install,
        Install::Checking { size: 4096, stage: None },
        "the file is read at once"
    );
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Installing, "a file being read keeps the page");
    app.install(Event::Finished(Outcome::Refused(Refusal::Unknown)), 0);
    tap_at(&mut app, &f, &i, Action::Back, 1000);
    assert_eq!(
        (app.screen(), &app.state.install),
        (storage, &Install::Idle),
        "back to choose another"
    );
    assert_eq!(
        app.key(Key::Back, 1000),
        vec![Effect::ReadPlace(Place::Storage, String::new())],
        "back climbs to the top"
    );
    app.set_place(Place::Storage, String::new(), Ok(Vec::new()), LISTED);
    assert!(app.key(Key::Back, 1000).is_empty());
    assert_eq!(app.screen(), Screen::Install, "from the top, back leaves");
}
#[test]
fn usb_hotplug_notifications_only_show_on_changes_and_keep_navigation_effects() {
    for (lang, connected, removed, unreadable) in [
        (Lang::Hans, "U 盘已连接", "U 盘已拔出", "U 盘已插入，无法读取"),
        (Lang::Hant, "USB 隨身碟已連接", "USB 隨身碟已拔出", "USB 隨身碟已插入，無法讀取"),
        (
            Lang::En,
            "USB drive connected",
            "USB drive removed",
            "USB drive inserted, but unreadable",
        ),
    ] {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.lang = lang;
        assert!(app.usb_drive_changed(None, 0).is_empty());
        assert!(app.toast.is_none());
        app.sleep(0);
        assert!(app.usb_drive_changed(Some(UsbDrive::Shown), 100).is_empty());
        assert!(app.display_on());
        assert_eq!(app.toast.as_ref().unwrap(), &(connected.to_string(), 100 + TOAST_MS));
        app.usb_drive_changed(Some(UsbDrive::Shown), 500);
        assert_eq!(app.toast.as_ref().unwrap().1, 100 + TOAST_MS, "polling cannot extend the toast");
        app.act(Action::OpenUsbManager, 1000);
        assert_eq!(app.usb_drive_changed(None, 2000), vec![Effect::ReadDir("/".into())]);
        assert_eq!(app.toast.as_ref().unwrap().0, removed);
        app.usb_drive_changed(Some(UsbDrive::Unreadable("I/O error".into())), 3000);
        assert_eq!(app.toast.as_ref().unwrap().0, unreadable);
        assert_eq!(app.state.install, Install::Idle);
    }
}
#[test]
fn usb_manager_is_available_on_home_without_a_duplicate_advanced_entry() {
    let (f, i) = (fonts(), info());
    for screen in [Screen::Home, Screen::Advanced] {
        let mut app = App::new(screen, W, H, PPD);
        assert_eq!(
            app.frame(&f, &i).hits.iter().any(|hit| hit.action == Action::OpenUsbManager),
            screen == Screen::Home
        );
        assert!(app.act(Action::OpenUsbManager, 0).is_empty());
        assert_eq!(app.screen(), screen);
        app.set_usb_drive(Some(UsbDrive::Unreadable("unsupported filesystem".into())));
        assert!(app.act(Action::OpenUsbManager, 0).is_empty());
        assert_eq!(app.screen(), screen);
        app.set_usb_drive(Some(UsbDrive::Shown));
        assert_eq!(app.act(Action::OpenUsbManager, 0), vec![Effect::ReadDir(USB_DRIVE.into())]);
        assert_eq!(app.screen(), Screen::Files);
        assert_eq!(app.state.files.path, USB_DRIVE);
        assert_eq!(app.state.install, Install::Idle);
        app.set_files(USB_DRIVE.into(), Ok(vec![entry("notes.txt", FileKind::File)]), LISTED);
        assert_eq!(
            app.act(Action::OpenEntry(0), 0),
            vec![Effect::OpenText(format!("{USB_DRIVE}/notes.txt"))]
        );
    }
}
#[test]
fn removing_a_usb_drive_leaves_no_stale_file_manager_entries_or_clipboard() {
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    app.set_usb_drive(Some(UsbDrive::Shown));
    app.act(Action::OpenUsbManager, 0);
    app.set_files(format!("{USB_DRIVE}/ROMs"), Ok(vec![entry("rom.zip", FileKind::File)]), LISTED);
    app.state.clip = Some(Clip {
        op: FileOp::Copy,
        path: format!("{USB_DRIVE}/ROMs/rom.zip"),
    });
    assert_eq!(app.set_usb_drive(None), vec![Effect::ReadDir("/".into())]);
    assert_eq!(app.state.files.path, "/");
    assert!(app.state.clip.is_none());
    assert!(app.state.file.is_none());
    assert_eq!(app.state.install, Install::Idle);
}
#[test]
fn a_usb_drive_shows_its_files_as_soon_as_it_is_plugged_in() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Install, &f, &i);
    let page = Screen::Choose(Place::UsbDrive);
    assert!(
        tap(&mut app, &f, &i, Action::Open(page)).is_empty(),
        "with no drive, nothing to read"
    );
    assert_eq!(app.screen(), page);
    assert!(app.frame(&f, &i).rows.is_none(), "the page waits for one");
    assert_eq!(
        app.set_usb_drive(Some(UsbDrive::Shown)),
        vec![Effect::ReadPlace(Place::UsbDrive, String::new())],
        "read once shown"
    );
    app.set_place(Place::UsbDrive, String::new(), Ok(vec![entry("ROMs", folder(2))]), LISTED);
    assert_eq!(
        tap(&mut app, &f, &i, Action::ChooseEntry(0)),
        vec![Effect::ReadPlace(Place::UsbDrive, "ROMs".into())]
    );
    app.set_place(Place::UsbDrive, "ROMs".into(), Ok(vec![entry("boot.img", FileKind::File)]), LISTED);
    assert_eq!(
        tap(&mut app, &f, &i, Action::ChooseEntry(0)),
        vec![Effect::InstallFile(Place::UsbDrive, "ROMs/boot.img".into())]
    );
    assert_eq!(app.state.source, Source::File(Place::UsbDrive));
    app.install(
        Event::Finished(Outcome::Failed(Failure::Unreadable(
            "read /mnt/usb/ROMs/boot.img: Input/output error".into(),
        ))),
        0,
    );
    tap_at(&mut app, &f, &i, Action::Back, 1000);
    assert_eq!(app.screen(), page, "back to choose another");
    assert!(app.set_usb_drive(None).is_empty());
    assert_eq!(app.state.drive.path, "", "a drive pulled out leaves nothing of its folders");
    assert!(app.frame(&f, &i).rows.is_none(), "the page waits for another");
    assert!(app.key(Key::Back, 1000).is_empty());
    assert_eq!(app.screen(), Screen::Install);
    let unreadable = Some(UsbDrive::Unreadable("sdg holds no filesystem the kernel reads".into()));
    assert!(app.set_usb_drive(unreadable).is_empty(), "another page reads nothing");
    assert!(
        tap_at(&mut app, &f, &i, Action::Open(page), 1000).is_empty(),
        "nothing on the drive can be read"
    );
}
#[test]
fn a_file_waits_for_the_install_before_it_to_end() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Install, &f, &i);
    app.data_waiting(None, Vec::new());
    tap(&mut app, &f, &i, Action::Open(Screen::Choose(Place::Storage)));
    app.set_place(Place::Storage, String::new(), Ok(vec![entry("ota.zip", FileKind::File)]), LISTED);
    app.state.install = Install::Ending;
    assert!(tap(&mut app, &f, &i, Action::ChooseEntry(0)).is_empty());
    assert_eq!(app.screen(), Screen::Choose(Place::Storage));
    assert_eq!(app.deadline(0), Some(TOAST_MS), "it says to wait");
}
/// The file manager in /sdcard, listing a folder and a file.
fn in_sdcard(f: &Fonts, i: &DeviceInfo) -> App {
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    tap(&mut app, f, i, Action::Open(Screen::Files));
    app.set_files(
        "/sdcard".into(),
        Ok(vec![entry("DCIM", folder(2)), entry("a.zip", FileKind::File)]),
        LISTED,
    );
    app
}
/// Hold the file manager's entry at `index` until the page of what can
/// be done with it opens.
fn hold_entry(app: &mut App, f: &Fonts, i: &DeviceInfo, index: usize) {
    let (x, y) = spot(app, f, i, Action::OpenEntry(index));
    app.touch(Touch::Down(x, y), f, 0);
    app.tick(LONG_PRESS_MS);
    app.touch(Touch::Up, f, LONG_PRESS_MS);
    assert_eq!(app.screen(), Screen::FileInfo);
}
#[test]
fn an_entry_held_opens_what_can_be_done_with_it_a_folder_too() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    let (x, y) = spot(&mut app, &f, &i, Action::OpenEntry(0));
    app.touch(Touch::Down(x, y), &f, 1000);
    assert_eq!(app.deadline(1000), Some(1000 + LONG_PRESS_MS), "the press is timed");
    assert!(app.tick(1000 + LONG_PRESS_MS - 1).is_empty());
    assert_eq!(
        app.tick(1000 + LONG_PRESS_MS),
        vec![Effect::Haptic],
        "held long enough, it opens with a tap of the motor"
    );
    assert_eq!(app.screen(), Screen::FileInfo);
    assert_eq!(app.state.file.as_ref().map(|e| e.name.as_str()), Some("DCIM"));
    assert!(
        app.touch(Touch::Up, &f, 1500).is_empty(),
        "lifting does not open the folder as well"
    );
    assert_eq!(app.screen(), Screen::FileInfo);
}
#[test]
fn a_picked_entry_goes_into_the_folder_the_bar_is_used_in() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    assert!(tap(&mut app, &f, &i, Action::PickEntry(FileOp::Copy)).is_empty());
    assert_eq!(app.screen(), Screen::Files, "back to the folders to pick one");
    assert_eq!(
        app.state.clip,
        Some(Clip {
            op: FileOp::Copy,
            path: "/sdcard/a.zip".into()
        })
    );
    assert!(app.frame(&f, &i).dock.is_some(), "the bar is held at the bottom");
    assert_eq!(
        tap(&mut app, &f, &i, Action::OpenEntry(0)),
        vec![Effect::ReadDir("/sdcard/DCIM".into())]
    );
    app.set_files("/sdcard/DCIM".into(), Ok(Vec::new()), LISTED);
    let paste = Effect::Paste {
        op: FileOp::Copy,
        path: "/sdcard/a.zip".into(),
        into: "/sdcard/DCIM".into(),
        clash: None,
    };
    assert_eq!(tap(&mut app, &f, &i, Action::Paste), vec![paste]);
    app.file_job_started(FileTask::Copy {
        path: "/sdcard/a.zip".into(),
        into: "/sdcard/DCIM".into(),
    });
    assert_eq!(app.state.clip, None, "the entry is put down once the copy starts");
    assert_eq!(app.screen(), Screen::FileJob);
    app.file_progress((50, 100), (0, 1));
    assert_eq!(app.key(Key::Back, 0), Vec::new());
    assert_eq!(app.screen(), Screen::FileJob, "the page stays while the copy runs");
    app.file_job_ended(JobEnd::Done, 0);
    assert_eq!(app.screen(), Screen::Files);
    assert_eq!(app.state.job, None);
}
#[test]
fn a_folder_goes_nowhere_inside_itself_and_a_move_nowhere_it_is_already() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    let paste = |app: &mut App, op: FileOp, path: &str| {
        app.state.clip = Some(Clip { op, path: path.into() });
        app.toast = None;
        app.refresh();
        tap(app, &f, &i, Action::Paste)
    };
    assert!(paste(&mut app, FileOp::Copy, "/sdcard").is_empty());
    assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("无法将\"sdcard\"放入其自身中"));
    assert!(paste(&mut app, FileOp::Move, "/sdcard/a.zip").is_empty());
    assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("\"a.zip\"已在该文件夹中"));
    assert!(app.state.clip.is_some(), "the entry stays picked, for another folder");
    let copy = Effect::Paste {
        op: FileOp::Copy,
        path: "/sdcard/a.zip".into(),
        into: "/sdcard".into(),
        clash: None,
    };
    assert_eq!(
        paste(&mut app, FileOp::Copy, "/sdcard/a.zip"),
        vec![copy],
        "a copy where the entry is makes a second one"
    );
}
#[test]
fn a_taken_name_waits_for_the_user_to_choose() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    app.state.clip = Some(Clip {
        op: FileOp::Move,
        path: "/tmp/a.zip".into(),
    });
    app.file_clash(FileOp::Move, "/tmp/a.zip".into(), "/sdcard".into(), vec!["a.zip".into()]);
    assert_eq!(app.screen(), Screen::Clash);
    let paste = Effect::Paste {
        op: FileOp::Move,
        path: "/tmp/a.zip".into(),
        into: "/sdcard".into(),
        clash: Some(ClashChoice::KeepBoth),
    };
    assert_eq!(tap(&mut app, &f, &i, Action::Resolve(ClashChoice::KeepBoth)), vec![paste]);
    assert_eq!(app.screen(), Screen::Files);
    app.file_clash(FileOp::Move, "/tmp/a.zip".into(), "/sdcard".into(), vec!["a.zip".into()]);
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Files, "turning the choice down goes back to the folder");
    assert!(app.state.clip.is_some(), "with the entry still picked, for another folder");
    app.file_clash(FileOp::Move, "/tmp/a.zip".into(), "/sdcard".into(), vec!["a.zip".into()]);
    assert!(
        tap(&mut app, &f, &i, Action::Resolve(ClashChoice::Skip)).is_empty(),
        "skipping the entry leaves nothing to do"
    );
    assert_eq!(app.screen(), Screen::Files);
    assert_eq!(app.state.clip, None, "and nothing picked");
}
#[test]
fn renaming_takes_the_name_typed() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    tap(&mut app, &f, &i, Action::RenameEntry);
    assert_eq!(app.screen(), Screen::Rename);
    assert_eq!(app.state.new_name, "a.zip", "the name it has to start from");
    for _ in 0..4 {
        tap(&mut app, &f, &i, Action::Type(Cap::Backspace));
    }
    type_text(&mut app, &f, &i, "b");
    let rename = Effect::Rename {
        path: "/sdcard/a.zip".into(),
        name: "ab".into(),
    };
    assert_eq!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)), vec![rename]);
    app.renamed(
        Err(RenameRefused::Failed(
            "rename /sdcard/a.zip to ab: No space left on device (os error 28)".into(),
        )),
        0,
    );
    assert_eq!(app.screen(), Screen::Rename, "a name refused can be typed again");
    app.toast = None;
    app.renamed(Err(RenameRefused::Taken), 0);
    assert_eq!(
        app.toast.as_ref().map(|t| t.0.as_str()),
        Some("该文件夹中已有\"ab\""),
        "as the folder compares names"
    );
    app.renamed(Ok(()), 0);
    assert_eq!(app.screen(), Screen::Files, "renamed, the folder shows again");
}
#[test]
fn permissions_switch_a_bit_at_a_time_and_show_what_the_entry_took() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    tap(&mut app, &f, &i, Action::OpenPermissions);
    assert_eq!(app.screen(), Screen::Permissions);
    let bits: Vec<u32> = app
        .frame(&f, &i)
        .hits
        .iter()
        .filter_map(|h| if let Action::ModeBit(b) = h.action { Some(b) } else { None })
        .collect();
    assert_eq!(
        bits,
        [
            0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001, 0o4000, 0o2000, 0o1000
        ]
    );
    let set = |mode| {
        vec![Effect::SetMode {
            path: "/sdcard/a.zip".into(),
            mode,
        }]
    };
    assert_eq!(tap(&mut app, &f, &i, Action::ModeBit(0o002)), set(0o757), "everyone may write");
    app.mode_set(0o757, Ok(0o757), 0);
    assert_eq!(app.state.file.as_ref().unwrap().mode, 0o040_757, "its type stays");
    assert_eq!(tap(&mut app, &f, &i, Action::ModeBit(0o100)), set(0o657), "and back off");
    app.toast = None;
    app.mode_set(0o4757, Ok(0o757), 0);
    assert_eq!(
        app.toast.as_ref().map(|t| t.0.as_str()),
        Some("该位置不支持修改权限"),
        "a filesystem that kept none of it"
    );
    app.mode_set(
        0o755,
        Err("give /sdcard/a.zip the permissions 0755: Read-only file system (os error 30)".into()),
        0,
    );
    assert!(
        app.toast
            .as_ref()
            .is_some_and(|t| t.0.starts_with("修改权限失败：give /sdcard/a.zip")),
        "{:?}",
        app.toast
    );
    assert_eq!(app.state.file.as_ref().unwrap().mode, 0o040_757, "as it last was");
    tap_back(&mut app, &f, &i, 0);
    tap_back(&mut app, &f, &i, 0);
    assert_eq!(
        app.state.files.entries.as_ref().unwrap()[1].mode,
        0o040_757,
        "the folder knows it too"
    );
}
#[test]
fn a_links_page_offers_no_permissions_of_its_own() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    app.state.file = Some(entry(
        "latest",
        FileKind::Link {
            target: "DCIM".into(),
            folder: true,
        },
    ));
    app.open(Screen::FileInfo);
    assert!(!app.frame(&f, &i).hits.iter().any(|h| h.action == Action::OpenPermissions));
    app.state.file = Some(entry("a.zip", FileKind::File));
    app.refresh();
    assert!(app.frame(&f, &i).hits.iter().any(|h| h.action == Action::OpenPermissions));
}
fn toast(app: &App) -> Option<&str> {
    app.toast.as_ref().map(|t| t.0.as_str())
}
#[test]
fn a_text_file_opens_to_edit_and_saves_what_was_typed() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Files));
    app.set_files("/data/adb".into(), Ok(vec![entry("run.sh", FileKind::File)]), LISTED);
    assert_eq!(
        tap(&mut app, &f, &i, Action::OpenEntry(0)),
        vec![Effect::OpenText("/data/adb/run.sh".into())]
    );
    app.text_opened(&f, "/data/adb/run.sh".into(), Ok(Some(b"echo hi\n".to_vec())), 0);
    assert_eq!(app.screen(), Screen::Editor);
    let frame = app.frame(&f, &i);
    assert_eq!(frame.button, None, "nothing waits to be saved");
    assert!(frame.caret.is_some());
    let keys: Vec<Action> = frame
        .hits
        .iter()
        .map(|h| h.action)
        .filter(|a| matches!(a, Action::TermKey(_)))
        .collect();
    assert_eq!(keys, editor::EDITOR_KEYS.map(Action::TermKey));
    tap(&mut app, &f, &i, Action::Type(Cap::Shift));
    tap(&mut app, &f, &i, Action::Type(Cap::Char('x')));
    assert_eq!(app.state.layer, Layer::Letters, "a capital lasts one letter");
    tap(&mut app, &f, &i, Action::Type(Cap::Space));
    tap(&mut app, &f, &i, Action::TermKey(TermKey::Tab));
    tap(&mut app, &f, &i, Action::Type(Cap::Enter));
    assert_eq!(app.frame(&f, &i).button, Some((widgets::Glyph::Done, Action::SaveText)));
    let save = vec![Effect::SaveText {
        path: "/data/adb/run.sh".into(),
        text: "X \t\necho hi\n".into(),
    }];
    assert_eq!(tap_button(&mut app, &f, &i, 0), save);
    app.text_saved(
        Err("rename /data/adb/.run.sh.0.saving to /data/adb/run.sh: Read-only file system (os error 30)".into()),
        0,
    );
    assert!(toast(&app).is_some_and(|t| t.starts_with("保存失败：rename")), "{:?}", app.toast);
    assert_eq!(
        app.frame(&f, &i).button,
        Some((widgets::Glyph::Done, Action::SaveText)),
        "still to save"
    );
    assert_eq!(tap_button(&mut app, &f, &i, 0), save);
    app.text_saved(Ok(()), 0);
    assert_eq!(toast(&app), Some("已保存"));
    assert_eq!(app.frame(&f, &i).button, None);
    tap_back(&mut app, &f, &i, 0);
    assert_eq!(app.screen(), Screen::Files, "a file saved leaves at once");
    assert_eq!(app.state.editor, None);
}
#[test]
fn leaving_the_editor_with_changes_unsaved_takes_going_back_again() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    app.set_files("/tmp".into(), Ok(vec![entry("a.txt", FileKind::File)]), LISTED);
    app.text_opened(&f, "/tmp/a.txt".into(), Ok(Some(b"a".to_vec())), 0);
    tap(&mut app, &f, &i, Action::Type(Cap::Char('b')));
    tap_back(&mut app, &f, &i, 1000);
    assert_eq!(app.screen(), Screen::Editor, "the first back only warns");
    assert_eq!(toast(&app), Some("修改尚未保存，再次返回将放弃修改"));
    tap_back(&mut app, &f, &i, 1000 + TOAST_MS);
    assert_eq!(app.screen(), Screen::Editor, "once the warning is gone, back warns again");
    tap_back(&mut app, &f, &i, 1000 + TOAST_MS + 500);
    assert_eq!(app.screen(), Screen::Files, "back again while it shows leaves");
    assert_eq!(app.state.editor, None, "without the changes");
}
#[test]
fn files_the_editor_does_not_take_are_turned_down_saying_why() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    let refused = [
        (Ok(None), "暂不支持编辑超过 2 MB 的文件"),
        (Ok(Some(vec![0xff, 0xfe, b'a'])), "仅支持编辑 UTF-8 编码的文本文件"),
        (Ok(Some(b"\x7fELF\x02\x01\x01\0".to_vec())), "仅支持编辑 UTF-8 编码的文本文件"),
    ];
    for (read, said) in refused {
        app.toast = None;
        app.text_opened(&f, "/sdcard/a.bin".into(), read, 0);
        assert_eq!(app.screen(), Screen::Files);
        assert_eq!(toast(&app), Some(said));
    }
    app.text_opened(
        &f,
        "/sdcard/a.bin".into(),
        Err("open /sdcard/a.bin: Permission denied (os error 13)".into()),
        0,
    );
    assert_eq!(toast(&app), Some("读取失败：open /sdcard/a.bin: Permission denied (os error 13)"));
    assert_eq!(app.state.editor, None);
}
#[test]
fn the_file_page_edits_a_file_and_nothing_else() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    assert_eq!(
        tap(&mut app, &f, &i, Action::EditEntry),
        vec![Effect::OpenText("/sdcard/a.zip".into())]
    );
    for kind in [
        folder(2),
        FileKind::Link {
            target: "a.zip".into(),
            folder: false,
        },
        FileKind::Special,
    ] {
        app.state.file = Some(entry("other", kind));
        app.refresh();
        assert!(
            !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::EditEntry),
            "{:?}",
            app.state.file
        );
    }
}
#[test]
fn the_file_page_shows_a_file_as_saved() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    app.text_opened(&f, "/sdcard/a.zip".into(), Ok(Some(b"x".to_vec())), 0);
    let saved = FileRow {
        size: 9,
        modified: LISTED,
        ..entry("a.zip", FileKind::File)
    };
    app.set_files("/sdcard".into(), Ok(vec![entry("DCIM", folder(2)), saved.clone()]), LISTED);
    assert_eq!(app.state.file, Some(saved));
}
#[test]
fn a_package_or_an_image_opened_in_the_file_manager_installs_and_another_is_chosen_there() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    assert_eq!(
        tap(&mut app, &f, &i, Action::OpenEntry(1)),
        vec![Effect::InstallPath("/sdcard/a.zip".into())]
    );
    assert_eq!(
        (app.screen(), app.state.source),
        (Screen::Installing, Source::Opened { drive: false })
    );
    assert_eq!(app.state.package_name.as_deref(), Some("a.zip"), "the page names the file");
    app.install(Event::Finished(Outcome::Refused(Refusal::Unknown)), 0);
    tap_at(&mut app, &f, &i, Action::Back, 1000);
    assert_eq!(
        (app.screen(), &app.state.install),
        (Screen::Files, &Install::Idle),
        "another file is chosen where this one was"
    );
    let drive = format!("{USB_DRIVE}/ROMs");
    app.set_files(drive.clone(), Ok(vec![entry("boot.img", FileKind::File)]), LISTED);
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::OpenEntry(0), 2000),
        vec![Effect::InstallPath(format!("{drive}/boot.img"))]
    );
    assert_eq!(
        app.state.source,
        Source::Opened { drive: true },
        "the page warns about pulling the drive out"
    );
}
#[test]
fn a_file_opened_as_an_install_ends_waits_for_it() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    tap(&mut app, &f, &i, Action::OpenEntry(1));
    app.install(
        Event::Image {
            kind: ImageKind::Dtbo,
            size: 1 << 20,
            partitions: vec!["dtbo"],
        },
        0,
    );
    assert_eq!(app.key(Key::Back, 1000), vec![Effect::Cancel]);
    assert_eq!(app.screen(), Screen::Files);
    assert!(tap_at(&mut app, &f, &i, Action::OpenEntry(1), 2000).is_empty());
    assert_eq!(toast(&app), Some("上一次安装尚未结束，请稍后再试"));
}
#[test]
fn the_file_page_installs_what_the_installer_takes_and_nothing_else() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    assert_eq!(
        tap(&mut app, &f, &i, Action::InstallEntry),
        vec![Effect::InstallPath("/sdcard/a.zip".into())]
    );
    assert_eq!(app.screen(), Screen::Installing);
    let link = FileKind::Link {
        target: "/sdcard/a.zip".into(),
        folder: false,
    };
    let taken = [entry("boot.IMG", FileKind::File), entry("b.zip", link)];
    let left = [
        entry("a.txt", FileKind::File),
        entry("img", FileKind::File),
        entry("c.zip", folder(1)),
        entry("d.zip", FileKind::Special),
    ];
    for (e, offered) in taken.into_iter().map(|e| (e, true)).chain(left.into_iter().map(|e| (e, false))) {
        app.state.file = Some(e);
        app.state.install = Install::Idle;
        app.stack = vec![(Screen::FileInfo, 0.0, Focus::Unset)];
        app.refresh();
        let has = app.frame(&f, &i).hits.iter().any(|h| h.action == Action::InstallEntry);
        assert_eq!(has, offered, "{:?}", app.state.file);
    }
}
#[test]
fn relabeling_starts_once_the_data_partition_is_open_and_its_page_follows_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Relabel));
    assert_eq!(app.screen(), Screen::Relabel);
    assert!(swipe(&mut app, &f, &i, Confirm::Relabel).is_empty());
    assert_eq!(toast(&app), Some("请先在「解密」中打开 data 分区"));
    app.state.data = Data::Locked {
        user: 0,
        keys: Keys::Digits,
        checking: false,
    };
    app.refresh();
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::Relabel),
        vec![Effect::Relabel],
        "open, though a user has yet to unlock"
    );
    app.state.data = Data::Open;
    app.refresh();
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Relabel), vec![Effect::Relabel]);
    app.relabel_started();
    assert!(!app.frame(&f, &i).back, "no going back while it works");
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::Relabel);
    assert_eq!(toast(&app), Some("请等待完成或停止后再返回"));
    app.relabel_progress(0, 0, 0);
    app.relabel_progress(10, 400, 2);
    assert_eq!(tap(&mut app, &f, &i, Action::StopRelabel), vec![Effect::StopRelabel]);
    assert!(
        !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::StopRelabel),
        "stopping, it is not asked again"
    );
    app.relabel_progress(12, 400, 3);
    app.relabel_ended(JobEnd::Stopped, 0);
    let job = app.state.relabel.clone().expect("the page shows how it ended");
    assert_eq!(
        (job.checked, job.total, job.changed, job.outcome),
        (12, 400, 3, Some(JobEnd::Stopped))
    );
    assert!(app.frame(&f, &i).back);
    tap_at(&mut app, &f, &i, Action::Back, 2000);
    assert_eq!(app.screen(), Screen::Advanced);
    assert_eq!(app.state.relabel, None);
}
/// The filesystem page's partitions on a phone with a cache partition:
/// the data partition closed, and a cache partition its table lets hold
/// either filesystem.
fn fs_rows() -> Vec<FsRow> {
    let row = |name: &str, filesystem: Option<&str>, others: &[&str]| FsRow {
        name: name.into(),
        mount_point: format!("/{name}"),
        filesystem: filesystem.map(str::to_string),
        size: 64 << 20,
        others: others.iter().map(|o| o.to_string()).collect(),
    };
    vec![
        row("data", None, &[]),
        row("metadata", Some("f2fs"), &[]),
        row("cache", Some("ext4"), &["f2fs"]),
    ]
}
#[test]
fn the_filesystem_page_does_to_a_partition_what_is_chosen_and_reads_them_again_after() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Filesystems)),
        vec![Effect::ReadFilesystems]
    );
    app.set_filesystems(Ok(fs_rows()));
    tap(&mut app, &f, &i, Action::OpenFilesystem(2));
    assert_eq!(app.screen(), Screen::Filesystem);
    assert_eq!(
        app.state.fs_action,
        FsAction::Repair,
        "a repair, which changes nothing the partition holds, to begin with"
    );
    tap(&mut app, &f, &i, Action::ChooseFs(FsChoice::Remake(0)));
    assert_eq!(app.state.fs_action, FsAction::Remake("f2fs".into()));
    let remake = Effect::Filesystem {
        name: "cache".into(),
        action: FsAction::Remake("f2fs".into()),
    };
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Filesystem), vec![remake]);
    app.part_work_started(PartKind::Remake("f2fs".into()), false);
    assert_eq!(app.screen(), Screen::PartWork);
    app.part_listed(vec!["cache".into()]);
    app.part_at(0);
    assert!(!app.frame(&f, &i).back, "no going back while it works");
    app.key(Key::Back, 0);
    assert_eq!((app.screen(), toast(&app)), (Screen::PartWork, Some("请等待完成后再返回")));
    app.part_ended(0, ItemEnd::Made, "");
    app.part_work_ended(Ok(()), 0);
    assert!(app.frame(&f, &i).back);
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::Back, 2000),
        vec![Effect::ReadFilesystems],
        "the partition may hold another now"
    );
    assert_eq!(app.screen(), Screen::Filesystem);
    assert_eq!(app.state.part_work, None);
    assert_eq!(app.state.fs_action, FsAction::Repair, "what to do to it is chosen afresh");
}
#[test]
fn work_on_the_data_partition_waits_for_it_to_open_and_leaves_it_closed() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Filesystems, W, H, PPD);
    app.set_filesystems(Ok(fs_rows()));
    tap(&mut app, &f, &i, Action::OpenFilesystem(0));
    tap(&mut app, &f, &i, Action::ChooseFs(FsChoice::Repair));
    app.state.data = Data::Opening;
    app.refresh();
    assert!(swipe(&mut app, &f, &i, Confirm::Filesystem).is_empty());
    assert_eq!(toast(&app), Some("正在打开 data 分区，请稍后再试"));
    app.state.data = Data::Open;
    app.refresh();
    let repair = Effect::Filesystem {
        name: "data".into(),
        action: FsAction::Repair,
    };
    assert_eq!(swipe(&mut app, &f, &i, Confirm::Filesystem), vec![repair]);
    app.part_work_started(PartKind::Repair, false);
    app.part_listed(vec!["data".into()]);
    app.part_at(0);
    app.part_ended(
        0,
        ItemEnd::Fixed,
        "[FSCK] valid_block_count matching with CP   [Fail]\n\nDone: 3.2 secs\n",
    );
    app.part_work_ended(Ok(()), 0);
    let work = app.state.part_work.clone().expect("the page shows how it ended");
    assert_eq!(work.items, [("data".to_string(), ItemState::Ended(ItemEnd::Fixed))]);
    assert_eq!(
        work.output,
        ["[FSCK] valid_block_count matching with CP   [Fail]", "Done: 3.2 secs"],
        "blank lines left out"
    );
    assert_eq!(app.state.data, Data::Closed, "worked on, the partition is closed");
}
#[test]
fn partition_work_the_system_asked_for_is_the_only_page_and_offers_somewhere_to_go_only_when_it_fell_short() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.part_work_started(PartKind::Disable, true);
    assert_eq!(app.stack.len(), 1);
    assert_eq!(app.screen(), Screen::PartWork);
    app.part_listed(vec!["/dev/block/by-name/userdata".into(), "/dev/block/by-name/metadata".into()]);
    app.part_at(0);
    app.part_ended(0, ItemEnd::Erased, "");
    app.part_at(1);
    app.part_ended(1, ItemEnd::Erased, "");
    app.part_work_ended(Ok(()), 0);
    let reboot = |app: &mut App| app.frame(&f, &i).hits.iter().any(|h| h.action == Action::RebootTo(Target::System));
    assert!(!reboot(&mut app), "done, the phone leaves on its own");
    app.part_work_started(PartKind::Disable, true);
    app.part_work_ended(Err(PartStop::Refused(Refusal::Unverified)), 0);
    assert!(reboot(&mut app), "refused, the user chooses where to go");
    let frame = app.frame(&f, &i);
    assert!(!frame.back);
    assert!(
        frame.hits.iter().any(|h| h.action == Action::EnterRecovery),
        "the recovery, which ran without the slot's partitions, brings them up to go on"
    );
}
#[test]
fn an_update_the_system_asked_for_waits_for_the_files_that_hold_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.start_asked(false, "/sdcard/update.zip".into());
    assert_eq!(
        (app.screen(), app.state.source),
        (Screen::Installing, Source::Asked { security: false })
    );
    app.asked_opening();
    app.key(Key::Back, 0);
    assert_eq!(
        (app.screen(), toast(&app)),
        (Screen::Installing, Some("正在打开 data 分区，请稍后再试"))
    );
    app.asked_locked(0, 0);
    assert_eq!(app.state.install, Install::Locked(0));
    assert!(app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Open(Screen::Decrypt)));
    assert_eq!(app.key(Key::Back, 5000), vec![Effect::Cancel], "leaving gives the update up");
    app.install(Event::Finished(Outcome::Cancelled), 5000);
    assert_eq!(app.state.install, Install::Idle);
}
#[test]
fn rescue_stays_for_one_command_after_another_until_its_page_is_left() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.start_rescue();
    assert_eq!((app.screen(), app.state.source), (Screen::Installing, Source::Rescue));
    app.install(Event::Waiting, 0);
    app.install(Event::Checking { size: 4096 }, 0);
    app.install(
        Event::Done(Outcome::Installed {
            slot: Slot::B,
            wipe: false,
        }),
        0,
    );
    assert!(matches!(app.state.install, Install::Finished { .. }));
    app.rescue_wiping();
    app.key(Key::Back, 5000);
    assert_eq!((app.screen(), toast(&app)), (Screen::Installing, Some("正在清除，请等待清除完成")));
    app.rescue_wiped(Ok(()), 6000);
    assert_eq!(
        (app.state.install.clone(), app.state.data.clone()),
        (Install::Wiped(Ok(())), Data::Erased)
    );
    app.frame(&f, &i);
    assert_eq!(app.key(Key::Back, 12000), vec![Effect::Cancel], "leaving ends rescue");
    assert_eq!(app.state.install, Install::Ending);
}
fn network(name: &str, signal: i32, security: Security) -> Network {
    let bands = skyward_net::wlan::Bands::of(5805);
    Network {
        ssid: name.as_bytes().to_vec(),
        name: name.into(),
        signal,
        frequency: 5805,
        bands,
        security,
        remembered: false,
    }
}
fn joined(name: &str, stage: Joining) -> Wlan {
    let n = network(name, -34, Security::Psk);
    Wlan {
        power: Power::On,
        networks: vec![n.clone()],
        joined: Some(skyward_net::wlan::Joined {
            ssid: n.ssid,
            name: n.name,
            security: n.security,
            stage,
        }),
    }
}
fn connected(name: &str) -> Wlan {
    use std::net::Ipv4Addr;
    joined(
        name,
        Joining::Connected(skyward_net::wlan::Link {
            address: Ipv4Addr::new(192, 168, 1, 23),
            mask: Ipv4Addr::new(255, 255, 255, 0),
            router: Some(Ipv4Addr::new(192, 168, 1, 1)),
            dns: vec![Ipv4Addr::new(192, 168, 1, 1)],
            signal: -40,
            frequency: 5805,
            speed: Some(866),
            generation: Some(5),
            mac: "02:00:00:00:00:04".into(),
            reach: Some(Reach::Online),
        }),
    )
}
#[test]
fn wlan_lists_only_nearby_networks_and_keeps_remembering_independent_of_connection() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Wlan, W, H, PPD);
    app.set_wlan(Wlan {
        power: Power::On,
        networks: vec![network("Nearby", -40, Security::Psk)],
        joined: None,
    });
    let actions = |app: &mut App| app.frame(&f, &i).hits.iter().map(|h| h.action).collect::<Vec<_>>();
    let disconnected = actions(&mut app);
    assert!(disconnected.contains(&Action::JoinNetwork(0)));
    assert!(disconnected.contains(&Action::AddNetwork));
    assert!(disconnected.contains(&Action::Toggle(Switch::RememberWlan)));
    app.set_wlan(connected("Nearby"));
    let connected = actions(&mut app);
    assert!(connected.contains(&Action::OpenJoined));
    assert!(connected.contains(&Action::Toggle(Switch::RememberWlan)));
    assert!(!connected.contains(&Action::Confirm(Confirm::ForgetNetwork)));
}

#[test]
fn selecting_a_remembered_network_reuses_its_credential_and_a_refusal_asks_again() {
    let mut app = App::new(Screen::Wlan, W, H, PPD);
    let mut home = network("Home", -40, Security::Psk);
    home.remembered = true;
    app.set_wlan(Wlan {
        power: Power::On,
        networks: vec![home],
        joined: None,
    });
    assert_eq!(
        app.act(Action::JoinNetwork(0), 100),
        [Effect::JoinRememberedWlan {
            ssid: b"Home".to_vec(),
            name: "Home".into(),
            security: Security::Psk,
        }]
    );
    assert_eq!(app.screen(), Screen::Wlan);
    app.set_wlan(joined("Home", Joining::WrongPassword));
    app.act(Action::JoinNetwork(0), 200);
    assert_eq!(app.screen(), Screen::WlanPassword);
    assert!(app.state.password.is_empty());
}

#[test]
fn forgetting_a_network_waits_for_the_result_and_preserves_details_on_failure() {
    let (f, i) = (fonts(), info());
    for connected_now in [false, true] {
        let mut app = App::new(Screen::Wlan, W, H, PPD);
        let mut wlan = if connected_now {
            connected("Home")
        } else {
            joined("Home", Joining::Joining)
        };
        wlan.networks[0].remembered = true;
        if !connected_now {
            wlan.joined = None;
        }
        app.set_wlan(wlan);
        app.act(Action::NetworkDetails(0), 1000);
        assert!(app.frame(&f, &i).hits.iter().any(|hit| hit.action == Action::ForgetNetwork));
        assert!(
            app.confirmed(Confirm::ForgetNetwork, 1010).is_empty(),
            "no confirmation was requested"
        );
        assert!(app.act(Action::ForgetNetwork, 1020).is_empty());
        assert!(
            app.frame(&f, &i)
                .hits
                .iter()
                .any(|hit| hit.action == Action::Confirm(Confirm::ForgetNetwork))
        );
        assert_eq!(app.confirmed(Confirm::ForgetNetwork, 1030), [Effect::ForgetWlan("Home".into())]);
        assert!(app.state.details.is_some(), "the asynchronous result has not arrived");
        app.frame(&f, &i);
        app.wlan_forgotten(Err("read-only metadata".into()), 1040);
        assert_eq!(app.screen(), Screen::WlanNetwork);
        assert!(app.state.details.is_some());
        assert!(!app.state.asking_delete);
        app.frame(&f, &i);
        app.act(Action::ForgetNetwork, 1050);
        assert_eq!(app.confirmed(Confirm::ForgetNetwork, 1060), [Effect::ForgetWlan("Home".into())]);
        app.wlan_forgotten(Ok("Another network".into()), 1070);
        assert_eq!(
            app.screen(),
            Screen::WlanNetwork,
            "an old completion cannot close different details"
        );
        app.wlan_forgotten(Ok("Home".into()), 1080);
        app.tick(3000);
        assert_eq!(app.screen(), Screen::Wlan);
        assert!(app.state.details.is_none());
        app.frame(&f, &i);
    }
}

#[test]
fn leaving_network_details_cancels_only_the_confirmation_not_the_saved_credential() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Wlan, W, H, PPD);
    let mut wlan = connected("Home");
    wlan.networks[0].remembered = true;
    app.set_wlan(wlan);
    app.act(Action::NetworkDetails(0), 1000);
    app.act(Action::ForgetNetwork, 1010);
    assert!(app.back(1020).is_empty());
    app.tick(3000);
    assert_eq!(app.screen(), Screen::Wlan);
    assert!(!app.state.asking_delete);
    app.act(Action::NetworkDetails(0), 3100);
    assert!(
        !app.frame(&f, &i)
            .hits
            .iter()
            .any(|hit| hit.action == Action::Confirm(Confirm::ForgetNetwork))
    );
    assert_eq!(app.act(Action::LeaveNetwork, 3110), [Effect::LeaveWlan]);
    assert!(app.state.wlan.as_ref().unwrap().networks[0].remembered);
}

#[test]
fn the_wlan_comes_up_with_the_recovery_rather_than_by_a_switch() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    assert!(
        !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Open(Screen::Wlan)),
        "a phone whose WLAN is not brought up has no WLAN"
    );
    app.start_wlan();
    assert_eq!(fmt::wlan_value(Lang::Hans, app.state.wlan.as_ref().unwrap()), "正在开启");
    assert!(
        app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Open(Screen::Wlan)),
        "the WLAN is there from the first frame"
    );
    app.state.wlan = Some(Wlan {
        power: Power::Starting,
        ..Wlan::default()
    });
    assert!(
        app.act(Action::Open(Screen::Wlan), 0).is_empty(),
        "a WLAN still coming up is not scanned"
    );
    assert_eq!(app.screen(), Screen::Wlan);
    app.set_wlan(Wlan {
        power: Power::On,
        networks: Vec::new(),
        joined: None,
    });
    assert_eq!(fmt::wlan_value(Lang::Hans, app.state.wlan.as_ref().unwrap()), "未连接");
    assert_eq!(
        app.arrive(),
        vec![Effect::ScanWlan],
        "the page scans by itself once the radio is up"
    );
}
#[test]
fn a_wlan_that_does_not_come_up_shows_why_and_leaves_the_pages_alone() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.start_wlan();
    app.set_wlan(Wlan {
        power: Power::Failed("no wlan0 in 20 seconds".into()),
        ..Wlan::default()
    });
    assert_eq!(fmt::wlan_value(Lang::Hans, app.state.wlan.as_ref().unwrap()), "已关闭");
    app.act(Action::Open(Screen::Wlan), 0);
    assert_eq!(app.screen(), Screen::Wlan);
    let t = app.theme();
    let drawn = pages::render(
        Screen::Wlan,
        &Ctx {
            t: &t,
            f: &f,
            info: &i,
            state: &app.state,
            term: None,
            band: None,
        },
    );
    assert!(
        drawn.canvas.data.iter().any(|&b| b != 0),
        "the WLAN page draws with no radio behind it"
    );
    assert_eq!(app.key(Key::Back, 0), Vec::<Effect>::new(), "leaving it costs nothing");
    assert_eq!(app.screen(), Screen::Home, "the other pages are reachable");
}
#[test]
fn a_wlan_switch_still_turns_the_radio_off_and_on_after_it_came_up_by_itself() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.start_wlan();
    app.set_wlan(Wlan {
        power: Power::On,
        networks: Vec::new(),
        joined: None,
    });
    app.act(Action::Open(Screen::Wlan), 0);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Toggle(Switch::Wlan)),
        vec![Effect::Wlan(false)],
        "the switch still turns it off"
    );
    assert_eq!(app.state.wlan.as_ref().unwrap().power, Power::Off, "and says so at once");
    assert_eq!(
        tap(&mut app, &f, &i, Action::Toggle(Switch::Wlan)),
        vec![Effect::Wlan(true)],
        "and on again"
    );
    assert_eq!(app.state.wlan.as_ref().unwrap().power, Power::Starting);
}
#[test]
fn the_wlan_page_joins_a_network_with_its_password_and_leaves_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let entry = |app: &mut App| app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Open(Screen::Wlan));
    assert!(!entry(&mut app), "a phone whose WLAN is not brought up has no WLAN");
    app.state.wlan = Some(Wlan::default());
    app.refresh();
    assert!(entry(&mut app));
    assert!(
        app.act(Action::Open(Screen::Wlan), 0).is_empty(),
        "a WLAN that is off is not scanned"
    );
    assert_eq!(tap(&mut app, &f, &i, Action::Toggle(Switch::Wlan)), vec![Effect::Wlan(true)]);
    assert_eq!(app.state.wlan.as_ref().unwrap().power, Power::Starting, "the switch moves at once");
    let nearby = vec![
        network("Skyward", -34, Security::Psk),
        network("Skyward-Open", -72, Security::Open),
        network("Skyward-WEP", -60, Security::Wep),
    ];
    app.set_wlan(Wlan {
        power: Power::On,
        networks: nearby,
        joined: None,
    });
    tap(&mut app, &f, &i, Action::JoinNetwork(2));
    assert_eq!(toast(&app), Some("暂不支持 WEP 加密的网络"));
    let cafe = Effect::JoinWlan {
        ssid: b"Skyward-Open".to_vec(),
        name: "Skyward-Open".into(),
        security: Security::Open,
        password: Secret::default(),
    };
    assert_eq!(
        tap(&mut app, &f, &i, Action::JoinNetwork(1)),
        vec![cafe],
        "an open network is joined at once"
    );
    tap(&mut app, &f, &i, Action::JoinNetwork(0));
    assert_eq!(app.screen(), Screen::WlanPassword);
    let type_in = |app: &mut App, text: &str| {
        for ch in text.chars() {
            tap(app, &f, &i, Action::Type(Cap::Char(ch)));
        }
    };
    type_in(&mut app, "secret");
    assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
    assert_eq!(toast(&app), Some("请输入 8 到 63 位密码"));
    tap(&mut app, &f, &i, Action::Type(Cap::Show(Layer::Digits)));
    type_in(&mut app, "12");
    let fx = tap(&mut app, &f, &i, Action::Type(Cap::Enter));
    let join = Effect::JoinWlan {
        ssid: b"Skyward".to_vec(),
        name: "Skyward".into(),
        security: Security::Psk,
        password: Secret("secret12".into()),
    };
    assert_eq!(
        format!("{join:?}"),
        r#"JoinWlan { ssid: [83, 107, 121, 119, 97, 114, 100], name: "Skyward", security: Psk, password: Secret(8 characters) }"#,
        "what is printed leaves the password out"
    );
    assert_eq!(fx, vec![join]);
    assert_eq!(app.screen(), Screen::Wlan, "the WLAN page shows how joining goes");
    assert_eq!(
        (app.state.password.as_str(), app.state.joining.as_ref()),
        ("", None),
        "the password is not kept"
    );
    app.set_wlan(connected("Skyward"));
    assert_eq!(fmt::wlan_value(Lang::Hans, app.state.wlan.as_ref().unwrap()), "Skyward");
    tap(&mut app, &f, &i, Action::OpenJoined);
    assert_eq!(app.screen(), Screen::WlanNetwork);
    assert_eq!(tap(&mut app, &f, &i, Action::LeaveNetwork), vec![Effect::LeaveWlan]);
    assert_eq!(app.screen(), Screen::Wlan);
    assert_eq!(tap(&mut app, &f, &i, Action::Toggle(Switch::Wlan)), vec![Effect::Wlan(false)]);
    assert_eq!(app.state.wlan, Some(Wlan::default()));
}
#[test]
fn a_refused_password_is_asked_for_again_from_the_networks_row() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.wlan = Some(joined("Skyward", Joining::WrongPassword));
    app.act(Action::Open(Screen::Wlan), 0);
    tap(&mut app, &f, &i, Action::OpenJoined);
    assert_eq!(app.screen(), Screen::WlanPassword);
    assert_eq!(app.state.joining.as_ref().map(|n| n.name.as_str()), Some("Skyward"));
    app.key(Key::Back, 0);
    app.set_wlan(joined("Skyward", Joining::Joining));
    tap(&mut app, &f, &i, Action::OpenJoined);
    assert_eq!(app.screen(), Screen::WlanNetwork, "a network being joined shows how that goes");
    assert_eq!(app.state.details.as_ref().map(|n| n.name.as_str()), Some("Skyward"));
    app.key(Key::Back, 0);
    assert_eq!(app.state.details, None);
}
#[test]
fn the_status_bar_shows_the_network_joined_and_how_fast_data_goes_without_the_page_drawn_again() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.set_wlan(Wlan {
        power: Power::On,
        networks: Vec::new(),
        joined: None,
    });
    assert!(app.wlan_mark().is_none(), "no network joined");
    app.set_wlan(joined("Skyward", Joining::Addressing));
    assert!(app.wlan_mark().is_none(), "not while it has no address");
    app.set_wlan(connected("Skyward"));
    let mark = app.wlan_mark().unwrap();
    assert_eq!(
        (mark.level, mark.speed.as_str(), mark.unit, mark.receiving, mark.sending),
        (3, "0.00", "KB/s", false, false)
    );
    app.frame(&f, &i);
    app.dirty = false;
    app.set_traffic(Traffic { rx: 42_000, tx: 0 });
    assert!(
        app.dirty && app.frame.is_some(),
        "the status bar is drawn again, and the page as it was"
    );
    let mark = app.wlan_mark().unwrap();
    assert_eq!((mark.speed.as_str(), mark.receiving, mark.sending), ("41.0", true, false));
}
#[test]
fn a_network_is_added_by_hand_with_its_name_protection_and_password() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.wlan = Some(Wlan {
        power: Power::On,
        networks: Vec::new(),
        joined: None,
    });
    app.act(Action::Open(Screen::Wlan), 0);
    tap(&mut app, &f, &i, Action::AddNetwork);
    assert_eq!(
        app.screen(),
        Screen::WlanAdd,
        "the list ends with adding one, found networks or none"
    );
    let type_in = |app: &mut App, text: &str| {
        for ch in text.chars() {
            tap(app, &f, &i, Action::Type(Cap::Char(ch)));
        }
    };
    assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
    assert_eq!(toast(&app), Some("请输入网络名称"));
    type_in(&mut app, "skyward");
    let frame = app.frame(&f, &i);
    assert!(
        !frame.hits.iter().any(|h| h.action == Action::TypeInto(Field::Password)),
        "an open network takes no password"
    );
    tap(&mut app, &f, &i, Action::ChooseSecurity);
    let frame = app.frame(&f, &i);
    assert!(
        frame
            .hits
            .iter()
            .all(|h| matches!(h.action, Action::CloseChoices | Action::PickSecurity(_)))
    );
    assert!(frame.caret.is_none(), "no cursor shows through the list");
    let below = (W as f32 / 2.0, H as f32 - 20.0);
    app.touch(Touch::Down(below.0, below.1), &f, 0);
    app.touch(Touch::Up, &f, 0);
    let a = app.state.adding.clone().unwrap();
    assert_eq!(
        (a.name.as_str(), a.security, a.choosing),
        ("skyward", Security::Open, false),
        "a touch off the list types nothing"
    );
    tap(&mut app, &f, &i, Action::ChooseSecurity);
    app.key(Key::Back, 0);
    assert_eq!(
        (app.screen(), app.state.adding.as_ref().unwrap().choosing),
        (Screen::WlanAdd, false),
        "back closes the list first"
    );
    tap(&mut app, &f, &i, Action::ChooseSecurity);
    tap(&mut app, &f, &i, Action::PickSecurity(Security::Psk));
    let a = app.state.adding.clone().unwrap();
    assert_eq!((a.security, a.choosing, a.field), (Security::Psk, false, Field::Name));
    tap(&mut app, &f, &i, Action::TypeInto(Field::Password));
    type_in(&mut app, "secret");
    tap(&mut app, &f, &i, Action::TypeInto(Field::Name));
    assert_eq!(
        (app.state.adding.as_ref().unwrap().name.as_str(), app.state.password.as_str()),
        ("skyward", "secret")
    );
    assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
    assert_eq!(toast(&app), Some("请输入 8 到 63 位密码"));
    assert_eq!(
        app.state.adding.as_ref().unwrap().field,
        Field::Password,
        "the keyboard types into the field turned down"
    );
    tap(&mut app, &f, &i, Action::Type(Cap::Show(Layer::Digits)));
    type_in(&mut app, "12");
    let fx = tap(&mut app, &f, &i, Action::Type(Cap::Enter));
    let join = Effect::JoinWlan {
        ssid: b"skyward".to_vec(),
        name: "skyward".into(),
        security: Security::Psk,
        password: Secret("secret12".into()),
    };
    assert_eq!(fx, vec![join]);
    assert_eq!(app.screen(), Screen::Wlan, "the WLAN page shows how joining goes");
    assert_eq!(
        (app.state.adding.as_ref(), app.state.password.as_str()),
        (None, ""),
        "nothing typed is kept"
    );
}
#[test]
fn a_network_added_by_hand_is_turned_down_for_a_name_too_long_and_joined_open_without_its_password() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.wlan = Some(Wlan {
        power: Power::On,
        networks: vec![network("Skyward-Open", -72, Security::Open)],
        joined: None,
    });
    app.act(Action::Open(Screen::Wlan), 0);
    tap(&mut app, &f, &i, Action::AddNetwork);
    tap(&mut app, &f, &i, Action::ChooseSecurity);
    tap(&mut app, &f, &i, Action::PickSecurity(Security::Sae));
    tap(&mut app, &f, &i, Action::TypeInto(Field::Password));
    tap(&mut app, &f, &i, Action::Type(Cap::Char('x')));
    tap(&mut app, &f, &i, Action::ChooseSecurity);
    tap(&mut app, &f, &i, Action::PickSecurity(Security::Open));
    assert_eq!(
        app.state.adding.as_ref().unwrap().field,
        Field::Name,
        "with the password field gone, the keyboard types the name"
    );
    for _ in 0..33 {
        tap(&mut app, &f, &i, Action::Type(Cap::Char('a')));
    }
    assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
    assert_eq!(toast(&app), Some("网络名称不能超过 32 个字符"));
    tap(&mut app, &f, &i, Action::Type(Cap::Backspace));
    let fx = tap(&mut app, &f, &i, Action::Type(Cap::Enter));
    let name = "a".repeat(32);
    assert_eq!(
        fx,
        vec![Effect::JoinWlan {
            ssid: name.clone().into_bytes(),
            name,
            security: Security::Open,
            password: Secret::default()
        }]
    );
}
#[test]
fn the_keys_open_a_list_of_choices_take_one_and_close_it_going_back() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.wlan = Some(Wlan {
        power: Power::On,
        networks: Vec::new(),
        joined: None,
    });
    app.act(Action::Open(Screen::Wlan), 0);
    app.act(Action::AddNetwork, 0);
    app.frame(&f, &i);
    light_on(&mut app, &f, &i, Stop::Hit(Action::ChooseSecurity));
    press(&mut app, Key::Power, 0);
    assert!(app.state.adding.as_ref().unwrap().choosing);
    let t = app.theme();
    let stops: Vec<Stop> = focus::spots(&t, app.frame(&f, &i)).iter().map(|s| s.stop).collect();
    let pick = |s| Stop::Hit(Action::PickSecurity(s));
    assert_eq!(
        stops,
        [Stop::Back, pick(Security::Open), pick(Security::Psk), pick(Security::Sae)],
        "only the list and back"
    );
    light_on(&mut app, &f, &i, pick(Security::Sae));
    press(&mut app, Key::Power, 0);
    let a = app.state.adding.clone().unwrap();
    assert_eq!((a.security, a.choosing), (Security::Sae, false));
    light_on(&mut app, &f, &i, Stop::Hit(Action::ChooseSecurity));
    press(&mut app, Key::Power, 0);
    press(&mut app, Key::Back, 0);
    assert_eq!(
        (app.screen(), app.state.adding.as_ref().unwrap().choosing),
        (Screen::WlanAdd, false)
    );
}
#[test]
fn a_choice_shows_held_down_while_touched() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.wlan = Some(Wlan {
        power: Power::On,
        networks: Vec::new(),
        joined: None,
    });
    app.act(Action::Open(Screen::Wlan), 0);
    tap(&mut app, &f, &i, Action::AddNetwork);
    tap(&mut app, &f, &i, Action::ChooseSecurity);
    let wpa3 = Action::PickSecurity(Security::Sae);
    let rect = app.frame(&f, &i).hits.iter().find(|h| h.action == wpa3).unwrap().rect;
    let at = |app: &mut App| {
        let c = &app.frame(&f, &i).canvas;
        let (x, y) = ((rect.x + rect.w) as u32 - 20, (rect.y + rect.h / 2.0) as u32);
        let k = ((y - c.held().start) * c.w + x) as usize * 4;
        Rgba::rgb(c.data[k], c.data[k + 1], c.data[k + 2])
    };
    let t = app.theme();
    assert_eq!(at(&mut app), t.pal.popup);
    let (x, y) = spot(&mut app, &f, &i, wpa3);
    app.touch(Touch::Down(x, y), &f, 0);
    assert_eq!(app.state.held, Some(wpa3));
    assert_eq!(at(&mut app), over(t.pal.row_press, t.pal.popup));
    app.touch(Touch::Up, &f, 0);
    assert_eq!(app.state.held, None);
    assert_eq!(app.state.adding.as_ref().unwrap().security, Security::Sae);
}
#[test]
fn a_networks_round_button_opens_its_details_and_its_row_joins_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let nearby = vec![network("Skyward", -34, Security::Psk), network("Skyward-Open", -72, Security::Open)];
    app.state.wlan = Some(Wlan {
        power: Power::On,
        networks: nearby,
        joined: None,
    });
    app.act(Action::Open(Screen::Wlan), 0);
    assert_eq!(tap(&mut app, &f, &i, Action::Rescan), vec![Effect::ScanWlan]);
    tap(&mut app, &f, &i, Action::NetworkDetails(1));
    assert_eq!(app.screen(), Screen::WlanNetwork);
    assert_eq!(app.state.details.as_ref().map(|n| n.name.as_str()), Some("Skyward-Open"));
    assert!(
        !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::LeaveNetwork),
        "a network not chosen has nothing to forget"
    );
    app.key(Key::Back, 0);
    let fr = app.frame(&f, &i);
    let row = fr.hits.iter().find(|h| h.action == Action::JoinNetwork(1)).expect("the row").rect;
    let round = fr
        .hits
        .iter()
        .find(|h| h.action == Action::NetworkDetails(1))
        .expect("its round button")
        .rect;
    assert!(row.contains(round.x + round.w / 2.0, round.y + round.h / 2.0));
    assert_eq!(
        fr.hit(round.x + round.w / 2.0, round.y + round.h / 2.0).map(|h| h.action),
        Some(Action::NetworkDetails(1))
    );
    assert_eq!(
        fr.hit(row.x + row.w / 3.0, row.y + row.h / 2.0).map(|h| h.action),
        Some(Action::JoinNetwork(1))
    );
}
fn newest() -> Release {
    Release {
        version: "OS3.0.310.0.WMACNXM".into(),
        name: "ishtar-ota_full-OS3.0.310.0.WMACNXM-user-16.0-0123456789.zip".into(),
        urls: vec!["https://ultimateota.d.miui.com/OS3.0.310.0.WMACNXM/ishtar-ota_full.zip".into()],
        md5: "0123456789abcdef0123456789abcdef".into(),
        size: "7.1G".into(),
        length: Some(7_572_375_802),
    }
}
#[test]
fn the_update_page_asks_for_the_newest_system_and_downloads_its_full_package_to_install() {
    let f = fonts();
    let i = DeviceInfo {
        os_version: Some("OS3.0.308.0.WMACNXM".into()),
        ..info()
    };
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    app.state.update_server = true;
    app.state.wlan = Some(Wlan::default());
    app.refresh();
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Update)),
        vec![Effect::ReadUpdate],
        "nothing is asked without a network"
    );
    assert!(tap(&mut app, &f, &i, Action::CheckUpdate).is_empty());
    assert_eq!(app.screen(), Screen::Wlan, "asking takes a network, which the WLAN page joins");
    app.key(Key::Back, 0);
    app.set_wlan(connected("Skyward"));
    assert_eq!(tap(&mut app, &f, &i, Action::CheckUpdate), vec![Effect::CheckUpdate]);
    assert_eq!(app.state.check, Check::Asking);
    app.update_checked(Ok(Some(newest())), Have::Nothing);
    app.state.data = Data::Locked {
        user: 0,
        keys: Keys::Digits,
        checking: false,
    };
    app.refresh();
    assert!(tap(&mut app, &f, &i, Action::Download).is_empty());
    assert_eq!(
        app.screen(),
        Screen::Decrypt,
        "the package goes to internal storage, which opens with the owner's files"
    );
    app.state.data = Data::Open;
    app.act(Action::Skip, 0);
    assert_eq!(app.screen(), Screen::Update);
    assert_eq!(tap(&mut app, &f, &i, Action::Download), vec![Effect::Download(newest())]);
    app.download_progress(
        Progress::Checking {
            done: 0,
            total: 7_572_375_802,
        },
        0,
    );
    app.download_progress(
        Progress::Downloading {
            done: 1_000_000_000,
            total: 7_572_375_802,
        },
        12_000_000,
    );
    assert!(app.state.downloading());
    assert_eq!(tap(&mut app, &f, &i, Action::PauseDownload), vec![Effect::PauseDownload]);
    assert!(
        !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::PauseDownload),
        "pausing, it is not asked again"
    );
    app.download_ended(DownloadEnd::Stopped, Have::Part(1_000_000_000), 0);
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::Download, 2000),
        vec![Effect::Download(newest())],
        "a download paused carries on"
    );
    let damaged = skyward_net::download::Failure::Damaged;
    app.download_ended(DownloadEnd::Failed(damaged.clone()), Have::Nothing, 3000);
    assert_eq!(
        fmt::download_failure(Lang::Hans, &damaged),
        "完整更新包校验失败，已删除，请重新下载。"
    );
    assert_eq!(tap_at(&mut app, &f, &i, Action::Download, 5000), vec![Effect::Download(newest())]);
    app.download_ended(DownloadEnd::Downloaded, Have::Whole(7_572_375_802), 6000);
    let fx = tap_at(&mut app, &f, &i, Action::InstallDownloaded, 8000);
    assert_eq!(
        fx,
        vec![Effect::InstallFile(Place::Storage, format!("downloaded_rom/{}", newest().name))]
    );
    assert_eq!(app.screen(), Screen::Installing);
}
#[test]
fn the_update_page_asks_the_server_as_it_opens_once_a_network_is_joined() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    app.state.wlan = Some(connected("Skyward"));
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Update)),
        vec![Effect::ReadUpdate],
        "a phone whose server is not known asks nothing"
    );
    app.key(Key::Back, 0);
    app.state.update_server = true;
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Update)),
        vec![Effect::ReadUpdate, Effect::CheckUpdate]
    );
    app.update_checked(Err(Unanswered::Failed("update.miui.com did not answer".into())), Have::Nothing);
    app.key(Key::Back, 0);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::Update)),
        vec![Effect::ReadUpdate, Effect::CheckUpdate],
        "a failed check is tried again"
    );
}
fn skyward_newest(newer: bool) -> Newest {
    Newest {
        version: "0.1.4".into(),
        newer,
        build: Some(Build {
            name: "skyward-v0.1.4-ishtar-OS3.0.308.0.WMACNXM.zip".into(),
            url: "https://api.github.com/repos/kirklin/skyward/releases/assets/719751598".into(),
            link: "https://github.com/kirklin/skyward/releases/download/v0.1.4/skyward-v0.1.4-ishtar-OS3.0.308.0.WMACNXM.zip".into(),
            size: 42_739_608,
            sha256: "62b66f7d13a0eb6c9af3359b5ec6f9c223fd9a86b1f8abab3e9d2646cb2ce534".into(),
        }),
        via: None,
    }
}
#[test]
fn skyward_updates_from_its_newest_release_and_its_page_stays_until_the_build_is_in() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.refresh();
    let offered = |app: &mut App| {
        app.frame(&f, &i)
            .hits
            .iter()
            .any(|h| h.action == Action::Open(Screen::SkywardUpdate))
    };
    assert!(!offered(&mut app), "a phone whose WLAN the recovery does not bring up");
    app.state.wlan = Some(Wlan::default());
    app.refresh();
    assert!(!offered(&mut app), "a phone that keeps no recovery partition in each slot");
    app.state.recovery_partitions = true;
    app.refresh();
    assert!(
        tap(&mut app, &f, &i, Action::Open(Screen::SkywardUpdate)).is_empty(),
        "nothing is asked without a network"
    );
    assert!(tap(&mut app, &f, &i, Action::CheckSkyward).is_empty());
    assert_eq!(app.screen(), Screen::Wlan, "asking takes a network, which the WLAN page joins");
    app.key(Key::Back, 0);
    app.set_wlan(connected("Skyward"));
    assert_eq!(tap(&mut app, &f, &i, Action::CheckSkyward), vec![Effect::CheckSkyward]);
    assert_eq!(app.state.skyward_check, SkywardCheck::Asking);
    app.skyward_checked(Ok(skyward_newest(true)));
    let build = skyward_newest(true).build.unwrap();
    app.state.install = Install::Waiting;
    assert!(
        tap(&mut app, &f, &i, Action::UpdateSkyward).is_empty(),
        "an install under way goes first"
    );
    app.state.install = Install::Idle;
    assert_eq!(
        tap(&mut app, &f, &i, Action::UpdateSkyward),
        vec![Effect::UpdateSkyward(build.clone())]
    );
    app.skyward_progress(
        SkywardStage::Download(Progress::Downloading {
            done: 10_000_000,
            total: 42_739_608,
        }),
        2_000_000,
    );
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::SkywardUpdate, "the page stays while the build downloads");
    assert_eq!(tap(&mut app, &f, &i, Action::StopSkywardUpdate), vec![Effect::StopSkywardUpdate]);
    assert!(
        !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::StopSkywardUpdate),
        "stopping, it is not asked again"
    );
    app.skyward_ended(SkywardEnd::Stopped, 0);
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::UpdateSkyward, 2000),
        vec![Effect::UpdateSkyward(build.clone())],
        "a download stopped carries on"
    );
    app.skyward_progress(
        SkywardStage::Write {
            done: 1 << 20,
            total: 4 << 20,
        },
        0,
    );
    assert!(
        !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::StopSkywardUpdate),
        "writing goes on to its end"
    );
    app.key(Key::Back, 2100);
    assert_eq!(
        app.screen(),
        Screen::SkywardUpdate,
        "nor is the page left while the build is written"
    );
    app.skyward_ended(SkywardEnd::Updated(vec![Slot::A, Slot::B]), 3000);
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::RebootTo(Target::Recovery), 5000),
        vec![Effect::Reboot(Target::Recovery)]
    );
    app.key(Key::Back, 5100);
    assert_eq!(app.screen(), Screen::Settings);
}
#[test]
fn downloaded_recovery_requires_a_new_slide_before_any_write_effect() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::SkywardUpdate, W, H, PPD);
    app.state.wlan = Some(connected("Skyward"));
    app.skyward_checked(Ok(skyward_newest(true)));
    let build = skyward_newest(true).build.unwrap();
    assert_eq!(
        tap(&mut app, &f, &i, Action::UpdateSkyward),
        vec![Effect::UpdateSkyward(build.clone())]
    );
    app.skyward_ended(SkywardEnd::Ready(Slot::A), 0);
    assert!(matches!(
        app.state.skyward_job.as_ref().unwrap().outcome,
        Some(SkywardEnd::Ready(Slot::A))
    ));
    assert!(
        app.frame(&f, &i)
            .hits
            .iter()
            .any(|hit| hit.action == Action::Confirm(Confirm::SkywardUpdate))
    );
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|hit| hit.action == Action::Confirm(Confirm::SkywardUpdate))
        .unwrap()
        .rect;
    let y = track.centre().1 - app.top().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 1000);
    app.touch(Touch::Move(track.x + track.w, y), &f, 1000);
    assert_eq!(app.touch(Touch::Up, &f, 1000), vec![Effect::WriteSkyward { build, slot: Slot::A }]);
    assert!(matches!(
        app.state.skyward_job.as_ref().unwrap().stage,
        Some(SkywardStage::Write { .. })
    ));
}
#[test]
fn the_skyward_page_asks_github_as_it_opens_once_a_network_is_joined() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    app.state.wlan = Some(connected("Skyward"));
    app.state.recovery_partitions = true;
    app.refresh();
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::SkywardUpdate)),
        vec![Effect::CheckSkyward]
    );
    app.skyward_checked(Err(Unanswered::Failed(
        "GitHub answered 403 Forbidden: API rate limit exceeded".into(),
    )));
    app.key(Key::Back, 0);
    assert_eq!(
        tap(&mut app, &f, &i, Action::Open(Screen::SkywardUpdate)),
        vec![Effect::CheckSkyward],
        "a failed check is tried again"
    );
    app.skyward_checked(Ok(skyward_newest(false)));
    assert!(
        !app.frame(&f, &i).hits.iter().any(|h| h.action == Action::UpdateSkyward),
        "nothing newer to update to"
    );
    app.key(Key::Back, 0);
    assert!(
        tap(&mut app, &f, &i, Action::Open(Screen::SkywardUpdate)).is_empty(),
        "an answer heard is not asked again"
    );
}
#[test]
fn a_tap_on_the_text_moves_the_cursor_and_a_drag_or_a_flick_scrolls_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    let text: String = (0..200).map(|n| format!("line {n}\n")).collect();
    app.text_opened(&f, "/tmp/long.txt".into(), Ok(Some(text.into_bytes())), 0);
    let (card, pitch) = app.frame(&f, &i).scrollback.expect("the editor's text scrolls");
    let l = editor::layout(&app.theme(), &f);
    let ed = |app: &App| app.state.editor.clone().expect("the editor page shows a file");
    app.touch(Touch::Down(l.x0 + 4.2 * l.cell, l.y0 + 2.5 * l.pitch), &f, 0);
    app.touch(Touch::Up, &f, 0);
    assert_eq!(ed(&app).cursor(), (2, 4), "after \"line\" on the third row");
    let (x, y) = (card.x + card.w / 2.0, card.y + card.h / 2.0);
    app.touch(Touch::Down(x, y), &f, 100);
    app.touch(Touch::Move(x, y - pitch * 5.5), &f, 110);
    app.touch(Touch::Up, &f, 400);
    assert_eq!(ed(&app).top(), 5, "up shows the lines after");
    assert_eq!(ed(&app).cursor(), (2, 4), "a drag leaves the cursor where it was");
    assert_eq!(app.top().1, 0.0, "the page itself stays put");
    tap(&mut app, &f, &i, Action::TermKey(TermKey::Down));
    assert_eq!(
        (ed(&app).cursor(), ed(&app).top()),
        ((3, 4), 3),
        "moving the cursor brings it into view"
    );
    app.touch(Touch::Down(x, y), &f, 1000);
    for k in 1..=5 {
        app.touch(Touch::Move(x, y - pitch * k as f32), &f, 1000 + k * 8);
    }
    app.touch(Touch::Up, &f, 1045);
    let lifted = ed(&app).top();
    app.tick(1200);
    let gliding = ed(&app).top();
    assert!(gliding > lifted + 5, "a flick glides on: {lifted} to {gliding}");
    app.touch(Touch::Down(x, y), &f, 1210);
    app.touch(Touch::Up, &f, 1220);
    app.tick(2000);
    assert_eq!(ed(&app).top(), gliding, "a finger on it stops it there");
    assert_eq!(ed(&app).cursor(), (3, 4), "and puts no cursor down");
}
#[test]
fn a_name_the_folder_cannot_take_is_turned_down_before_renaming() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    tap(&mut app, &f, &i, Action::RenameEntry);
    let refused = [
        ("", "请输入名称"),
        ("..", "名称不能为\".\"或\"..\""),
        ("x/y", "名称不能包含\"/\""),
        ("DCIM", "该文件夹中已有\"DCIM\""),
    ];
    for (name, said) in refused {
        app.state.new_name = name.into();
        app.toast = None;
        assert!(
            tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty(),
            "{name:?} is not renamed to"
        );
        assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some(said));
        assert_eq!(app.screen(), Screen::Rename);
    }
    app.state.new_name = "a.zip".into();
    assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
    assert_eq!(app.screen(), Screen::FileInfo, "the name left as it was, the page goes");
}
#[test]
fn deleting_asks_with_a_swipe_and_runs_on_its_page() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    assert!(
        app.frame(&f, &i)
            .hits
            .iter()
            .all(|h| h.action != Action::Confirm(Confirm::DeleteEntry))
    );
    tap(&mut app, &f, &i, Action::AskDelete);
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::DeleteEntry),
        vec![Effect::Delete("/sdcard/a.zip".into())]
    );
    app.file_job_started(FileTask::Delete {
        path: "/sdcard/a.zip".into(),
    });
    assert_eq!(app.screen(), Screen::FileJob);
    assert_eq!(tap(&mut app, &f, &i, Action::StopJob), vec![Effect::StopJob]);
    app.key(Key::Back, 0);
    assert_eq!(app.screen(), Screen::FileJob, "it stays until stopped");
    app.file_job_ended(JobEnd::Stopped, 0);
    assert_eq!(app.screen(), Screen::FileJob, "how it ended stays to be read");
    app.key(Key::Back, 1000);
    assert_eq!(
        app.screen(),
        Screen::Files,
        "back past the entry's page, which was for an entry being deleted"
    );
}
/// A delete that has begun takes the entry its page described away with
/// it. Everything that page could still ask for from a frame drawn
/// before that has no entry to act on, and must say so: the run's own
/// `Err` and panic both restart the recovery, which starts at the home
/// page — a phone that goes home in the middle of deleting a file is
/// what that looks like to whoever is holding it.
#[test]
fn what_asks_for_an_entry_a_started_delete_took_away_says_so_and_stays() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    hold_entry(&mut app, &f, &i, 1);
    tap(&mut app, &f, &i, Action::AskDelete);
    assert_eq!(
        swipe(&mut app, &f, &i, Confirm::DeleteEntry),
        vec![Effect::Delete("/sdcard/a.zip".into())]
    );
    app.file_job_started(FileTask::Delete {
        path: "/sdcard/a.zip".into(),
    });
    assert_eq!(app.state.file, None, "the work takes the entry it describes");
    for (action, at) in [
        (Action::EditEntry, 1000),
        (Action::InstallEntry, 1001),
        (Action::RenameEntry, 1002),
        (Action::PickEntry(FileOp::Copy), 1003),
        (Action::ModeBit(0o100), 1004),
        (Action::Confirm(Confirm::DeleteEntry), 1005),
    ] {
        assert!(app.act(action, at).is_empty(), "{action:?} asks for an entry that is gone");
        assert_eq!(app.screen(), Screen::FileJob, "{action:?} leaves the work's page up");
    }
}
#[test]
fn work_that_ends_on_its_page_takes_the_title_of_how_it_ended() {
    let (f, i) = (fonts(), info());
    let mut app = in_sdcard(&f, &i);
    app.file_job_started(FileTask::Copy {
        path: "/sdcard/a.zip".into(),
        into: "/tmp".into(),
    });
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:00", 0);
    app.file_job_ended(JobEnd::Stopped, 0);
    app.compose(&mut c, &f, &i, "12:00", 16);
    assert_eq!(app.titles.as_ref().map(|((title, _), _, _)| *title), Some("复制未完成"));
}
#[test]
fn an_unreadable_folder_offers_nothing_to_open() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    app.set_files("/data".into(), Err("read /data: Permission denied (os error 13)".into()), LISTED);
    let fr = app.frame(&f, &i);
    assert!(fr.hits.is_empty() && fr.rows.is_none());
}
#[test]
fn a_long_folder_scrolls_to_its_last_entry_in_one_list() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    let many: Vec<FileRow> = (0..5000).map(|k| entry(&format!("IMG_{k:05}.jpg"), FileKind::File)).collect();
    app.set_files("/sdcard/DCIM/Camera".into(), Ok(many), LISTED);
    let rows = app.frame(&f, &i).rows.as_ref().expect("the entries are listed").count();
    assert_eq!(rows, 5000, "every entry has its row");
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:00", 0);
    let end = app.max_scroll();
    scroll_to(&mut app, &f, end, 1000);
    app.compose(&mut c, &f, &i, "12:00", 2000);
    assert!(
        app.drawn.len() < 40,
        "only the rows near the screen are kept drawn: {}",
        app.drawn.len()
    );
    assert!(app.drawn.iter().any(|(k, _)| *k == 4999), "the last row shows");
    tap_at(&mut app, &f, &i, Action::OpenEntry(4999), 3000);
    assert_eq!(app.screen(), Screen::FileInfo);
    assert_eq!(app.state.file.as_ref().map(|e| e.name.as_str()), Some("IMG_04999.jpg"));
}
#[test]
fn a_folder_gone_back_up_to_shows_where_it_was_left() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Files));
    let folders: Vec<FileRow> = (0..300).map(|k| entry(&format!("dir{k:03}"), folder(k))).collect();
    app.set_files("/".into(), Ok(folders.clone()), LISTED);
    app.frame(&f, &i);
    scroll_to(&mut app, &f, 9000.0, 0);
    let (_, y) = spot(&mut app, &f, &i, Action::OpenEntry(36));
    assert!(y > 0.0 && y < H as f32, "row 36 is on screen at {y}");
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::OpenEntry(36), 2000),
        vec![Effect::ReadDir("/dir036".into())]
    );
    app.set_files("/dir036".into(), Ok(vec![entry("a", FileKind::File)]), LISTED);
    assert_eq!(app.top().1, 0.0, "a folder opened starts from its top");
    assert_eq!(app.key(Key::Back, 3000), vec![Effect::ReadDir("/".into())]);
    app.set_files("/".into(), Ok(folders), LISTED);
    assert_eq!(app.top().1, 9000.0, "back up, the folder is where it was left");
}
#[test]
fn a_folder_opened_or_gone_back_up_to_has_its_bar_from_its_first_frame() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Advanced, W, H, PPD);
    tap(&mut app, &f, &i, Action::Open(Screen::Files));
    let folders: Vec<FileRow> = (0..300).map(|k| entry(&format!("dir{k:03}"), folder(k))).collect();
    app.set_files("/".into(), Ok(folders.clone()), LISTED);
    app.frame(&f, &i);
    scroll_to(&mut app, &f, 9000.0, 0);
    app.tick(2000);
    assert_eq!(
        (app.title.level(2000), app.bar.level(2000)),
        (1.0, 1.0),
        "scrolled down, the bar is up"
    );
    let mut c = Canvas::new(W, H);
    tap_at(&mut app, &f, &i, Action::OpenEntry(36), 3000);
    app.set_files("/dir036".into(), Ok(vec![entry("a", FileKind::File)]), LISTED);
    app.tick(3008);
    app.compose(&mut c, &f, &i, "12:00", 3008);
    assert_eq!(
        (app.title.level(3008), app.bar.level(3008)),
        (0.0, 0.0),
        "a folder opened at its top has only its large title"
    );
    app.tick(4008);
    assert_eq!(app.key(Key::Back, 4100), vec![Effect::ReadDir("/".into())]);
    app.set_files("/".into(), Ok(folders), LISTED);
    app.tick(4108);
    app.compose(&mut c, &f, &i, "12:00", 4108);
    assert_eq!(
        (app.title.level(4108), app.bar.level(4108)),
        (1.0, 1.0),
        "back up, the list is under the bar from the first frame"
    );
    assert_ne!(app.deadline(4108), Some(0), "nothing eases in after it");
}
#[test]
fn the_path_goes_up_from_its_capsules_even_held_under_the_bar() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Files, W, H, PPD);
    let many: Vec<FileRow> = (0..200).map(|k| entry(&format!("lib{k:03}.so"), FileKind::File)).collect();
    app.set_files("/system/lib64".into(), Ok(many), LISTED);
    assert_eq!(
        tap(&mut app, &f, &i, Action::OpenAncestor(1)),
        vec![Effect::ReadDir("/system".into())]
    );
    scroll_to(&mut app, &f, 6000.0, 0);
    app.tick(2000);
    let t = app.theme();
    let (_, y) = spot(&mut app, &f, &i, Action::OpenAncestor(0));
    assert!(
        y > widgets::bar_bottom(&t) && y < widgets::bar_bottom(&t) + t.m.dp(51.0),
        "the capsules stay under the bar: {y}"
    );
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:00", 2000);
    let (x, _) = spot(&mut app, &f, &i, Action::OpenAncestor(0));
    let px = |c: &Canvas, x: f32, y: f32| c.data[(y as usize * W as usize + x as usize) * 4..][..3].to_vec();
    assert_eq!(
        px(&c, x - t.m.dp(20.0), y),
        vec![243, 244, 247],
        "the root's capsule shows over the list"
    );
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::OpenAncestor(0), 3000),
        vec![Effect::ReadDir("/".into())]
    );
    app.touch(Touch::Down(W as f32 - t.m.dp(40.0), y), &f, 4000);
    assert!(app.touch(Touch::Up, &f, 4000).is_empty());
    assert_eq!(app.screen(), Screen::Files);
}
#[test]
fn rows_are_found_by_where_they_lie() {
    let rows = Rows {
        edges: vec![100.0, 352.0, 682.0, 934.0],
        action: Some(Action::OpenEntry),
    };
    assert_eq!(rows.count(), 3);
    assert_eq!(rows.at(99.0), None);
    assert_eq!(rows.at(100.0), Some(0));
    assert_eq!(rows.at(352.0), Some(1));
    assert_eq!(rows.at(933.0), Some(2));
    assert_eq!(rows.at(934.0), None);
    assert_eq!(rows.within(0.0, 100.0), 0..0);
    assert_eq!(rows.within(0.0, 101.0), 0..1);
    assert_eq!(rows.within(351.0, 700.0), 0..3);
    assert_eq!(rows.within(700.0, 5000.0), 2..3);
    assert_eq!(rows.within(934.0, 5000.0), 3..3);
}
#[test]
fn power_key_turns_the_panel_off_and_on() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    assert!(app.key(Key::Power, 0).is_empty());
    assert_eq!(app.panel(0), Some(SLIDER_START), "it waits for volume down first");
    assert_eq!(app.deadline(0), Some(CHORD_MS));
    assert_eq!(
        app.tick(CHORD_MS),
        vec![Effect::Ticks],
        "held on alone, it turns the panel off, ticking the motor"
    );
    assert!(!app.display_on());
    assert!(app.key_up(Key::Power, CHORD_MS + 50).is_empty());
    assert!(app.key(Key::Power, CHORD_MS + 60).is_empty());
    assert_eq!(app.panel(CHORD_MS + 60), Some(SLIDER_START));
    assert!(app.key_up(Key::Power, CHORD_MS + 100).is_empty());
    assert!(app.key(Key::Power, 1000).is_empty());
    assert_eq!(app.key_up(Key::Power, 1050), vec![Effect::Ticks]);
    assert!(!app.display_on(), "let go sooner, it turns the panel off then");
}
#[test]
fn the_power_key_turning_the_screen_off_ticks_the_motor_whatever_the_settings() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    assert_eq!(press(&mut app, Key::Power, 1000), vec![Effect::Ticks]);
    assert!(app.tick(1400).is_empty(), "once");
    app.state.vibrate = false;
    press(&mut app, Key::Power, 2000);
    assert_eq!(press(&mut app, Key::Power, 3000), vec![Effect::Ticks], "with vibration off too");
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    assert!(app.tick(OFF).is_empty(), "but not the screen going off by itself");
    assert!(!app.display_on());
}
#[test]
fn the_power_key_and_volume_down_together_take_a_screenshot() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.frame(&f, &i);
    app.state.vibrate = true;
    assert!(app.key(Key::Power, 0).is_empty());
    let shot = vec![Effect::Haptic, Effect::Screenshot { storage: false }];
    assert_eq!(app.key(Key::VolumeDown, CHORD_MS - 1), shot, "volume down follows in time");
    assert_eq!(app.chord, None, "nothing is left waiting");
    assert!(app.tick(1000).is_empty());
    assert!(app.key_up(Key::Power, 1000).is_empty());
    assert!(app.key_up(Key::VolumeDown, 1010).is_empty());
    assert!(app.display_on(), "the power key did not turn the panel off");
    assert!(!app.keys, "nor did volume down light a control");
    assert!(app.key(Key::VolumeDown, 2000).is_empty());
    assert_eq!(app.key(Key::Power, 2100), shot);
    assert!(app.key_up(Key::VolumeDown, 2150).is_empty());
    assert!(app.key_up(Key::Power, 2200).is_empty());
    assert!(app.display_on());
    assert!(!app.keys);
    app.state.vibrate = false;
    app.data_waiting(None, Vec::new());
    app.key(Key::Power, 3000);
    assert_eq!(app.key(Key::VolumeDown, 3010), vec![Effect::Screenshot { storage: true }]);
}
#[test]
fn the_power_key_and_volume_down_too_far_apart_each_do_their_own() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.frame(&f, &i);
    assert!(app.key(Key::VolumeDown, 0).is_empty());
    assert!(!app.keys, "volume down waits for the power key first");
    assert!(app.tick(CHORD_MS).is_empty());
    assert!(app.keys, "then lights a control on its own");
    assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::Search))));
    assert!(app.key(Key::Power, CHORD_MS + 10).is_empty(), "the power key came too late");
    assert!(app.key_up(Key::VolumeDown, CHORD_MS + 20).is_empty());
    assert!(app.tick(CHORD_MS * 2 + 10).is_empty());
    assert_eq!(
        app.screen(),
        Screen::Home,
        "the power key waits on the control lit until it comes up"
    );
    app.key_up(Key::Power, CHORD_MS * 3);
    assert_eq!(app.screen(), Screen::Search, "and then works it");
}
#[test]
fn a_screenshot_says_where_it_went() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.screenshot_saved(Ok(Shot::Album), 0);
    assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("截图已保存到相册"));
    app.screenshot_saved(Ok(Shot::Held), 0);
    assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("截图已暂存，解密后存入相册"));
    app.screenshot_saved(Err("No space left on device".into()), 0);
    assert_eq!(
        app.toast.as_ref().map(|t| t.0.as_str()),
        Some("截图保存失败：No space left on device")
    );
    app.screenshots_not_kept("Read-only file system", 0);
    assert_eq!(
        app.toast.as_ref().map(|t| t.0.as_str()),
        Some("暂存的截图未能存入相册：Read-only file system")
    );
}
/// With the default two minutes, when the panel dims and when it goes
/// off.
const DIMS: u64 = 113_000;
const OFF: u64 = 120_000;
#[test]
fn left_alone_the_panel_dims_then_goes_off() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    assert_eq!(app.deadline(0), Some(DIMS));
    app.tick(DIMS - 1);
    assert_eq!(app.panel(DIMS - 1), Some(SLIDER_START));
    app.tick(DIMS);
    assert_eq!(
        app.deadline(DIMS),
        Some(DIMS + BACKLIGHT_STEP_MS),
        "the backlight steps down a frame at a time"
    );
    let midway = app.panel(DIMS + 250).unwrap();
    assert!(midway < SLIDER_START && midway > dim_level(SLIDER_START), "{midway}");
    assert_eq!(app.panel(DIMS + 500), Some(dim_level(SLIDER_START)));
    assert_eq!(app.deadline(DIMS + 500), Some(OFF), "dim until it goes off");
    app.tick(OFF);
    assert!(!app.display_on());
    assert_eq!(
        app.panel(OFF),
        Some(dim_level(SLIDER_START)),
        "the content goes dark under the dim backlight"
    );
    app.tick(OFF + 365);
    assert_eq!(app.panel(OFF + 365), None);
    assert_eq!(app.deadline(OFF + 365), None, "nothing to wait for on a dark panel");
}
#[test]
fn time_before_the_first_tick_does_not_count_as_idle() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(5_000_000);
    assert_eq!(app.panel(5_000_000), Some(SLIDER_START));
    assert_eq!(app.deadline(5_000_000), Some(5_000_000 + DIMS));
    app.tick(5_000_000 + OFF);
    assert!(!app.display_on());
}
#[test]
fn the_dim_level_sits_below_the_brightness_set() {
    assert_eq!(dim_level(1.0), DIM_CEILING);
    assert_eq!(dim_level(0.06), MIN_BRIGHTNESS);
    assert_eq!(dim_level(0.03), MIN_BRIGHTNESS);
    assert_eq!(dim_level(0.01), 0.01, "never brighter than the level set");
    assert_eq!(dim_span(15_000), 3000);
    assert_eq!(dim_span(120_000), DIM_MS);
}
#[test]
fn a_touch_while_dim_brightens_and_still_counts() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    app.tick(DIMS);
    assert_eq!(app.panel(DIMS + 500), Some(dim_level(SLIDER_START)));
    let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
    app.touch(Touch::Down(x, y), &f, DIMS + 600);
    app.touch(Touch::Up, &f, DIMS + 650);
    assert_eq!(app.screen(), Screen::Settings, "the tap counts");
    assert_eq!(app.panel(DIMS + 800), Some(SLIDER_START), "back to full brightness");
    assert_eq!(app.deadline(DIMS + 800), Some(DIMS + 650 + DIMS), "the wait starts over");
}
#[test]
fn a_touch_on_the_dark_panel_lights_nothing() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
    press(&mut app, Key::Power, 0);
    assert!(app.touch(Touch::Down(x, y), &f, 100).is_empty());
    assert!(app.touch(Touch::Move(x, y - 300.0), &f, 120).is_empty());
    assert!(app.touch(Touch::Up, &f, 140).is_empty());
    assert!(!app.display_on(), "the panel stays dark");
    assert_eq!(app.screen(), Screen::Home, "the tap pressed nothing");
    assert_eq!(app.top().1, 0.0, "nor scrolled the page");
    assert!(app.touch(Touch::Down(x + W as f32 * 0.5, y), &f, 200).is_empty());
    assert!(app.touch(Touch::Up, &f, 220).is_empty());
    assert!(!app.display_on(), "a tap elsewhere is not the second of a double tap");
    press(&mut app, Key::Power, 400);
    assert!(app.display_on());
    tap_at(&mut app, &f, &i, Action::Open(Screen::Settings), 400 + WAKE_GUARD_MS + 10);
    assert_eq!(app.screen(), Screen::Settings, "a tap on the lit panel does");
}
/// Two taps on the dark panel light it, as the phone's own system's double
/// tap does, and neither tap works the page: they are one gesture that
/// lights the panel.
#[test]
fn two_taps_on_the_dark_panel_light_it() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
    press(&mut app, Key::Power, 0);
    for (at, lit) in [(100, false), (200, true)] {
        app.touch(Touch::Down(x, y), &f, at);
        app.touch(Touch::Up, &f, at + 20);
        assert_eq!(app.display_on(), lit, "after the tap at {at} ms");
    }
    assert_eq!(app.screen(), Screen::Home, "the taps pressed nothing: they lit the panel");
    assert_eq!(app.panel(320), Some(SLIDER_START), "and the panel comes up at full brightness");
    let after = 200 + WAKE_GUARD_MS + 10;
    app.touch(Touch::Down(x, y), &f, after);
    app.touch(Touch::Up, &f, after + 20);
    assert_eq!(app.screen(), Screen::Settings);
}
#[test]
fn double_tap_can_be_disabled_without_disabling_the_power_key() {
    let f = fonts();
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.act(Action::Toggle(Switch::DoubleTap), 0);
    assert!(!app.state.double_tap);
    press(&mut app, Key::Power, 10);
    for at in [100, 200] {
        app.touch(Touch::Down(300.0, 500.0), &f, at);
        app.touch(Touch::Up, &f, at + 20);
    }
    assert!(!app.display_on());
    press(&mut app, Key::Power, 400);
    assert!(app.display_on());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.double_tap_supported = false;
    press(&mut app, Key::Power, 0);
    app.touch(Touch::Down(300.0, 500.0), &f, 100);
    app.touch(Touch::Down(300.0, 500.0), &f, 200);
    assert!(!app.display_on(), "a preference does not invent hardware support");
}
#[test]
fn hardware_gesture_wakes_without_toggling_or_clicking() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
    press(&mut app, Key::Power, 0);
    app.hardware_wake(100);
    assert!(app.display_on());
    app.hardware_wake(110);
    assert!(app.display_on(), "a repeated wake event cannot turn the screen off");
    app.touch(Touch::Down(x, y), &f, 120);
    app.touch(Touch::Up, &f, 140);
    assert_eq!(app.screen(), Screen::Home, "wake keeps the stray-touch guard");
}
#[test]
fn hardware_gesture_obeys_preference_and_capability() {
    for (enabled, supported) in [(false, true), (true, false)] {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.double_tap = enabled;
        app.state.double_tap_supported = supported;
        press(&mut app, Key::Power, 0);
        app.hardware_wake(100);
        assert!(!app.display_on());
        press(&mut app, Key::Power, 200);
        assert!(app.display_on(), "power key remains independent");
    }
}
#[test]
fn the_double_tap_switch_is_offered_only_when_supported() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Settings, W, H, PPD);
    let offered = |app: &mut App| app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Toggle(Switch::DoubleTap));
    assert!(offered(&mut app));
    app.state.double_tap_supported = false;
    app.refresh();
    assert!(!offered(&mut app));
}
/// The two taps of one double tap are close together on the glass and
/// inside Android's own window; anything else is a tap of its own, and the
/// panel stays dark.
#[test]
fn taps_too_far_apart_or_too_late_do_not_light_the_panel() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
    press(&mut app, Key::Power, 0);
    app.touch(Touch::Down(x, y), &f, 100);
    app.touch(Touch::Up, &f, 120);
    app.touch(Touch::Down(x, y), &f, 100 + Tap::WINDOW_MS + 50);
    app.touch(Touch::Up, &f, 100 + Tap::WINDOW_MS + 70);
    assert!(!app.display_on(), "a tap past the window starts a count of its own");
    let far = W as f32 * (Tap::SLOP + 0.05);
    app.touch(Touch::Down(x + far, y), &f, 100 + Tap::WINDOW_MS + 100);
    app.touch(Touch::Up, &f, 100 + Tap::WINDOW_MS + 120);
    assert!(!app.display_on(), "a tap far from the last is not its pair");
    app.touch(Touch::Down(x + far, y), &f, 100 + Tap::WINDOW_MS + 150);
    app.touch(Touch::Up, &f, 100 + Tap::WINDOW_MS + 170);
    assert!(app.display_on());
}
#[test]
fn a_cable_coming_out_lights_the_dark_panel_and_keeps_a_lit_one_lit() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    app.tick(OFF);
    app.tick(OFF + 365);
    assert_eq!(app.panel(OFF + 365), None);
    let pulled = OFF + 1000;
    app.plugged(pulled, false);
    assert!(app.display_on(), "the cable coming out lights the panel");
    assert!(!app.charge_shown(), "and shows the page");
    assert_eq!(app.panel(pulled), Some(SLIDER_START));
    assert_eq!(
        app.deadline(pulled + 520),
        Some(pulled + DIMS),
        "for the screen-off time, once the content is up"
    );
    app.tick(pulled + DIMS);
    assert_eq!(app.panel(pulled + DIMS + 500), Some(dim_level(SLIDER_START)));
    let again = pulled + DIMS + 600;
    app.plugged(again, false);
    assert_eq!(
        app.panel(again + 150),
        Some(SLIDER_START),
        "a cable coming out brightens a lit panel"
    );
    assert_eq!(app.deadline(again + 150), Some(again + DIMS), "and the wait starts over");
    press(&mut app, Key::Power, again + 200);
    assert!(!app.display_on(), "the power key turns it off at once, as ever");
}
#[test]
fn a_cable_going_in_on_the_dark_panel_shows_the_charging_screen_and_goes_dark_again() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    press(&mut app, Key::Power, 0);
    app.tick(365);
    assert_eq!(app.panel(365), None);
    let plugged = 1000;
    app.plugged(plugged, true);
    assert!(app.display_on() && app.charge_shown(), "the charging screen lights the panel");
    assert_eq!(app.deadline(plugged + 1000), Some(0), "its tide moves a frame at a time");
    app.tick(plugged + CHARGE_MS - 1);
    assert!(app.display_on());
    app.tick(plugged + CHARGE_MS);
    assert!(!app.display_on(), "then the panel goes dark again");
    assert!(app.panel(plugged + CHARGE_MS + 100).is_some(), "as the charging screen goes dark");
    app.tick(plugged + CHARGE_MS + 365);
    assert_eq!(app.panel(plugged + CHARGE_MS + 365), None);
    assert!(app.charging.is_none(), "and goes with it");
}
#[test]
fn a_cable_going_in_on_a_lit_panel_brings_the_charging_screen_over_the_page_and_away() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    app.plugged(1000, true);
    assert!(app.display_on() && app.charge_shown());
    app.tick(1000 + CHARGE_MS);
    assert!(app.display_on(), "the panel stays lit");
    assert!(!app.charge_shown(), "as the charging screen goes for the page");
    app.tick(1000 + CHARGE_MS + 300);
    assert!(app.charging.is_none());
}
#[test]
fn a_touch_or_a_volume_key_sends_the_charging_screen_away_and_keeps_the_panel_lit() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
    app.tick(0);
    press(&mut app, Key::Power, 0);
    app.tick(365);
    app.plugged(1000, true);
    app.touch(Touch::Down(x, y), &f, 1500);
    app.touch(Touch::Up, &f, 1550);
    assert!(!app.charge_shown(), "the touch sent it away");
    assert_eq!(app.screen(), Screen::Home, "and pressed nothing");
    app.tick(1000 + CHARGE_MS);
    assert!(app.display_on(), "the panel stays lit for the page");
    app.plugged(10_000, true);
    press(&mut app, Key::VolumeDown, 10_500);
    assert!(!app.charge_shown(), "a volume key sends it away too");
    assert!(app.display_on());
}
#[test]
fn the_charging_screen_shows_the_level_over_a_tide_of_light() {
    let (f, i) = (
        fonts(),
        DeviceInfo {
            battery: Some(battery_at(64, Charge::Charging)),
            ..info()
        },
    );
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    press(&mut app, Key::Power, 0);
    app.tick(365);
    app.plugged(1000, true);
    let mut c = Canvas::new(W, H);
    app.compose(&mut c, &f, &i, "12:00", 3000);
    let px = |x: u32, y: u32| {
        let k = ((y * W + x) * 4) as usize;
        [c.data[k], c.data[k + 1], c.data[k + 2]]
    };
    assert_eq!(px(0, 0), [0, 0, 0], "on black");
    let foot = px(W / 2, H - 4);
    assert!(
        foot[2] > 40 && foot[2] > foot[0] * 4 && foot[2] > foot[1] * 2,
        "the tide's light at the foot is blue: {foot:?}"
    );
}
#[test]
fn the_content_goes_dark_under_the_backlight_it_had_before_the_panel_goes_off() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    press(&mut app, Key::Power, 1000);
    assert!(!app.display_on(), "touches and keys take the screen as off at once");
    assert_eq!(
        app.panel(1000),
        Some(SLIDER_START),
        "while its content goes dark under the backlight it had"
    );
    assert_eq!(app.deadline(1000), Some(0), "a frame at a time");
    assert_eq!(app.panel(1300), Some(SLIDER_START));
    app.tick(1365);
    assert_eq!(app.panel(1365), None, "then the panel goes off");
    assert_eq!(app.deadline(1365), None);
}
#[test]
fn the_power_key_as_the_content_goes_dark_brings_it_back() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    press(&mut app, Key::Power, 1000);
    app.tick(1100);
    press(&mut app, Key::Power, 1100);
    assert!(app.display_on(), "the power key lit it again");
    let level = app.curtain.level(1100);
    assert!(level > 0.0 && level < 1.0, "from as dark as it had gone: {level}");
    assert_eq!(app.curtain.level(1100 + 520), 1.0);
}
#[test]
fn the_content_comes_up_from_black_as_the_panel_lights() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.tick(0);
    press(&mut app, Key::Power, 0);
    app.tick(365);
    press(&mut app, Key::Power, 1000);
    let mut c = Canvas::new(W, H);
    let light = |c: &Canvas| {
        c.data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[0] as u64 + p[1] as u64 + p[2] as u64)
            .sum::<u64>()
    };
    app.compose(&mut c, &f, &i, "12:00", 1000);
    let first = light(&c);
    app.compose(&mut c, &f, &i, "12:00", 1260);
    let halfway = light(&c);
    app.compose(&mut c, &f, &i, "12:00", 1520);
    let full = light(&c);
    assert_eq!(first, 0, "black as the panel lights");
    assert!(halfway > full / 2 && halfway < full, "{halfway} of {full} halfway through");
}
#[test]
fn only_the_power_key_lights_the_dark_panel_and_does_nothing_else() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Settings, &f, &i);
    press(&mut app, Key::Power, 0);
    for (key, at) in [(Key::Back, 10), (Key::VolumeUp, 20), (Key::VolumeDown, 30)] {
        assert!(press(&mut app, key, at).is_empty());
        assert!(!app.display_on(), "{key:?} leaves the panel dark");
    }
    assert!(press(&mut app, Key::Power, 40).is_empty());
    assert!(app.display_on());
    assert_eq!(app.screen(), Screen::Settings, "back did not go back");
}
#[test]
fn a_stray_press_just_after_the_panel_lights_only_keeps_it_awake() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Reboot, &f, &i);
    press(&mut app, Key::Power, 0);
    app.key(Key::Power, 100);
    assert!(app.display_on());
    assert!(app.key(Key::Power, 300).is_empty());
    assert!(app.display_on(), "a second power press did not turn it off");
    assert!(app.key(Key::Back, 350).is_empty());
    assert_eq!(app.screen(), Screen::Reboot, "back did not go back");
    press(&mut app, Key::Power, 600);
    assert!(!app.display_on());
}
#[test]
fn a_stray_touch_just_after_an_install_lights_the_panel_presses_nothing() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    press(&mut app, Key::Power, 0);
    app.install(
        Event::Finished(Outcome::Installed {
            slot: Slot::A,
            wipe: false,
        }),
        100,
    );
    assert!(app.display_on(), "the finished install lit the panel");
    assert!(tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), 200).is_empty());
    assert_eq!(app.screen(), Screen::Installing, "the stray tap did not reboot");
    assert_eq!(
        tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), 700),
        vec![Effect::Reboot(Target::System)]
    );
}
#[test]
fn a_finger_held_as_the_panel_goes_off_lets_go() {
    let (f, i) = (fonts(), info());
    let mut app = App::new(Screen::Reboot, W, H, PPD);
    let track = app
        .frame(&f, &i)
        .hits
        .iter()
        .find(|h| h.action == Action::Confirm(Confirm::Reboot))
        .unwrap()
        .rect;
    let y = track.centre().1;
    app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 0);
    app.touch(Touch::Move(track.x + track.w * 0.6, y), &f, 10);
    assert!(app.state.confirm > 0.0);
    press(&mut app, Key::Power, 20);
    assert_eq!(app.state.confirm, 0.0, "the knob is back at the start");
    app.touch(Touch::Move(track.x + track.w, y), &f, 30);
    assert!(app.touch(Touch::Up, &f, 40).is_empty(), "no reboot from a dark panel");
    assert!(!app.display_on());
}
#[test]
fn never_leaves_the_panel_on() {
    let mut app = App::new(Screen::Home, W, H, PPD);
    app.state.screen_off = SCREEN_OFF.iter().position(Option::is_none).unwrap();
    app.tick(0);
    assert_eq!(app.deadline(0), None);
    let day = 24 * 3600 * 1000;
    app.tick(day);
    assert_eq!(app.panel(day), Some(SLIDER_START));
}
#[test]
fn the_installer_lights_the_panel_when_it_needs_the_user() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    press(&mut app, Key::Power, 0);
    app.install(Event::Checking { size: 1 << 20 }, 10);
    assert!(!app.display_on(), "reading the package does not light it");
    app.install(
        Event::Image {
            kind: ImageKind::Dtbo,
            size: 1 << 20,
            partitions: vec!["dtbo"],
        },
        20,
    );
    assert!(app.display_on(), "an image waiting for its partition does");
    app.tick(20 + 520);
    assert_eq!(app.deadline(20 + 520), Some(20 + DIMS));
    app.state.install = Install::Flashing {
        partition: "dtbo",
        stage: None,
    };
    press(&mut app, Key::Power, 500);
    app.install(Event::Progress(Stage::Flashing { done: 1, total: 2 }), 510);
    assert!(!app.display_on(), "progress does not");
    app.install(
        Event::Finished(Outcome::Flashed {
            partition: "dtbo",
            slots: vec![Slot::A],
        }),
        520,
    );
    assert!(app.display_on(), "the end does");
}
#[test]
fn an_install_ending_after_its_page_was_left_keeps_the_panel_dark() {
    let (f, i) = (fonts(), info());
    let mut app = sideloading(&f, &i);
    assert_eq!(app.key(Key::Back, 0), vec![Effect::Cancel]);
    press(&mut app, Key::Power, 10);
    app.install(Event::Finished(Outcome::Cancelled), 20);
    assert!(!app.display_on());
    assert_eq!(app.state.install, Install::Idle);
}
#[test]
fn choosing_how_long_the_screen_stays_on() {
    let (f, i) = (fonts(), info());
    let mut app = opened(Screen::Settings, &f, &i);
    tap(&mut app, &f, &i, Action::Open(Screen::ScreenOff));
    assert_eq!(app.screen(), Screen::ScreenOff);
    let frame = app.frame(&f, &i);
    assert_eq!(frame.title, "息屏时间");
    assert!(frame.back);
    let actions: Vec<Action> = frame.hits.iter().map(|h| h.action).collect();
    assert_eq!(actions, (0..SCREEN_OFF.len()).map(Action::ScreenOff).collect::<Vec<_>>());
    tap(&mut app, &f, &i, Action::ScreenOff(0));
    assert_eq!(app.state.screen_off, 0);
    assert_eq!(app.screen(), Screen::ScreenOff, "the page stays");
    app.tick(1000);
    assert_eq!(app.deadline(1000), Some(12_000));
    app.tick(15_000);
    assert!(!app.display_on());
}
