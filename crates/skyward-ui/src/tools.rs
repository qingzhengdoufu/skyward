use crate::{
    app::*,
    canvas::Canvas,
    color::Rgba,
    icons::Icon,
    pages::{Band, Ctx},
    widgets::{self, Caption, Row, Trail},
};

fn canvas(cx: &Ctx, height: u32) -> Canvas {
    let band = cx.band.unwrap_or(Band {
        rows: (0, height),
        cols: (0, cx.t.m.w),
    });
    Canvas::band(cx.t.m.w, height, band.rows.0..band.rows.1, band.cols.0..band.cols.1, cx.t.pal.bg)
}
fn hits(rects: &[crate::geom::Rect], actions: &[Action]) -> Vec<Hit> {
    rects.iter().zip(actions).map(|(&rect, &action)| Hit { rect, action }).collect()
}

pub fn modules(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let top = t.m.content_top;
    let count = s.root_modules.as_ref().and_then(|r| r.as_ref().ok()).map_or(0, |(_, m)| m.len());
    let mut c = canvas(cx, t.m.h.max((top + count as f32 * t.m.dp(170.0) + t.m.dp(280.0)) as u32));
    let mut out = Vec::new();
    let mut y = top;
    let read = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[Row::new(l.pick(["刷新模块", "重新整理模組", "Refresh modules"])).icon(Rgba::rgb(52, 130, 255), Icon::Restore)],
    );
    out.extend(hits(&read.rows, &[Action::ReadRootModules]));
    y = read.bottom + t.m.card_gap;
    if !s.root_notice.is_empty() {
        y = widgets::footnote(&mut c, t, f, y, &s.root_notice);
    }
    match &s.root_modules {
        Some(Ok((root, modules))) => {
            if !root.is_empty() {
                y = widgets::section(&mut c, t, f, y, root);
            }
            for (i, module) in modules.iter().enumerate() {
                let status = l.pick(if module.removing {
                    ["重启后移除", "重新啟動後移除", "Removal pending"]
                } else if module.enabled {
                    ["已启用", "已啟用", "Enabled"]
                } else {
                    ["已禁用", "已停用", "Disabled"]
                });
                let sub = format!("{} | {} | {}", module.version, module.author, status);
                let rows = [
                    Row::new(&module.name).subtitle(&sub).trail(Trail::Switch(module.enabled)),
                    Row::new(l.pick(if module.removing {
                        ["取消移除", "取消移除", "Cancel removal"]
                    } else {
                        ["移除模块", "移除模組", "Remove module"]
                    }))
                    .icon(Rgba::rgb(220, 65, 65), Icon::Wipe),
                ];
                let list = widgets::list(&mut c, t, f, y, &rows);
                out.extend(hits(&list.rows, &[Action::RootModule(i, 0), Action::RootModule(i, 1)]));
                y = list.bottom + t.m.card_gap;
            }
            if modules.is_empty() {
                y = widgets::footnote(&mut c, t, f, y, l.pick(["未发现模块", "未發現模組", "No modules found"]));
            }
        }
        Some(Err(why)) => y = widgets::footnote(&mut c, t, f, y, why),
        None => y = widgets::footnote(&mut c, t, f, y, l.pick(["正在读取", "正在讀取", "Reading"])),
    }
    Frame::new(
        t,
        c.crop_height((y + t.m.dp(24.0)) as u32),
        out,
        l.pick(["Root 模块", "Root 模組", "Root modules"]),
        top,
    )
}

