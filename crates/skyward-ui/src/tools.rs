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

pub fn root_install(cx: &Ctx, kernelsu: bool) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = canvas(cx, t.m.h * 2);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = if kernelsu {
        "KernelSU"
    } else {
        l.pick(["安装模块", "安裝模組", "Install module"])
    };
    let detail = if kernelsu {
        format!("{} · android15-6.6", cx.info.kernel)
    } else {
        s.module_package.clone().unwrap_or_default()
    };
    let mut y = widgets::info_card(&mut c, t, f, top, &[(l.pick(["目标", "目標", "Target"]), detail)]);
    if let Some(stage) = s.root_stage {
        let text = l.pick(match stage {
            skyward_root::Stage::Downloading => ["正在下载 6.6 安装资源", "正在下載 6.6 安裝資源", "Downloading 6.6 resources"],
            skyward_root::Stage::BackingUp => ["正在备份 init_boot", "正在備份 init_boot", "Backing up init_boot"],
            skyward_root::Stage::Patching => ["正在修补", "正在修補", "Patching"],
            skyward_root::Stage::Writing => ["正在写入并校验", "正在寫入並驗證", "Writing and verifying"],
            skyward_root::Stage::Done => ["完成", "完成", "Finished"],
        });
        y = widgets::progress_card(&mut c, t, f, y, title, text, None);
    } else {
        if let Some(result) = &s.root_result {
            let text = match result {
                Ok(text) => text,
                Err(text) => text,
            };
            y = widgets::footnote(&mut c, t, f, y, text);
        }
        let available = s.data.storage_open()
            && !s.system_detached
            && (!kernelsu || skyward_root::compatible(&cx.info.kernel))
            && (kernelsu || s.module_package.is_some());
        let note = if kernelsu {
            l.pick([
                "下载到 data，仅写入当前槽位 init_boot。原镜像保存至 Skyward/root。",
                "下載至 data，僅寫入目前槽位 init_boot。原映像檔儲存至 Skyward/root。",
                "Resources go to data. Current-slot init_boot only; original image in Skyward/root.",
            ])
        } else {
            l.pick([
                "由已安装的 Root 管理器执行模块脚本，重启系统后生效。",
                "由已安裝的 Root 管理器執行模組指令稿，重新啟動系統後生效。",
                "The installed Root manager runs the module scripts. Applied after Android restarts.",
            ])
        };
        y = widgets::footnote(&mut c, t, f, y, note);
        if available {
            let which = if kernelsu { Confirm::KernelSu } else { Confirm::RootModuleInstall };
            let (label, danger) = crate::pages::confirm_label(which, s);
            let rect = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
            out.push(Hit {
                rect,
                action: Action::Confirm(which),
            });
            y = rect.y + rect.h;
        } else {
            y = widgets::footnote(
                &mut c,
                t,
                f,
                y,
                l.pick([
                    "请先解密 data；KernelSU 需要确认 6.6 KMI。",
                    "請先解密 data；KernelSU 需要確認 6.6 KMI。",
                    "Decrypt data first. KernelSU requires a confirmed 6.6 KMI.",
                ]),
            );
        }
    }
    Frame::new(t, c.crop_height((y + t.m.dp(24.0)) as u32), out, title, top)
}

