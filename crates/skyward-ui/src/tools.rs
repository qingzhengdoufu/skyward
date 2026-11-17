use crate::{
    app::*,
    canvas::Canvas,
    color::Rgba,
    icons::Icon,
    pages::{Band, Ctx},
    widgets::{self, Row, Trail},
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

pub fn tasks(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let top = t.m.content_top;
    let mut c = canvas(cx, t.m.h * 2);
    let mut rows = vec![
        Row::new(l.pick(["返回上一任务", "返回上一工作", "Resume previous task"])).icon(Rgba::rgb(52, 130, 255), Icon::Restore),
        Row::new(l.pick(["浏览器", "瀏覽器", "Browser"])).icon(Rgba::rgb(0, 160, 160), Icon::Globe),
    ];
    let mut actions = vec![Action::ResumeTask, Action::Open(Screen::Browser)];
    if !matches!(s.install, Install::Idle) {
        rows.push(Row::new(l.pick(["安装任务", "安裝工作", "Installation"])).icon(Rgba::rgb(29, 205, 58), Icon::Install));
        actions.push(Action::Open(Screen::Installing));
    }
    rows.push(Row::new(l.pick(["浏览器下载", "瀏覽器下載", "Browser downloads"])).icon(Rgba::rgb(29, 205, 58), Icon::Folder));
    actions.push(Action::Open(Screen::BrowserDownloads));
    rows.push(Row::new(l.pick(["系统下载", "系統下載", "System download"])).icon(Rgba::rgb(52, 130, 255), Icon::Phone));
    actions.push(Action::Open(Screen::Update));
    let list = widgets::list(&mut c, t, f, top, &rows);
    Frame::new(
        t,
        c.crop_height((list.bottom + t.m.dp(24.0)) as u32),
        hits(&list.rows, &actions),
        l.pick(["任务", "工作", "Tasks"]),
        top,
    )
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
        let sizes: Vec<_> = parts.iter().map(|(_, size)| crate::fmt::file_size(*size)).collect();
        let rows: Vec<_> = parts
            .iter()
            .zip(&sizes)
            .map(|((name, _), size)| {
                let sub = if name == "super" {
                    l.pick([
                        "共享分区，包含双槽逻辑分区",
                        "共用分割區，包含雙槽邏輯分割區",
                        "Shared; contains both slots",
                    ])
                } else {
                    ""
                };
                Row::new(name).subtitle(sub).trail(Trail::Text(size))
            })
            .collect();
        y = widgets::list(&mut c, t, f, y, &rows).bottom;
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
    let m = &t.m;
    let mut c = canvas(cx, m.h * 3);
    let background = Rgba::rgb(26, 26, 28).with_alpha(150);
    let panel = Rgba::rgb(29, 29, 31).with_alpha(210);
    let inactive = Rgba::rgb(53, 53, 55);
    let blue = Rgba::rgb(0, 132, 216);
    c.clear(background);
    let mut out = Vec::new();
    let x = m.dp(16.0);
    let width = m.w as f32 - x * 2.0;
    c.text_top(&f.body, &s.control_clock, x, m.dp(28.0), m.dp(44.0), t.pal.text, None);
    c.text_top(
        &f.body,
        &s.control_date,
        x,
        m.dp(80.0),
        m.dp(13.0),
        t.pal.text,
        Some(width - m.dp(44.0)),
    );
    let network = s
        .wlan
        .as_ref()
        .and_then(|w| w.joined.as_ref())
        .map(|j| j.name.as_str())
        .unwrap_or("WiFi");
    c.text_top(
        &f.body,
        network,
        x,
        m.dp(102.0),
        m.dp(12.0),
        t.pal.subtitle,
        Some(width - m.dp(60.0)),
    );
    let wifi = s.wlan.as_ref().is_some_and(|w| w.power == skyward_net::wlan::Power::On);
    let mut controls = vec![
        (network, Icon::Wlan, wifi, s.wlan.is_some(), Action::Toggle(Switch::Wlan)),
        (
            l.pick(["震动", "震動", "Haptics"]),
            Icon::Phone,
            s.vibrate,
            true,
            Action::Toggle(Switch::Vibrate),
        ),
        ("MTP", Icon::Usb, s.file_transfer, true, Action::Toggle(Switch::FileTransfer)),
        (
            l.pick(["双击唤醒", "點兩下喚醒", "Tap to wake"]),
            Icon::Power,
            s.double_tap,
            s.double_tap_supported,
            Action::Toggle(Switch::DoubleTap),
        ),
        (
            l.pick(["浏览器", "瀏覽器", "Browser"]),
            Icon::Globe,
            false,
            true,
            Action::Open(Screen::Browser),
        ),
        (
            l.pick(["下载", "下載", "Downloads"]),
            Icon::Folder,
            false,
            true,
            Action::Open(Screen::BrowserDownloads),
        ),
        (
            l.pick(["文件管理", "檔案管理", "Files"]),
            Icon::Folder,
            false,
            true,
            Action::Open(Screen::Files),
        ),
        (
            l.pick(["终端", "終端機", "Terminal"]),
            Icon::Terminal,
            false,
            true,
            Action::Open(Screen::Terminal),
        ),
        (
            l.pick(["Root 模块", "Root 模組", "Modules"]),
            Icon::Layers,
            false,
            true,
            Action::Open(Screen::RootModules),
        ),
        (
            l.pick(["分区管理", "分割區管理", "Partitions"]),
            Icon::Mount,
            false,
            true,
            Action::Open(Screen::Partitions),
        ),
        (l.pick(["任务", "工作", "Tasks"]), Icon::More, false, true, Action::NavTasks),
        (
            l.pick(["设置", "設定", "Settings"]),
            Icon::Gear,
            false,
            true,
            Action::Open(Screen::Settings),
        ),
    ];
    if !s.double_tap_supported {
        controls[3] = (
            l.pick(["网络", "網路", "Network"]),
            Icon::Wlan,
            false,
            s.wlan.is_some(),
            Action::Open(Screen::Wlan),
        );
    }
    let top = m.dp(132.0);
    let pad = m.dp(12.0);
    let cell = (width - pad * 2.0) / 4.0;
    let pitch = m.dp(88.0);
    let side = m.dp(48.0).min(cell - m.dp(14.0));
    let grid_h = pad * 2.0 + pitch * 3.0;
    c.fill_round_rect(x, top, width, grid_h, m.dp(8.0), panel);
    for (index, (label, icon, enabled, available, action)) in controls.iter().enumerate() {
        let cx = x + pad + cell * (index % 4) as f32 + cell / 2.0;
        let y = top + pad + pitch * (index / 4) as f32;
        let color = if *enabled && *available { blue } else { inactive };
        c.fill_circle(cx, y + side / 2.0, side / 2.0, color);
        crate::icons::glyph(
            &mut c,
            *icon,
            cx,
            y + side / 2.0,
            m.dp(22.0),
            if *available { t.pal.text } else { t.pal.subtitle },
        );
        let max = cell - m.dp(4.0);
        let normal = m.dp(11.5);
        let measured = c.text_width(&f.label, label, normal);
        let size = if measured > max {
            normal * (max / measured).max(0.72)
        } else {
            normal
        };
        let tw = c.text_width(&f.label, label, size).min(max);
        c.text_top(
            &f.label,
            label,
            cx - tw / 2.0,
            y + side + m.dp(8.0),
            size,
            if *available { t.pal.text } else { t.pal.subtitle },
            Some(max),
        );
        if *available {
            out.push(Hit {
                rect: crate::geom::Rect::new(cx - cell / 2.0, y, cell, pitch),
                action: *action,
            });
        }
    }
    let slider = crate::geom::Rect::new(x, top + grid_h + m.dp(12.0), width, m.dp(42.0));
    c.fill_round_rect(slider.x, slider.y, slider.w, slider.h, m.dp(8.0), panel);
    let filled = (slider.w * s.brightness.clamp(0.0, 1.0)).max(m.dp(8.0));
    c.fill_round_rect(slider.x, slider.y, filled, slider.h, m.dp(8.0), Rgba::rgb(158, 158, 160));
    let (sunx, suny) = (slider.x + m.dp(22.0), slider.y + slider.h / 2.0);
    c.fill_circle(sunx, suny, m.dp(4.0), t.pal.text);
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        let (dx, dy) = (a.cos(), a.sin());
        c.line(
            sunx + dx * m.dp(7.0),
            suny + dy * m.dp(7.0),
            sunx + dx * m.dp(9.0),
            suny + dy * m.dp(9.0),
            m.dp(1.5),
            t.pal.text,
        );
    }
    out.push(Hit {
        rect: slider,
        action: Action::Slider(Slider::Brightness),
    });
    let handle = crate::geom::Rect::new(
        m.w as f32 / 2.0 - m.dp(18.0),
        slider.y + slider.h + m.dp(10.0),
        m.dp(36.0),
        m.dp(4.0),
    );
    c.fill_round_rect(handle.x, handle.y, handle.w, handle.h, handle.h / 2.0, t.pal.subtitle);
    out.push(Hit {
        rect: crate::geom::Rect::new(handle.x - m.dp(24.0), handle.y - m.dp(5.0), handle.w + m.dp(48.0), m.dp(18.0)),
        action: Action::CloseControlCenter,
    });
    let mut y = handle.y + m.dp(20.0);
    let battery = cx
        .info
        .battery
        .as_ref()
        .map(|b| {
            format!(
                "{}%{}",
                b.level,
                b.temp.map(|v| format!("  {:.1} C", v as f32 / 10.0)).unwrap_or_default()
            )
        })
        .unwrap_or_default();
    let slot = cx
        .info
        .slot
        .map(|slot| format!("{} {}", l.pick(["槽位", "槽位", "Slot"]), slot.letter()))
        .unwrap_or_default();
    let detail = format!("{}  {}  {}", crate::fmt::data(l, &s.data), slot, battery);
    let row = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[Row::new(l.pick(["设备状态", "裝置狀態", "Device status"]))
            .icon(blue, Icon::Phone)
            .subtitle(&detail)
            .trail(Trail::None)],
    );
    y = row.bottom;
    if let Some((title, detail, share)) = task(s) {
        y = widgets::progress_card(&mut c, t, f, y + m.dp(12.0), &title, &detail, share);
    }
    let green = crate::theme::Theme {
        pal: crate::theme::Palette {
            accent: Rgba::rgb(29, 205, 58),
            ..t.pal
        },
        ..*t
    };
    for d in &s.browser.downloads {
        if let BrowserSave::Downloading { done, total } = d.state {
            let detail = format!(
                "{} / {}",
                crate::fmt::file_size(done),
                total.map(crate::fmt::file_size).unwrap_or_default()
            );
            y = widgets::progress_card(
                &mut c,
                &green,
                f,
                y + m.dp(12.0),
                &d.name,
                &detail,
                total.filter(|n| *n > 0).map(|n| done as f32 / n as f32),
            );
        }
    }
    let mut frame = Frame::new(
        t,
        c.crop_height((y + m.dp(24.0)).max(m.h as f32) as u32),
        out,
        l.pick(["控制中心", "控制中心", "Control center"]),
        0.0,
    );
    frame.header = false;
    frame.back = false;
    frame.bg = background;
    frame
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