pub fn partitions(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let top = t.m.content_top;
    let mut c = canvas(cx, t.m.h * 3);
    let mut out = Vec::new();
    let rows = [
        Row::new(l.pick(["刷入分区镜像", "刷入分割區映像檔", "Flash partition image"])).icon(Rgba::rgb(29, 205, 58), Icon::Install),
        Row::new(l.pick(["导出单分区镜像", "匯出單分割區映像檔", "Export partition image"])).icon(Rgba::rgb(52, 130, 255), Icon::Backup),
        Row::new(l.pick(["文件系统管理", "檔案系統管理", "Filesystems"])).icon(Rgba::rgb(140, 157, 176), Icon::Mount),
        Row::new(l.pick(["挂载管理", "掛載管理", "Mounts"])).icon(Rgba::rgb(119, 103, 249), Icon::Layers),
    ];
    let list = widgets::list(&mut c, t, f, top, &rows);
    out.extend(hits(
        &list.rows,
        &[
            Action::FlashPartitionImage,
            Action::Open(Screen::ImageBackup),
            Action::Open(Screen::Filesystems),
            Action::Open(Screen::Mount),
        ],
    ));
    let mut y = list.bottom + t.m.card_gap;
    if let Some(Ok(parts)) = &s.image_partitions {
        for (name, size) in parts {
            let sub = if name == "super" {
                l.pick([
                    "共享分区，包含双槽逻辑分区",
                    "共用分割區，包含雙槽邏輯分割區",
                    "Shared; contains both slots",
                ])
            } else {
                ""
            };
            let value = crate::fmt::file_size(*size);
            let rows = [Row::new(name).subtitle(sub).trail(Trail::Text(&value))];
            y = widgets::list(&mut c, t, f, y, &rows).bottom;
        }
    }
    Frame::new(
        t,
        c.crop_height((y + t.m.dp(24.0)) as u32),
        out,
        l.pick(["分区管理", "分割區管理", "Partitions"]),
        top,
    )
}

pub fn control(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let top = t.m.content_top;
    let mut c = canvas(cx, t.m.h * 3);
    let mut out = Vec::new();
    let on = s.wlan.as_ref().is_some_and(|w| w.power == skyward_net::wlan::Power::On);
    let mut controls = vec![
        ("WiFi", Icon::Wlan, on, s.wlan.is_some(), Action::Toggle(Switch::Wlan)),
        (
            l.pick(["震动", "震動", "Haptics"]),
            Icon::Phone,
            s.vibrate,
            true,
            Action::Toggle(Switch::Vibrate),
        ),
        ("MTP", Icon::Usb, s.file_transfer, true, Action::Toggle(Switch::FileTransfer)),
    ];
    if s.double_tap_supported {
        controls.push((
            l.pick(["双击唤醒", "點兩下喚醒", "Tap to wake"]),
            Icon::Power,
            s.double_tap,
            true,
            Action::Toggle(Switch::DoubleTap),
        ));
    }
    let cell = (t.m.w as f32 - t.m.margin * 2.0) / controls.len() as f32;
    let side = t.m.dp(54.0).min(cell - t.m.dp(12.0));
    for (index, (label, icon, enabled, available, action)) in controls.iter().enumerate() {
        let x = t.m.margin + cell * (index as f32 + 0.5);
        let cy = top + side / 2.0;
        let color = if *enabled && *available {
            Rgba::rgb(52, 130, 255)
        } else {
            t.pal.search
        };
        c.fill_round_rect(x - side / 2.0, top, side, side, side / 2.0, color);
        crate::icons::glyph(
            &mut c,
            *icon,
            x,
            cy,
            t.m.dp(26.0),
            if *enabled && *available {
                Rgba::rgb(255, 255, 255)
            } else {
                t.pal.subtitle
            },
        );
        let size = t.m.dp(11.0);
        let width = c.text_width(&f.label, label, size);
        c.text_top(
            &f.label,
            label,
            x - width / 2.0,
            top + side + t.m.dp(9.0),
            size,
            t.pal.text,
            Some(cell),
        );
        if *available {
            out.push(Hit {
                rect: crate::geom::Rect::new(x - cell / 2.0, top, cell, side + t.m.dp(36.0)),
                action: *action,
            });
        }
    }
    let y = widgets::section(&mut c, t, f, top + side + t.m.dp(40.0), l.pick(["亮度", "亮度", "Brightness"]));
    let slider = widgets::slider_card(
        &mut c,
        t,
        f,
        y,
        Caption::Sun,
        l.pick(["屏幕亮度", "螢幕亮度", "Screen brightness"]),
        s.brightness,
        None,
    );
    out.push(Hit {
        rect: slider.track,
        action: Action::Slider(Slider::Brightness),
    });
    let mut y = slider.bottom + t.m.card_gap;
    let data = crate::fmt::data(l, &s.data);
    let slot = cx.info.slot.map(|s| s.letter().to_string()).unwrap_or_default();
    let battery = cx.info.battery.as_ref().map(|b| format!("{}%", b.level)).unwrap_or_default();
    y = widgets::info_card(
        &mut c,
        t,
        f,
        y,
        &[
            ("Data", data.to_string()),
            (l.pick(["槽位", "槽位", "Slot"]), slot),
            (l.pick(["电量", "電量", "Battery"]), battery),
        ],
    );
    if let Some((title, detail, share)) = task(s) {
        y = widgets::progress_card(&mut c, t, f, y + t.m.card_gap, &title, &detail, share);
    }
    for d in &s.browser.downloads {
        if let BrowserSave::Downloading { done, total } = d.state {
            let detail = format!(
                "{} / {}",
                crate::fmt::file_size(done),
                total.map(crate::fmt::file_size).unwrap_or_default()
            );
            y = widgets::progress_card(
                &mut c,
                &crate::theme::Theme {
                    pal: crate::theme::Palette {
                        accent: Rgba::rgb(29, 205, 58),
                        ..t.pal
                    },
                    ..*t
                },
                f,
                y + t.m.card_gap,
                &d.name,
                &detail,
                total.filter(|n| *n > 0).map(|n| done as f32 / n as f32),
            );
        }
    }
    let browser = widgets::list(
        &mut c,
        t,
        f,
        y + t.m.card_gap,
        &[Row::new(l.pick(["浏览器", "瀏覽器", "Browser"])).icon(Rgba::rgb(52, 130, 255), Icon::Globe)],
    );
    out.extend(hits(&browser.rows, &[Action::Open(Screen::Browser)]));
    let close = widgets::button(&mut c, t, f, browser.bottom + t.m.card_gap, l.pick(["收起", "收起", "Close"]));
    out.push(Hit {
        rect: close,
        action: Action::CloseControlCenter,
    });
    Frame::new(
        t,
        c.crop_height((close.y + close.h + t.m.dp(24.0)) as u32),
        out,
        l.pick(["控制中心", "控制中心", "Control center"]),
        top,
    )
}