pub fn tasks(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let tiles = task_tiles(t, s.task_cards.len());
    let height = tiles.last().map_or(t.m.h, |r| (r.y + r.h + t.m.dp(24.0)).ceil() as u32).max(t.m.h);
    let mut c = canvas(cx, height);
    if let Some(background) = &s.task_background {
        draw_picture(
            &mut c,
            background,
            crate::geom::Rect::new(0.0, 0.0, t.m.w as f32, height as f32),
            0.0,
        );
        c.shade_rows(0, height, |_, _| t.pal.bg.with_alpha(if t.dark { 180 } else { 205 }));
    }
    let mut out = Vec::new();
    for (index, (task, r)) in s.task_cards.iter().zip(tiles).enumerate() {
        let preview = task_preview_rect(t, r);
        let radius = t.m.card_radius;
        c.fill_round_rect(preview.x, preview.y, preview.w, preview.h, radius, t.pal.card);
        if let Some(picture) = &task.preview {
            draw_picture(&mut c, picture, preview, radius);
        }
        let (color, icon) = task_icon(task.screen);
        c.fill_circle(r.x + t.m.dp(13.0), r.y + t.m.dp(16.0), t.m.dp(12.0), color);
        crate::icons::glyph(
            &mut c,
            icon,
            r.x + t.m.dp(13.0),
            r.y + t.m.dp(16.0),
            t.m.dp(17.0),
            Rgba::rgb(255, 255, 255),
        );
        c.text_vc(
            &f.heading,
            task.title,
            r.x + t.m.dp(33.0),
            r.y + t.m.dp(16.0),
            t.m.dp(16.0),
            t.pal.text,
            Some(r.w - t.m.dp(69.0)),
        );
        out.push(Hit {
            rect: r,
            action: Action::ResumeTaskAt(index),
        });
        let close = crate::geom::Rect::new(r.x + r.w - t.m.dp(32.0), r.y, t.m.dp(32.0), t.m.dp(40.0));
        let (x, y) = (close.centre().0, r.y + t.m.dp(16.0));
        let size = t.m.dp(4.0);
        c.line(x - size, y - size, x + size, y + size, t.m.dp(1.7), t.pal.subtitle);
        c.line(x + size, y - size, x - size, y + size, t.m.dp(1.7), t.pal.subtitle);
        out.push(Hit {
            rect: close,
            action: Action::CloseTask(task.screen),
        });
    }
    if s.task_cards.is_empty() {
        widgets::footnote(
            &mut c,
            t,
            f,
            t.m.content_top,
            l.pick(["暂无其他任务", "暫無其他工作", "No recent tasks"]),
        );
    }
    let mut dock = Canvas::new(t.m.w, (t.m.dp(76.0) + t.bottom_inset).ceil() as u32);
    let (x, y, radius) = (t.m.w as f32 / 2.0, t.m.dp(34.0), t.m.dp(25.0));
    dock.fill_circle(x, y, radius, t.pal.round.with_alpha(235));
    dock.line(
        x - t.m.dp(7.0),
        y - t.m.dp(7.0),
        x + t.m.dp(7.0),
        y + t.m.dp(7.0),
        t.m.dp(2.4),
        t.pal.text,
    );
    dock.line(
        x + t.m.dp(7.0),
        y - t.m.dp(7.0),
        x - t.m.dp(7.0),
        y + t.m.dp(7.0),
        t.m.dp(2.4),
        t.pal.text,
    );
    Frame {
        header: false,
        back: false,
        dock: Some(Dock {
            canvas: dock,
            hits: vec![Hit {
                rect: crate::geom::Rect::new(x - radius, y - radius, radius * 2.0, radius * 2.0),
                action: Action::ClearTasks,
            }],
        }),
        ..Frame::new(t, c, out, "", t.m.dp(96.0))
    }
}

fn task_icon(screen: Screen) -> (Rgba, Icon) {
    match screen {
        Screen::Browser => (Rgba::rgb(0, 160, 160), Icon::Globe),
        Screen::Files => (Rgba::rgb(235, 171, 45), Icon::Folder),
        Screen::Install => (Rgba::rgb(29, 180, 58), Icon::Install),
        Screen::RootModules => (Rgba::rgb(35, 140, 200), Icon::Shield),
        Screen::Backup => (Rgba::rgb(65, 130, 240), Icon::Backup),
        Screen::Restore => (Rgba::rgb(65, 130, 240), Icon::Restore),
        Screen::Terminal => (Rgba::rgb(88, 98, 110), Icon::Terminal),
        _ => (Rgba::rgb(110, 120, 135), Icon::Gear),
    }
}

pub(crate) fn task_chrome(out: &mut Canvas, t: &crate::Theme, state: &State, rows: std::ops::Range<u32>) {
    let rows = rows.start.min(out.h)..rows.end.min(out.h);
    let mut band = Canvas::band(out.w, out.h, rows.clone(), 0..out.w, t.pal.bg);
    if let Some(background) = &state.task_background {
        draw_picture(
            &mut band,
            background,
            crate::geom::Rect::new(0.0, 0.0, out.w as f32, out.h as f32),
            0.0,
        );
        band.shade_rows(rows.start, rows.end, |_, _| t.pal.bg.with_alpha(if t.dark { 180 } else { 205 }));
    }
    out.put_rows(&band, rows.start as i32, rows.end as i32, rows.start as i32);
}