fn task(s: &State) -> Option<(String, String, Option<f32>)> {
    let progress = match &s.install {
        Install::Updating { stage, .. } => Some(crate::pages::update_stage(s.lang, stage.as_ref())),
        Install::Systeming { stage, .. } => Some(crate::pages::system_stage(s.lang, stage.as_ref())),
        Install::Scripting { stage, running: true, .. } => {
            let (title, share) = crate::pages::script_stage(s.lang, stage.as_ref(), &s.script_output);
            Some((title.into(), share))
        }
        Install::Flashing {
            partition,
            stage: Some(skyward_install::Stage::Flashing { done, total }),
        } => Some((
            format!("{partition}: {} / {}", crate::fmt::file_size(*done), crate::fmt::file_size(*total)),
            Some(*done as f32 / (*total).max(1) as f32),
        )),
        Install::Checking {
            stage: Some(skyward_install::Stage::Signature { done, total }),
            ..
        } => Some((
            s.lang.pick(["验证签名", "驗證簽章", "Verifying signature"]).into(),
            Some(*done as f32 / (*total).max(1) as f32),
        )),
        _ => None,
    };
    if let Some((detail, share)) = progress {
        return Some((s.lang.pick(["安装进度", "安裝進度", "Installation"]).into(), detail, share));
    }
    if matches!(s.install, Install::Scripting { running: true, .. }) {
        return Some((
            s.lang
                .pick(["正在运行刷包脚本", "正在執行安裝指令稿", "Install script running"])
                .into(),
            s.script_output.last().cloned().unwrap_or_default(),
            None,
        ));
    }
    if let Some(job) = &s.download
        && job.outcome.is_none()
    {
        return Some((
            s.lang.pick(["系统下载", "系統下載", "System download"]).into(),
            format!("{} B/s", job.rate),
            job.progress.as_ref().and_then(|p| match p {
                skyward_net::download::Progress::Downloading { done, total } => Some(*done as f32 / (*total).max(1) as f32),
                _ => None,
            }),
        ));
    }
    None
}