/// Scaled previews are drawn inside their card, so bands and page transitions clip them too.
fn draw_picture(c: &mut Canvas, picture: &BrowserPicture, r: crate::geom::Rect, radius: f32) {
    let (w, h) = (picture.width as usize, picture.height as usize);
    if w == 0 || h == 0 || picture.pixels.len() != w * h * 4 {
        return;
    }
    let held = c.held();
    let cols = c.cols();
    let (first, last) = (
        (r.y.floor().max(held.start as f32)) as u32,
        (r.y + r.h).ceil().min(held.end as f32).max(0.0) as u32,
    );
    let (left, right) = (
        (r.x.floor().max(cols.start as f32)) as u32,
        (r.x + r.w).ceil().min(cols.end as f32).max(0.0) as u32,
    );
    for y in first..last {
        let sy = ((y as f32 - r.y + 0.5) / r.h * h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
        let iy = sy as usize;
        let fy = sy.fract();
        for x in left..right {
            let qx = (x as f32 + 0.5 - r.x - r.w / 2.0).abs() - (r.w / 2.0 - radius);
            let qy = (y as f32 + 0.5 - r.y - r.h / 2.0).abs() - (r.h / 2.0 - radius);
            let cov = (0.5 - (qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius)).clamp(0.0, 1.0);
            if cov == 0.0 {
                continue;
            }
            let sx = ((x as f32 - r.x + 0.5) / r.w * w as f32 - 0.5).clamp(0.0, (w - 1) as f32);
            let ix = sx as usize;
            let fx = sx.fract();
            let pixel = |px: usize, py: usize, k: usize| picture.pixels[(py * w + px) * 4 + k] as f32;
            let channel = |k| {
                let top = pixel(ix, iy, k) * (1.0 - fx) + pixel((ix + 1).min(w - 1), iy, k) * fx;
                let bottom = pixel(ix, (iy + 1).min(h - 1), k) * (1.0 - fx) + pixel((ix + 1).min(w - 1), (iy + 1).min(h - 1), k) * fx;
                (top * (1.0 - fy) + bottom * fy).round() as u8
            };
            c.blend(x as i32, y as i32, Rgba::rgb(channel(0), channel(1), channel(2)), cov);
        }
    }
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
        &[
            Row::new("KernelSU").icon(Rgba::rgb(35, 140, 200), Icon::Shield),
            Row::new(l.pick(["安装模块 ZIP", "安裝模組 ZIP", "Install module ZIP"])).icon(Rgba::rgb(29, 205, 58), Icon::Install),
            Row::new(l.pick(crate::lang::legacy(["刷新模块", "重新整理模組", "Refresh modules"])))
                .icon(Rgba::rgb(52, 130, 255), Icon::Restore),
        ],
    );
    out.extend(hits(
        &read.rows,
        &[Action::Open(Screen::KernelSu), Action::PickRootModule, Action::ReadRootModules],
    ));
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
                    crate::lang::legacy(["重启后移除", "重新啟動後移除", "Removal pending"])
                } else if module.enabled {
                    crate::lang::legacy(["已启用", "已啟用", "Enabled"])
                } else {
                    crate::lang::legacy(["已禁用", "已停用", "Disabled"])
                });
                let sub = format!("{} | {} | {}", module.version, module.author, status);
                let rows = [
                    Row::new(&module.name).subtitle(&sub).trail(Trail::Switch(module.enabled)),
                    Row::new(l.pick(if module.removing {
                        crate::lang::legacy(["取消移除", "取消移除", "Cancel removal"])
                    } else {
                        crate::lang::legacy(["移除模块", "移除模組", "Remove module"])
                    }))
                    .icon(Rgba::rgb(220, 65, 65), Icon::Wipe),
                ];
                let list = widgets::list(&mut c, t, f, y, &rows);
                out.extend(hits(&list.rows, &[Action::RootModule(i, 0), Action::RootModule(i, 1)]));
                y = list.bottom + t.m.card_gap;
            }
            if modules.is_empty() {
                y = widgets::footnote(
                    &mut c,
                    t,
                    f,
                    y,
                    l.pick(crate::lang::legacy(["未发现模块", "未發現模組", "No modules found"])),
                );
            }
        }
        Some(Err(why)) => y = widgets::footnote(&mut c, t, f, y, why),
        None => y = widgets::footnote(&mut c, t, f, y, l.pick(crate::lang::legacy(["正在读取", "正在讀取", "Reading"]))),
    }
    Frame::new(
        t,
        c.crop_height((y + t.m.dp(24.0)) as u32),
        out,
        l.pick(crate::lang::legacy(["Root 模块", "Root 模組", "Root modules"])),
        top,
    )
}

pub fn partitions(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let top = t.m.content_top;
    let mut c = canvas(cx, t.m.h * 3);
    let mut out = Vec::new();
    let rows = [
        Row::new(l.pick(crate::lang::legacy(["刷入分区镜像", "刷入分割區映像檔", "Flash partition image"])))
            .icon(Rgba::rgb(29, 205, 58), Icon::Install),
        Row::new(l.pick(crate::lang::legacy([
            "导出单分区镜像",
            "匯出單分割區映像檔",
            "Export partition image",
        ])))
        .icon(Rgba::rgb(52, 130, 255), Icon::Backup),
        Row::new(l.pick(crate::lang::legacy(["文件系统管理", "檔案系統管理", "Filesystems"]))).icon(Rgba::rgb(140, 157, 176), Icon::Mount),
        Row::new(l.pick(crate::lang::legacy(["挂载管理", "掛載管理", "Mounts"]))).icon(Rgba::rgb(119, 103, 249), Icon::Layers),
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
                    l.pick(crate::lang::legacy([
                        "共享分区，包含双槽逻辑分区",
                        "共用分割區，包含雙槽邏輯分割區",
                        "Shared; contains both slots",
                    ]))
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
        l.pick(crate::lang::legacy(["分区管理", "分割區管理", "Partitions"])),
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
    c.text_top(&f.body, &s.control_clock, x, m.dp(45.0), m.dp(40.0), t.pal.text, Some(width));
    c.text_top(
        &f.body,
        &s.control_date,
        x,
        m.dp(94.0),
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
        m.dp(115.0),
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
            l.pick(crate::lang::legacy(["网络", "網路", "Network"])),
            Icon::Wlan,
            false,
            s.wlan.is_some(),
            Action::Open(Screen::Wlan),
        );
    }
    let top = m.dp(144.0);
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
        &[Row::new(l.pick(crate::lang::legacy(["设备状态", "裝置狀態", "Device status"])))
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
        l.pick(crate::lang::legacy(["控制中心", "控制中心", "Control center"])),
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
            s.lang
                .pick(crate::lang::legacy(["验证签名", "驗證簽章", "Verifying signature"]))
                .into(),
            Some(*done as f32 / (*total).max(1) as f32),
        )),
        _ => None,
    };
    if let Some((detail, share)) = progress {
        return Some((
            s.lang.pick(crate::lang::legacy(["安装进度", "安裝進度", "Installation"])).into(),
            detail,
            share,
        ));
    }
    if matches!(s.install, Install::Scripting { running: true, .. }) {
        return Some((
            s.lang
                .pick(crate::lang::legacy([
                    "正在运行刷包脚本",
                    "正在執行安裝指令稿",
                    "Install script running",
                ]))
                .into(),
            s.script_output.last().cloned().unwrap_or_default(),
            None,
        ));
    }
    if let Some(job) = &s.download
        && job.outcome.is_none()
    {
        return Some((
            s.lang.pick(crate::lang::legacy(["系统下载", "系統下載", "System download"])).into(),
            format!("{} B/s", job.rate),
            job.progress.as_ref().and_then(|p| match p {
                skyward_net::download::Progress::Downloading { done, total } => Some(*done as f32 / (*total).max(1) as f32),
                _ => None,
            }),
        ));
    }
    None
}
