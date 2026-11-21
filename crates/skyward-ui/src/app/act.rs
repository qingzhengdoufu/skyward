use super::*;

impl App {
    /// Carry out a tapped control.
    pub(super) fn act(&mut self, action: Action, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        // A background event may replace the field while a key is held.
        if matches!(action, Action::Type(_)) && !self.text_context() {
            return Vec::new();
        }
        if matches!(action, Action::ImeToggle | Action::ImePick(_) | Action::ImePage(_)) {
            return self.ime_action(action);
        }
        if let Action::Type(cap) = action
            && let Some(effects) = self.ime_key(cap, now)
        {
            return effects;
        }
        if Self::is_browser_action(action) {
            return self.browser_act(action, now);
        }
        match action {
            Action::NavHome => {
                self.home();
                return Vec::new();
            }
            Action::NavTasks => {
                if self.shade.is_some() {
                    self.dismiss_shade();
                }
                if self.screen() != Screen::Home && self.screen() != Screen::Tasks {
                    self.parked = Some(self.stack.clone());
                }
                if self.screen() != Screen::Tasks {
                    self.open(Screen::Tasks);
                }
                return Vec::new();
            }
            Action::ResumeTask => {
                self.resume();
                return Vec::new();
            }
            Action::NavBack => {
                if self.browser_immersive() {
                    return self.set_browser_immersive(false);
                }
                if self.screen() == Screen::Browser && self.state.browser.at > 0 {
                    return self.browser_act(Action::BrowserBack, now);
                }
                return self.back(now);
            }
            Action::FlashPartitionImage => {
                self.state.image_only = true;
                let place = if self.state.data.storage_open() {
                    Place::Storage
                } else if self.state.usb_drive == Some(UsbDrive::Shown) {
                    Place::UsbDrive
                } else {
                    self.open(Screen::Decrypt);
                    return self.arrive_at(Screen::Decrypt);
                };
                self.open(Screen::Choose(place));
                return self.arrive_at(Screen::Choose(place));
            }
            Action::OpenControlCenter => {
                let effects = if self.browser_immersive() {
                    self.set_browser_immersive(false)
                } else {
                    Vec::new()
                };
                self.open_shade(now, true);
                return effects;
            }
            Action::CloseControlCenter => {
                self.settle_shade(false, now, 0.0);
                return Vec::new();
            }
            Action::ReadRootModules => return vec![Effect::ReadRootModules],
            Action::RootModule(index, op) => {
                if let Some(Ok((_, modules))) = &self.state.root_modules
                    && let Some(module) = modules.get(index)
                {
                    let (marker, present) = match op {
                        0 => ("disable", module.enabled),
                        1 => ("remove", !module.removing),
                        _ => return Vec::new(),
                    };
                    return vec![Effect::RootModule {
                        path: module.path.clone(),
                        marker: marker.into(),
                        present,
                    }];
                }
            }
            Action::Open(s) => {
                if s == Screen::Browser && cfg!(feature = "aera-browser") {
                    return self.browser_enter();
                }
                if s == Screen::Install {
                    self.state.image_only = false;
                }
                if s == Screen::Search && self.screen() == Screen::Home {
                    if let Some((rest, _)) = self.seek.take() {
                        self.top_mut().1 = rest;
                    }
                    self.start_morph(true, now);
                    self.push(s);
                } else {
                    self.open(s);
                }
                return self.arrive_at(s);
            }
            Action::OpenData => return self.open_data(),
            Action::OpenUsbManager => match &self.state.usb_drive {
                Some(UsbDrive::Shown) => {
                    self.state.files = Folder::new(USB_DRIVE);
                    self.open(Screen::Files);
                    return vec![Effect::ReadDir(USB_DRIVE.into())];
                }
                Some(UsbDrive::Unreadable(_)) => self.say(l.pick(USB_DRIVE_UNREADABLE), now),
                None => self.say(l.pick(USB_DRIVE_WANTED), now),
            },
            Action::UnlockUser(user) => {
                if let Some((_, keys)) = self.state.others.iter().find(|(u, _)| *u == user).copied() {
                    self.state.data = Data::Locked {
                        user,
                        keys,
                        checking: false,
                    };
                    self.state.code.clear();
                    self.state.ime.clear();
                    self.state.ime.enabled = false;
                    self.refresh();
                }
            }
            Action::Skip => {
                self.leave(true);
                if self.stack.len() > 1 {
                    self.stack.pop();
                } else {
                    *self.top_mut() = (Screen::Home, 0.0, Focus::Unset);
                }
                self.settle_page();
                self.refresh();
            }
            Action::Back | Action::CloseSearch => return self.back(now),
            Action::Theme { dark } => {
                if self.state.dark != dark {
                    self.state.dark = dark;
                    self.refresh();
                }
            }
            Action::Toggle(Switch::Wlan) => {
                let Some(w) = self.state.wlan.as_mut() else {
                    return Vec::new();
                };
                let on = matches!(w.power, Power::Off | Power::Failed(_));
                *w = Wlan {
                    power: if on { Power::Starting } else { Power::Off },
                    ..Wlan::default()
                };
                self.refresh();
                return vec![Effect::Wlan(on)];
            }
            Action::Toggle(switch) => {
                let on = match switch {
                    Switch::Vibrate => &mut self.state.vibrate,
                    Switch::FileTransfer => &mut self.state.file_transfer,
                    Switch::Compress => &mut self.state.compress,
                    Switch::Verify => &mut self.state.verify,
                    Switch::Signature => &mut self.state.check_signature,
                    Switch::KeepRecovery => &mut self.state.keep_recovery,
                    Switch::DisableAvb => &mut self.state.disable_avb,
                    Switch::RememberPassword => &mut self.state.remember_password,
                    Switch::ShowPassword => &mut self.state.show_password,
                    Switch::AlwaysOn => &mut self.state.always_on,
                    Switch::DoubleTap => &mut self.state.double_tap,
                    Switch::RememberWlan => &mut self.state.remember_wlan,
                    Switch::Mirror => &mut self.state.mirror,
                    Switch::Wlan => unreachable!("the WLAN switch is turned above"),
                };
                *on = !*on;
                let now_on = *on;
                if switch == Switch::DoubleTap {
                    self.tapped = None;
                }
                if switch == Switch::RememberPassword && !now_on {
                    self.state.remembered = None;
                }
                self.refresh();
                if switch == Switch::FileTransfer {
                    return vec![Effect::FileTransfer(now_on)];
                }
                if switch == Switch::Mirror {
                    return vec![Effect::SetMirror(now_on)];
                }
                if switch == Switch::RememberPassword && !now_on {
                    return vec![Effect::RememberPassword(None)];
                }
                if switch == Switch::RememberWlan && !now_on {
                    return vec![Effect::ForgetWlan(String::new())];
                }
            }
            Action::MountToggle(i) => {
                if self.state.mount_failure.take().is_some() {
                    self.refresh();
                }
                let row = &self.state.mounts.as_ref().expect("a partition toggled is one the page lists")[i];
                let base = row.base.clone();
                return vec![if row.mounted { Effect::Unmount(base) } else { Effect::Mount(base) }];
            }
            Action::RebootTarget(i) => {
                self.state.reboot = i;
                self.refresh();
            }
            Action::WipeItem(item) => {
                let k = WIPES.iter().position(|&w| w == item).expect("the page offers only what it lists");
                let on = !self.state.wipe[k];
                match (on, item) {
                    (true, Wipe::All) => {
                        self.state.wipe = [false; WIPES.len()];
                        self.state.wipe[k] = true;
                    }
                    (true, _) => {
                        self.state.wipe[0] = false;
                        self.state.wipe[k] = true;
                    }
                    (false, _) => self.state.wipe[k] = false,
                }
                self.refresh();
            }
            Action::AvbToggle(check) => {
                if let Some(Ok(state)) = self.state.avb.clone()
                    && state.unlocked
                {
                    self.state.avb_pending = match &self.state.avb_pending {
                        Some((_, pending, _)) if *pending == check => None,
                        _ => Some((state.clone(), check, !state.disabled(check))),
                    };
                    self.state.avb_notice = None;
                    self.state.confirm = 0.0;
                    self.refresh();
                }
            }
            Action::CancelAvb => {
                self.state.avb_pending = None;
                self.state.avb_confirming = false;
                self.state.confirm = 0.0;
                self.refresh();
            }
            Action::Confirm(_) => self.say(
                l.pick(crate::texts::toast::SLIDE_RIGHT),
                now,
            ),
            Action::Slider(_) | Action::Pattern => {}
            Action::Sideload => match self.state.install {
                Install::Idle | Install::Finished { .. } => {
                    self.begin_install(Install::Starting, Source::Computer, None);
                    if self.screen() == Screen::Installing {
                        self.refresh();
                    } else {
                        self.open(Screen::Installing);
                    }
                    return vec![Effect::Sideload];
                }
                _ => self.say(l.pick(INSTALL_UNFINISHED), now),
            },
            Action::FlashPartition(k) => {
                if let Install::Choosing(choice) = &mut self.state.install {
                    choice.partition = k;
                    self.refresh();
                }
            }
            Action::FlashSlots(pick) => {
                if let Install::Choosing(choice) = &mut self.state.install {
                    choice.pick = pick;
                    self.refresh();
                }
            }
            Action::SystemSize(k) => {
                if let Install::SystemChoosing(choice) = &mut self.state.install {
                    choice.size = k;
                    self.refresh();
                }
            }
            Action::PickSystem(who) => {
                if self.state.systems_work.is_none() {
                    self.state.system_pick = Some(who);
                    self.state.asking_delete = false;
                    self.state.confirm = 0.0;
                    self.refresh();
                }
            }
            Action::RebootTo(target) => return vec![Effect::Reboot(target)],
            Action::EnterRecovery => return vec![Effect::EnterRecovery],
            Action::Dismiss => {
                self.stack = vec![(Screen::Home, 0.0, Focus::Unset)];
                self.settle_page();
                self.refresh();
                return vec![Effect::Dismiss];
            }
            Action::PickEntry(op) => {
                let Some(path) = self.entry_path() else { return self.no_entry(now) };
                self.state.clip = Some(Clip { op, path });
                return self.back(now);
            }
            Action::Paste => {
                let Some(clip) = self.state.clip.clone() else {
                    self.say(l.pick(crate::lang::legacy(["没有选中的条目", "沒有選取的項目", "Nothing is picked to paste"])), now);
                    return Vec::new();
                };
                let (path, folder) = (std::path::Path::new(&clip.path), std::path::Path::new(&self.state.files.path));
                let name = path
                    .file_name()
                    .map_or_else(|| clip.path.clone(), |n| n.to_string_lossy().into_owned());
                if folder.starts_with(path) {
                    self.say(
                        &tr!(
                            l,
                            "无法将\"{name}\"放入其自身中",
                            "無法將「{name}」放入其本身中",
                            "“{name}” can't go inside itself"
                        ),
                        now,
                    );
                } else if clip.op == FileOp::Move && path.parent() == Some(folder) {
                    self.say(
                        &tr!(
                            l,
                            "\"{name}\"已在该文件夹中",
                            "「{name}」已在這個資料夾中",
                            "“{name}” is already in this folder"
                        ),
                        now,
                    );
                } else {
                    return vec![Effect::Paste {
                        op: clip.op,
                        path: clip.path,
                        into: self.state.files.path.clone(),
                        clash: None,
                    }];
                }
            }
            Action::Unpick => {
                self.state.clip = None;
                self.refresh();
            }
            Action::RenameEntry => {
                let Some(name) = self.state.file.as_ref().map(|f| f.name.clone()) else {
                    return self.no_entry(now);
                };
                self.state.new_name = name;
                self.state.layer = Layer::Letters;
                self.open(Screen::Rename);
            }
            Action::EditEntry => {
                return match self.entry_path() {
                    Some(path) => vec![Effect::OpenText(path)],
                    None => self.no_entry(now),
                };
            }
            Action::InstallEntry => {
                let (Some(size), Some(path)) = (self.state.file.as_ref().map(|f| f.size), self.entry_path()) else {
                    return self.no_entry(now);
                };
                return self.install_opened(path, size, now);
            }
            Action::SaveText => {
                let ed = self.state.editor.as_ref().expect("the editor page shows a file");
                return vec![Effect::SaveText {
                    path: ed.path.clone(),
                    text: ed.text(),
                }];
            }
            Action::OpenPermissions => self.open(Screen::Permissions),
            Action::ModeBit(bit) => {
                let (Some(mode), Some(path)) = (self.state.file.as_ref().map(|f| (f.mode ^ bit) & 0o7777), self.entry_path()) else {
                    return self.no_entry(now);
                };
                return vec![Effect::SetMode { path, mode }];
            }
            Action::AskDelete => {
                self.state.asking_delete = true;
                self.state.confirm = 0.0;
                self.refresh();
            }
            Action::Resolve(choice) => {
                let Some((clip, into, _)) = self.state.clash.take() else {
                    self.say(
                        l.pick(crate::lang::legacy(["没有要处理的同名条目", "沒有要處理的同名項目", "There is no clash left to answer"])),
                        now,
                    );
                    return Vec::new();
                };
                self.leave(true);
                self.stack.pop();
                self.settle_page();
                self.refresh();
                if choice == ClashChoice::Skip {
                    self.state.clip = None;
                    return Vec::new();
                }
                return vec![Effect::Paste {
                    op: clip.op,
                    path: clip.path,
                    into,
                    clash: Some(choice),
                }];
            }
            Action::StopJob => {
                let job = self.state.job.as_mut().expect("only work under way is stopped");
                if job.outcome.is_none() && !job.stopping {
                    job.stopping = true;
                    self.refresh();
                    return vec![Effect::StopJob];
                }
            }
            Action::StopRelabel => {
                let job = self.state.relabel.as_mut().expect("only work under way is stopped");
                if job.outcome.is_none() && !job.stopping {
                    job.stopping = true;
                    self.refresh();
                    return vec![Effect::StopRelabel];
                }
            }
            Action::OpenFilesystem(k) => {
                self.state.fs_chosen = k;
                self.state.fs_action = FsAction::Repair;
                self.open(Screen::Filesystem);
            }
            Action::ChooseFs(choice) => {
                let row = self.fs_row().expect("the page of a partition shows one that was read");
                let action = match choice {
                    FsChoice::Repair => FsAction::Repair,
                    FsChoice::Remake(k) => FsAction::Remake(row.others[k].clone()),
                };
                self.state.fs_action = action;
                self.refresh();
            }
            Action::ScreenOff(k) => {
                self.state.screen_off = k;
                self.refresh();
            }
            Action::TimeZone(k) => {
                self.state.time_zone = k;
                self.refresh();
            }
            Action::Language(l) => {
                self.state.lang = l;
                self.state.language_chosen = true;
                self.refresh();
            }
            Action::BackupPart(k) => {
                if k == 3 && !self.state.efs_backup_available {
                    self.say(
                        l.pick(crate::lang::legacy([
                            "本机 EFS 分区清单尚未验证",
                            "本機 EFS 分割區清單尚未驗證",
                            "This device's EFS partition list is not verified",
                        ])),
                        now,
                    );
                    return Vec::new();
                }
                self.state.backup[k] = !self.state.backup[k];
                self.refresh();
            }
            Action::BackupTo(place) => {
                self.state.backup_to = place;
                self.refresh();
            }
            Action::ImagePartition(index) => {
                if self
                    .state
                    .image_partitions
                    .as_ref()
                    .and_then(|parts| parts.as_ref().ok())
                    .is_some_and(|parts| index < parts.len())
                {
                    self.state.image_partition = Some(index);
                    self.state.confirm = 0.0;
                    self.refresh();
                }
            }
            Action::OpenBackup(i) => {
                let Some(entry) = self.state.backups.get(i).cloned() else {
                    return Vec::new();
                };
                let parts = entry.held.as_ref().map_or([false; 4], |h| h.parts);
                self.state.restore = Some((entry, parts));
                self.state.asking_delete = false;
                self.open(Screen::RestoreBackup);
            }
            Action::RestorePart(k) => {
                let (_, parts) = self.state.restore.as_mut().expect("the restore page shows a backup");
                parts[k] = !parts[k];
                self.refresh();
            }
            Action::StopBackup => {
                let job = self.state.backup_job.as_mut().expect("only work under way is stopped");
                if job.outcome.is_none() && !job.stopping {
                    job.stopping = true;
                    self.refresh();
                    return vec![Effect::StopBackup];
                }
            }
            Action::LogPart(k) => {
                self.state.logs[k] = !self.state.logs[k];
                self.refresh();
            }
            Action::LogsTo(k) => {
                self.state.logs_to = k;
                self.refresh();
            }
            Action::ExportLogs => {
                let ([recovery, kernel], storage) = (self.state.logs, self.state.logs_to == 1);
                if !recovery && !kernel {
                    self.say(
                        l.pick(crate::texts::toast::CHOOSE_LOGS),
                        now,
                    );
                } else if storage && !self.state.data.storage_open() {
                    self.say(l.pick(DECRYPT_FIRST), now);
                } else {
                    return vec![Effect::ExportLogs { recovery, kernel, storage }];
                }
            }
            Action::PickSlot(slot) => {
                self.state.next_slot = Some(slot);
                self.refresh();
            }
            Action::Type(cap) if self.screen() == Screen::Terminal => {
                return self.term_typed(cap);
            }
            Action::Type(cap) if self.screen() == Screen::Editor => self.edit_typed(cap),
            Action::Type(cap) => return self.type_key(cap, now),
            Action::TermKey(key) if self.screen() == Screen::Editor => {
                self.state.ime.clear();
                let ed = self.state.editor.as_mut().expect("the editor page shows a file");
                match key {
                    TermKey::Tab => ed.insert('\t'),
                    TermKey::Left => ed.left(),
                    TermKey::Up => ed.up(),
                    TermKey::Down => ed.down(),
                    TermKey::Right => ed.right(),
                    TermKey::Esc | TermKey::Ctrl => {
                        unreachable!("the editor's row has no {key:?}")
                    }
                }
                self.edits += 1;
                self.refresh_text();
            }
            Action::TermKey(key) => {
                let Some(term) = self.term.as_mut() else { return Vec::new() };
                let bytes = term.key(key);
                let running = term.running;
                self.refresh();
                if running && !bytes.is_empty() {
                    return vec![Effect::TermInput(bytes)];
                }
            }
            Action::ClearSearch => {
                self.state.ime.clear();
                self.state.search.clear();
                self.edits += 1;
                self.refresh();
            }
            Action::OpenEntry(i) => {
                let Some(entry) = self.state.files.entry(i).cloned() else {
                    return Vec::new();
                };
                match entry.kind {
                    FileKind::Folder { .. } | FileKind::Link { folder: true, .. } => {
                        let scroll = self.top().1;
                        self.state.files.leave(scroll);
                        return vec![Effect::ReadDir(self.state.files.inside(&entry.name))];
                    }
                    FileKind::File if matches!(files::file_type(&entry.name), FileType::Text | FileType::Code) => {
                        return vec![Effect::OpenText(self.state.files.inside(&entry.name))];
                    }
                    _ if installs(&entry) => {
                        let path = self.state.files.inside(&entry.name);
                        return self.install_opened(path, entry.size, now);
                    }
                    FileKind::File | FileKind::Link { folder: false, .. } | FileKind::Special => {
                        self.state.file_path = None;
                        self.state.file = Some(entry);
                        self.open(Screen::FileInfo);
                    }
                }
            }
            Action::OpenFound(i) => {
                let Some(found) = self
                    .state
                    .found
                    .as_ref()
                    .and_then(|f| f.as_ref().ok())
                    .and_then(|list| list.get(i))
                    .cloned()
                else {
                    return Vec::new();
                };
                let path = found_path(&found);
                match found.entry.kind {
                    FileKind::Folder { .. } | FileKind::Link { folder: true, .. } => {
                        let scroll = self.top().1;
                        self.state.files.leave(scroll);
                        self.state.files_query.clear();
                        self.state.files_search = false;
                        self.clear_found();
                        return vec![Effect::ReadDir(found.folder)];
                    }
                    FileKind::File if matches!(files::file_type(&found.entry.name), FileType::Text | FileType::Code) => {
                        return vec![Effect::OpenText(path)];
                    }
                    _ if installs(&found.entry) => {
                        return self.install_opened(path, found.entry.size, now);
                    }
                    FileKind::File | FileKind::Link { folder: false, .. } | FileKind::Special => {
                        self.state.file_path = Some(path);
                        self.state.file = Some(found.entry);
                        self.open(Screen::FileInfo);
                    }
                }
            }
            Action::TerminalHere => {
                let folder = self.state.files.path.clone();
                let running = self.term.as_ref().is_some_and(|term| term.running);
                self.open(Screen::Terminal);
                self.state.layer = Layer::Letters;
                if running {
                    return vec![Effect::ShellTo(folder)];
                }
                self.shell_folder = folder.clone();
                return vec![Effect::StartShell(folder)];
            }
            Action::ChooseEntry(i) => {
                let screen = self.screen();
                let Screen::Choose(place) = screen else {
                    unreachable!("only a place's folder is chosen in")
                };
                let folder = self.state.folder(screen);
                let Some(entry) = folder.entry(i).cloned() else { return Vec::new() };
                let path = folder.inside(&entry.name);
                match entry.kind {
                    FileKind::Folder { .. } | FileKind::Link { folder: true, .. } => {
                        let scroll = self.top().1;
                        self.state.folder_mut(screen).leave(scroll);
                        return vec![Effect::ReadPlace(place, path)];
                    }
                    FileKind::File | FileKind::Link { folder: false, .. } => {
                        return self.install_file(place, path, entry.size, now);
                    }
                    FileKind::Special => unreachable!("a place lists no special files"),
                }
            }
            Action::OpenAncestor(depth) => {
                let screen = self.screen();
                if let Some(path) = self.state.folder(screen).ancestor(depth) {
                    return vec![match screen {
                        Screen::Choose(place) => Effect::ReadPlace(place, path),
                        _ => Effect::ReadDir(path),
                    }];
                }
            }
            Action::JoinNetwork(i) => {
                let w = self.state.wlan.as_ref().expect("networks show on a phone whose WLAN is brought up");
                let Some(network) = w.networks.get(i).cloned() else {
                    return Vec::new();
                };
                if w.joined.as_ref().is_some_and(|j| j.ssid == network.ssid) {
                    return self.act(Action::OpenJoined, now);
                }
                match network.security {
                    Security::Wep => self.say(
                        l.pick(crate::texts::toast::WEP_UNSUPPORTED),
                        now,
                    ),
                    Security::Enterprise => self.say(
                        l.pick(crate::texts::toast::ENTERPRISE_UNSUPPORTED),
                        now,
                    ),
                    s if s.password() && network.remembered => {
                        return vec![Effect::JoinRememberedWlan {
                            ssid: network.ssid,
                            name: network.name,
                            security: s,
                        }];
                    }
                    s if s.password() => self.ask_password(network),
                    _ => {
                        return vec![Effect::JoinWlan {
                            ssid: network.ssid,
                            name: network.name,
                            security: network.security,
                            password: Secret::default(),
                        }];
                    }
                }
            }
            Action::NetworkDetails(i) => {
                let w = self.state.wlan.as_ref().expect("networks show on a phone whose WLAN is brought up");
                let Some(network) = w.networks.get(i).cloned() else {
                    return Vec::new();
                };
                self.state.details = Some(network);
                self.open(Screen::WlanNetwork);
            }
            Action::OpenJoined => {
                let w = self
                    .state
                    .wlan
                    .as_ref()
                    .expect("a network is chosen on a phone whose WLAN is brought up");
                let Some(j) = w.joined.clone() else { return Vec::new() };
                let found = w.networks.iter().find(|n| n.ssid == j.ssid).cloned();
                let network = found.unwrap_or(Network {
                    ssid: j.ssid,
                    name: j.name,
                    signal: 0,
                    frequency: 0,
                    bands: Default::default(),
                    security: j.security,
                    remembered: false,
                });
                if j.stage == Joining::WrongPassword {
                    self.ask_password(network);
                } else {
                    self.state.details = Some(network);
                    self.open(Screen::WlanNetwork);
                }
            }
            Action::LeaveNetwork => {
                let mut fx = self.back(now);
                fx.push(Effect::LeaveWlan);
                return fx;
            }
            Action::Rescan => return vec![Effect::ScanWlan],
            Action::AddNetwork => {
                self.state.adding = Some(Adding {
                    name: String::new(),
                    security: Security::Open,
                    field: Field::Name,
                    choosing: false,
                });
                self.state.password.clear();
                self.state.show_password = false;
                self.state.layer = Layer::Letters;
                self.open(Screen::WlanAdd);
            }
            Action::TypeInto(field) => {
                let a = self.state.adding.as_mut().expect("the fields are on the page that adds a network");
                if a.field != field {
                    self.state.ime.clear();
                    self.state.ime.enabled = false;
                    a.field = field;
                    self.refresh();
                }
            }
            Action::ChooseSecurity => {
                self.state
                    .adding
                    .as_mut()
                    .expect("the protection is on the page that adds a network")
                    .choosing = true;
                self.row_press = None;
                self.row_art = None;
                self.refresh();
            }
            Action::PickSecurity(security) => {
                let a = self
                    .state
                    .adding
                    .as_mut()
                    .expect("the protections open on the page that adds a network");
                a.security = security;
                a.choosing = false;
                if !security.password() {
                    a.field = Field::Name;
                }
                self.refresh();
            }
            Action::CloseChoices => {
                self.state
                    .adding
                    .as_mut()
                    .expect("the protections open on the page that adds a network")
                    .choosing = false;
                self.refresh();
            }
            Action::CheckUpdate => {
                if self.state.downloading() {
                    return Vec::new();
                }
                if !self.state.online() {
                    self.open(Screen::Wlan);
                    return self.arrive_at(Screen::Wlan);
                }
                if self.state.check != Check::Asking {
                    self.state.check = Check::Asking;
                    self.refresh();
                    return vec![Effect::CheckUpdate];
                }
            }
            Action::Download => {
                let release = self.state.newest().expect("the download is offered for the newest system").clone();
                if !self.state.data.storage_open() {
                    self.open(Screen::Decrypt);
                    return self.arrive_at(Screen::Decrypt);
                }
                if !self.state.online() {
                    self.open(Screen::Wlan);
                    return self.arrive_at(Screen::Wlan);
                }
                if !self.state.downloading() {
                    self.state.download = Some(DownloadJob {
                        progress: None,
                        rate: 0,
                        outcome: None,
                        stopping: false,
                    });
                    self.refresh();
                    return vec![Effect::Download(release)];
                }
            }
            Action::InstallDownloaded => {
                let name = self
                    .state
                    .newest()
                    .expect("the package installed is the newest system's")
                    .name
                    .clone();
                let Have::Whole(size) = self.state.have else {
                    unreachable!("only a whole package is offered to install")
                };
                return self.install_file(Place::Storage, format!("{DOWNLOADS}/{name}"), size, now);
            }
            Action::CheckSkyward => {
                if !self.state.online() {
                    self.open(Screen::Wlan);
                    return self.arrive_at(Screen::Wlan);
                }
                if self.state.skyward_check != SkywardCheck::Asking {
                    self.state.skyward_check = SkywardCheck::Asking;
                    self.refresh();
                    return vec![Effect::CheckSkyward];
                }
            }
            Action::UpdateSkyward => {
                let build = self
                    .state
                    .skyward_newest()
                    .and_then(|n| n.build.clone())
                    .expect("the update is offered for the newest release's build");
                if !self.state.online() {
                    self.open(Screen::Wlan);
                    return self.arrive_at(Screen::Wlan);
                }
                if !matches!(self.state.install, Install::Idle | Install::Finished { .. }) {
                    self.say(l.pick(INSTALL_UNFINISHED), now);
                    return Vec::new();
                }
                if !self.state.updating_skyward() {
                    self.state.skyward_job = Some(SkywardJob {
                        stage: None,
                        rate: 0,
                        outcome: None,
                        stopping: false,
                    });
                    self.refresh();
                    return vec![Effect::UpdateSkyward(build)];
                }
            }
            Action::StopSkywardUpdate => {
                let job = self.state.skyward_job.as_mut().expect("only an update under way is stopped");
                if job.outcome.is_none() && !job.stopping && !matches!(job.stage, Some(SkywardStage::Write { .. })) {
                    job.stopping = true;
                    self.refresh();
                    return vec![Effect::StopSkywardUpdate];
                }
            }
            Action::Unavailable(text) => self.say(text, now),
            Action::SortFiles(key) => {
                if self.state.sort != key {
                    self.state.sort = key;
                    self.refresh();
                    return vec![Effect::ReadDir(self.state.files.path.clone())];
                }
            }
            Action::CycleSort => {
                let at = SORT_KEYS.iter().position(|key| *key == self.state.sort).unwrap_or(0);
                let next = SORT_KEYS[(at + 1) % SORT_KEYS.len()];
                self.state.sort = next;
                self.refresh();
                if !self.state.files_query.trim().is_empty() {
                    return self.find_files();
                }
                return vec![Effect::ReadDir(self.state.files.path.clone())];
            }
            Action::FilesSearch => {
                if !self.state.files_search {
                    self.state.ime.clear();
                    self.state.files_search = true;
                    self.refresh();
                }
            }
            Action::CloseFilesSearch => {
                if self.state.files_search {
                    self.state.ime.clear();
                    self.state.files_search = false;
                    self.refresh();
                }
            }
            Action::ClearFilesQuery => {
                self.state.ime.clear();
                if !self.state.files_query.is_empty() {
                    self.state.files_query.clear();
                    self.state.files.keep("");
                    self.clear_found();
                    self.edits += 1;
                    self.refresh();
                }
            }
            Action::AvbConfirmOpen => {
                if matches!(self.state.avb_pending, Some((_, _, true))) {
                    self.state.avb_confirming = true;
                    self.state.confirm = 0.0;
                    self.refresh();
                } else {
                    self.say(
                        l.pick(crate::lang::legacy(["没有要关闭的校验", "沒有要關閉的驗證", "There is no check to turn off"])),
                        now,
                    );
                }
            }
            Action::VbmetaBackup(place) => {
                self.state.vbmeta_notice = None;
                return vec![Effect::VbmetaBackup { place }];
            }
            Action::VbmetaRestore(i) => {
                let found = match &self.state.vbmeta_copies {
                    Some(Ok(copies)) => copies.get(i).cloned(),
                    _ => None,
                };
                match found {
                    Some(copy) => {
                        self.state.vbmeta_pending = Some(copy);
                        self.state.vbmeta_notice = None;
                        self.state.confirm = 0.0;
                        self.refresh();
                    }
                    None => self.say(l.pick(crate::lang::legacy(["没有这个备份", "沒有這個備份", "That copy is not here"])), now),
                }
            }
            Action::FindVbmeta => {
                self.state.vbmeta_copies = None;
                self.refresh();
                return vec![Effect::FindVbmeta {
                    storage: self.state.data.storage_open(),
                    usb: self.state.usb_drive.is_some(),
                }];
            }
            Action::ForgetNetwork => {
                if self.state.details.as_ref().is_some_and(|network| network.remembered) {
                    self.state.asking_delete = true;
                    self.state.confirm = 0.0;
                    self.refresh();
                }
            }
            Action::PauseDownload => {
                if self.state.download.as_ref().is_some_and(|j| j.outcome.is_none() && !j.stopping) {
                    if let Some(job) = self.state.download.as_mut() {
                        job.stopping = true;
                    }
                    self.refresh();
                    return vec![Effect::PauseDownload];
                }
            }
            Action::AskDeleteDownload
                if matches!(self.state.have, Have::Whole(_)) && self.state.download.as_ref().is_none_or(|j| j.outcome.is_some()) =>
            {
                self.state.asking_delete = true;
                self.state.confirm = 0.0;
                self.refresh();
            }
            _ => {}
        }
        Vec::new()
    }
}

impl App {
    /// The text the keyboard types into on this page.
    pub(super) fn typing(&mut self) -> &mut String {
        match self.screen() {
            Screen::BrowserAddress => &mut self.state.browser.address,
            Screen::BrowserForm => {
                let (form, field) = self.state.browser.field.expect("form input");
                &mut self.state.browser.form[form][field]
            }
            Screen::Browser if self.state.browser.field.is_some() => {
                let (form, field) = self.state.browser.field.expect("inline form input");
                &mut self.state.browser.form[form][field]
            }
            Screen::Search => &mut self.state.search,
            Screen::Files => &mut self.state.files_query,
            Screen::Decrypt => &mut self.state.code,
            Screen::Rename => &mut self.state.new_name,
            Screen::WlanPassword => &mut self.state.password,
            Screen::WlanAdd => {
                let a = self.state.adding.as_mut().expect("the page adds a network");
                match a.field {
                    Field::Name => &mut a.name,
                    Field::Password => &mut self.state.password,
                }
            }
            s => panic!("{s:?} has no text field"),
        }
    }
}

impl App {
    /// A key of the on-screen keyboard. A capital lasts one letter; return
    /// opens the first result of a search, and runs a terminal's command.
    pub(super) fn type_key(&mut self, cap: Cap, now: u64) -> Vec<Effect> {
        if self.screen() == Screen::BrowserAddress && matches!(cap, Cap::Char(_) | Cap::Space | Cap::Backspace) {
            return self.browser_edit_key(cap);
        }
        let (l, layer) = (self.state.lang, self.state.layer);
        match cap {
            Cap::Char(ch) => {
                let shifted = self.state.layer == Layer::Shifted;
                self.typing().push(if shifted { ch.to_ascii_uppercase() } else { ch });
                self.edits += 1;
                if shifted {
                    self.state.layer = Layer::Letters;
                }
            }
            Cap::Space => {
                self.typing().push(' ');
                self.edits += 1;
            }
            Cap::Backspace => {
                self.typing().pop();
                self.edits += 1;
            }
            Cap::Shift => {
                self.state.layer = if self.state.layer == Layer::Shifted {
                    Layer::Letters
                } else {
                    Layer::Shifted
                };
            }
            Cap::Show(layer) => self.state.layer = layer,
            Cap::Enter => match self.screen() {
                Screen::BrowserAddress => {
                    let mut effects = Vec::new();
                    for _ in 0..32 {
                        if self.state.browser.ime.spelling.is_empty() {
                            break;
                        }
                        effects.extend(self.browser_commit_ime(0));
                    }
                    if self.state.browser.full_active && self.state.browser.full_text {
                        self.state.browser.full_text = false;
                        self.state.browser.address.clear();
                        self.open(Screen::Browser);
                        effects.push(Effect::BrowserPluginKey(13));
                        return effects;
                    }
                    let input = self.state.browser.address.clone();
                    self.open(Screen::Browser);
                    effects.extend(self.browser_go(&input, now));
                    return effects;
                }
                Screen::BrowserForm => {
                    let (form, _) = self.state.browser.field.expect("form input");
                    self.state.browser.field = None;
                    self.back(now);
                    return self.act(Action::BrowserFormSubmit(form), now);
                }
                Screen::Search => {
                    if let Some(first) = search::find_on(&self.state.search, l, |screen| self.state.offers(screen)).first() {
                        return self.act(Action::Open(first.screen), now);
                    }
                }
                Screen::Files => return self.act(Action::CloseFilesSearch, now),
                Screen::Decrypt if self.state.code.is_empty() => {
                    self.say(l.pick(crate::texts::toast::ENTER_LOCK), now)
                }
                Screen::Decrypt => {
                    let code = std::mem::take(&mut self.state.code);
                    return self.unlock(code);
                }
                Screen::Rename => {
                    let name = self.state.new_name.clone();
                    let (Some(current), Some(path)) = (self.state.file.as_ref().map(|f| f.name.clone()), self.entry_path()) else {
                        return self.no_entry(now);
                    };
                    let taken = self
                        .state
                        .files
                        .entries
                        .as_ref()
                        .is_ok_and(|entries| entries.iter().any(|e| e.name == name));
                    match name.as_str() {
                        "" => self.say(l.pick(crate::texts::toast::ENTER_NAME), now),
                        "." | ".." => self.say(
                            l.pick(crate::texts::toast::NAME_DOTS),
                            now,
                        ),
                        _ if name.contains('/') => {
                            self.say(l.pick(crate::texts::toast::NAME_SLASH), now)
                        }
                        _ if name == current => return self.back(now),
                        _ if taken => self.say_taken(now),
                        _ => return vec![Effect::Rename { path, name }],
                    }
                }
                Screen::WlanPassword => {
                    let network = self.state.joining.clone().expect("the password page is for a network");
                    match refused(l, network.security, &self.state.password) {
                        Some(why) => self.say(why, now),
                        None => {
                            let password = Secret(std::mem::take(&mut self.state.password));
                            let mut fx = self.back(now);
                            fx.push(Effect::JoinWlan {
                                ssid: network.ssid,
                                name: network.name,
                                security: network.security,
                                password,
                            });
                            return fx;
                        }
                    }
                }
                Screen::WlanAdd => {
                    let a = self.state.adding.as_mut().expect("the page adds a network");
                    let (field, why) = match a.name.len() {
                        0 => (
                            Field::Name,
                            Some(l.pick(crate::texts::toast::ENTER_NETWORK_NAME).to_string()),
                        ),
                        n if n > SSID_MAX => (
                            Field::Name,
                            Some(tr!(
                                l,
                                "网络名称不能超过 {SSID_MAX} 个字符",
                                "網路名稱不能超過 {SSID_MAX} 個字元",
                                "A network's name can't be over {SSID_MAX} characters"
                            )),
                        ),
                        _ if a.security.password() => (Field::Password, refused(l, a.security, &self.state.password).map(str::to_string)),
                        _ => (Field::Name, None),
                    };
                    match why {
                        Some(why) => {
                            if a.field != field {
                                a.field = field;
                                self.refresh();
                            }
                            self.say(&why, now)
                        }
                        None => {
                            let (name, security) = (a.name.clone(), a.security);
                            let password = match security.password() {
                                true => Secret(std::mem::take(&mut self.state.password)),
                                false => Secret::default(),
                            };
                            let mut fx = self.back(now);
                            fx.push(Effect::JoinWlan {
                                ssid: name.clone().into_bytes(),
                                name,
                                security,
                                password,
                            });
                            return fx;
                        }
                    }
                }
                other => unreachable!("{other:?} shows no keyboard"),
            },
        }
        if self.screen() == Screen::Files {
            if matches!(cap, Cap::Char(_) | Cap::Space | Cap::Backspace) {
                return self.find_files();
            }
            self.refresh();
        } else if matches!(cap, Cap::Char(_) | Cap::Space | Cap::Backspace) && self.state.layer == layer {
            let t = self.theme();
            self.refresh_rows(0, (t.m.h as f32 - widgets::keyboard_height(&t)).floor() as u32);
        } else {
            self.refresh();
        }
        Vec::new()
    }
}

impl App {
    /// What a swipe-to-confirm control slid to its end does.
    pub(super) fn confirmed(&mut self, which: Confirm, now: u64) -> Vec<Effect> {
        let (l, mut fx) = (self.state.lang, Vec::new());
        match which {
            Confirm::SkywardUpdate => {
                let slot = self.state.skyward_job.as_ref().and_then(|job| match job.outcome {
                    Some(SkywardEnd::Ready(slot)) => Some(slot),
                    _ => None,
                });
                if let (Some(slot), Some(build)) = (slot, self.state.skyward_newest().and_then(|newest| newest.build.clone())) {
                    self.state.skyward_job = Some(SkywardJob {
                        stage: Some(SkywardStage::Write {
                            done: 0,
                            total: build.size * 2,
                        }),
                        rate: 0,
                        outcome: None,
                        stopping: false,
                    });
                    self.refresh();
                    fx.push(Effect::WriteSkyward { build, slot });
                }
            }
            Confirm::Reboot => fx.push(Effect::Reboot(TARGETS[self.state.reboot])),
            Confirm::Wipe => {
                let chosen: Vec<Wipe> = WIPES.iter().zip(self.state.wipe).filter(|(_, on)| *on).map(|(&w, _)| w).collect();
                match chosen.is_empty() {
                    true => self.say(l.pick(texts::toast::CHOOSE_WIPE), now),
                    false => fx.push(Effect::Wipe(chosen)),
                }
            }
            Confirm::Install | Confirm::InstallWipe => {
                if let Install::Confirming { version, .. } = &self.state.install {
                    self.state.install = Install::Updating {
                        version: version.clone(),
                        stage: None,
                        overwriting: false,
                    };
                    fx.push(Effect::Install);
                    self.refresh();
                }
            }
            Confirm::RunScript => {
                if self.state.install == Install::ConfirmingScript {
                    self.state.install = Install::Scripting {
                        stage: None,
                        running: false,
                    };
                    fx.push(Effect::Install);
                    self.refresh();
                }
            }
            Confirm::Backup => {
                let s = &self.state;
                let refused = if s.backup[3] && !s.efs_backup_available {
                    Some(crate::lang::legacy([
                        "本机 EFS 备份不可用，请取消勾选",
                        "本機 EFS 備份無法使用，請取消勾選",
                        "EFS backup is unavailable; deselect it",
                    ]))
                } else if !s.backup.contains(&true) {
                    Some(crate::texts::toast::CHOOSE_BACKUP)
                } else if s.backup[2] && s.data != Data::Open {
                    Some(crate::texts::toast::DECRYPT_TO_BACK_UP)
                } else if s.place_open(s.backup_to) {
                    None
                } else {
                    Some(match (s.backup_to, &s.usb_drive) {
                        (Place::Storage, _) if s.data == Data::Erased => DATA_ERASED,
                        (Place::Storage, _) => DECRYPT_FIRST,
                        (Place::UsbDrive, Some(UsbDrive::Unreadable(_))) => USB_DRIVE_UNREADABLE,
                        (Place::UsbDrive, _) => USB_DRIVE_WANTED,
                    })
                };
                let backup = Effect::Backup {
                    parts: s.backup,
                    place: s.backup_to,
                    compress: s.compress,
                    verify: s.verify,
                };
                match refused {
                    Some(why) => self.say(l.pick(why), now),
                    None => fx.push(backup),
                }
            }
            Confirm::ImageBackup => {
                if !self.state.place_open(self.state.backup_to) {
                    self.say(
                        l.pick(crate::lang::legacy([
                            "备份位置不可用，请先解密或连接 U 盘",
                            "備份位置無法使用，請先解密或連接 USB 隨身碟",
                            "Unlock storage or connect a readable USB drive",
                        ])),
                        now,
                    );
                } else if let Some((name, _)) = self
                    .state
                    .image_partitions
                    .as_ref()
                    .and_then(|parts| parts.as_ref().ok())
                    .and_then(|parts| self.state.image_partition.and_then(|index| parts.get(index)))
                {
                    fx.push(Effect::BackupImage {
                        name: name.clone(),
                        place: self.state.backup_to,
                    });
                } else {
                    self.say(l.pick(crate::lang::legacy(["请选择一个分区", "請選擇一個分割區", "Select one partition"])), now);
                }
            }
            Confirm::Restore => {
                let (entry, parts) = self.state.restore.as_ref().expect("the restore page shows a backup");
                if let (Some(image), Ok(held)) = (&entry.image, &entry.held) {
                    if self.state.place_open(entry.place) {
                        fx.push(Effect::RestoreImage {
                            place: entry.place,
                            folder: entry.name.clone(),
                            partition: image.partition.clone(),
                            size: held.size,
                            sha256: image.sha256.clone(),
                        });
                    }
                    return fx;
                }
                let restore = Effect::Restore {
                    place: entry.place,
                    name: entry.name.clone(),
                    parts: *parts,
                };
                if !parts.contains(&true) {
                    self.say(l.pick(texts::toast::CHOOSE_RESTORE), now);
                } else if parts[2] && self.state.data != Data::Open {
                    self.say(
                        l.pick(crate::texts::toast::DECRYPT_TO_RESTORE),
                        now,
                    );
                } else {
                    fx.push(restore);
                }
            }
            Confirm::DeleteBackup => {
                let (entry, _) = self.state.restore.as_ref().expect("the restore page shows a backup");
                if entry.image.is_some() {
                    return vec![Effect::DeleteImageBackup {
                        place: entry.place,
                        name: entry.name.clone(),
                    }];
                }
                fx.push(Effect::DeleteBackup {
                    place: entry.place,
                    name: entry.name.clone(),
                });
            }
            Confirm::Slot => match (&self.state.boot_slots, self.state.next_slot) {
                (Some(Ok(slots)), Some(pick)) if pick != slots.active => fx.push(Effect::SetSlot(pick)),
                (Some(Ok(slots)), _) => {
                    let slot = slots.active.letter();
                    let said = tr!(
                        l,
                        "下次已从槽位 {slot} 启动",
                        "下次已從槽位 {slot} 啟動",
                        "Slot {slot} already starts next"
                    );
                    self.say(&said, now);
                }
                _ => self.say(
                    l.pick(crate::texts::slot::UNREADABLE),
                    now,
                ),
            },
            Confirm::FactoryReset => fx.push(Effect::FactoryReset),
            Confirm::Merge => {
                if self.state.update == Standing::Waiting {
                    self.state.update = Standing::Merging(0.0);
                    self.refresh();
                    fx.push(Effect::Merge);
                }
            }
            Confirm::DeleteEntry => match self.entry_path() {
                Some(path) => fx.push(Effect::Delete(path)),
                None => return self.no_entry(now),
            },
            Confirm::Avb => match self.state.avb_pending.take() {
                Some((expected, check, false)) => fx.push(Effect::SetAvb {
                    expected,
                    check,
                    disabled: false,
                }),
                Some((expected, check, true)) => {
                    self.state.avb_pending = Some((expected, check, true));
                    self.state.avb_confirming = true;
                    self.state.confirm = 0.0;
                    self.refresh();
                }
                None => self.say(
                    l.pick(crate::lang::legacy(["没有待确认的改动", "沒有待確認的變更", "There is no change to confirm"])),
                    now,
                ),
            },
            Confirm::Relabel => match self.state.data {
                Data::Open | Data::Locked { .. } => fx.push(Effect::Relabel),
                Data::Opening => self.say(l.pick(DATA_OPENING), now),
                Data::Erased => self.say(l.pick(DATA_ERASED), now),
                Data::Closed | Data::Failed(_) => self.say(
                    l.pick(crate::texts::toast::OPEN_DATA_FIRST),
                    now,
                ),
            },
            Confirm::Filesystem => {
                let row = self.fs_row().expect("the page of a partition shows one that was read");
                let data = row.name == DATA_PARTITION;
                match (&self.state.fs_action, &self.state.data) {
                    (_, Data::Opening) if data => self.say(l.pick(DATA_OPENING), now),
                    (FsAction::Repair, Data::Erased) if data => self.say(l.pick(DATA_ERASED), now),
                    (action, _) => fx.push(Effect::Filesystem {
                        name: row.name.clone(),
                        action: action.clone(),
                    }),
                }
            }
            Confirm::Flash => {
                if let Install::Choosing(choice) = &self.state.install {
                    let partition = choice.partitions[choice.partition];
                    fx.push(Effect::Flash {
                        partition,
                        pick: choice.pick,
                    });
                    self.state.install = Install::Flashing { partition, stage: None };
                    self.refresh();
                }
            }
            Confirm::SwitchSystem => {
                if let (Some(Ok(systems)), Some(pick)) = (&self.state.systems, self.state.system_pick)
                    && pick != starting(systems)
                {
                    fx.push(Effect::SwitchSystem(pick));
                    self.state.systems_work = Some(SystemsWork::Switching(pick));
                    self.state.system_pick = None;
                    self.refresh();
                }
            }
            Confirm::RemoveSystem => {
                fx.push(Effect::RemoveSystem);
                self.state.systems_work = Some(SystemsWork::Removing);
                self.state.asking_delete = false;
                self.state.system_pick = None;
                self.refresh();
            }
            Confirm::InstallSystem => {
                if let Install::SystemChoosing(choice) = &self.state.install {
                    fx.push(Effect::InstallSystem {
                        size: choice.sizes[choice.size],
                    });
                    self.state.install = Install::Systeming {
                        name: choice.name.clone(),
                        stage: None,
                    };
                    self.refresh();
                }
            }
            Confirm::AvbDisable => match self.state.avb_pending.take() {
                Some((expected, check, true)) => {
                    self.state.avb_confirming = false;
                    fx.push(Effect::SetAvb {
                        expected,
                        check,
                        disabled: true,
                    });
                }
                other => {
                    self.state.avb_pending = other;
                    self.state.avb_confirming = false;
                    self.say(
                        l.pick(crate::lang::legacy(["没有要关闭的校验", "沒有要關閉的驗證", "There is no check to turn off"])),
                        now,
                    );
                }
            },
            Confirm::VbmetaRestore => match self.state.vbmeta_pending.take() {
                Some(copy) => fx.push(Effect::VbmetaRestore {
                    place: copy.place,
                    name: copy.name,
                }),
                None => self.say(
                    l.pick(crate::lang::legacy(["没有选择要写回的备份", "沒有選擇要寫回的備份", "No copy is chosen to write back"])),
                    now,
                ),
            },
            Confirm::ForgetNetwork => {
                let name = self
                    .state
                    .details
                    .as_ref()
                    .filter(|network| self.state.asking_delete && network.remembered)
                    .map(|network| network.name.clone());
                self.state.asking_delete = false;
                self.state.confirm = 0.0;
                match name {
                    Some(name) => fx.push(Effect::ForgetWlan(name)),
                    None => self.say(
                        l.pick(crate::lang::legacy(["没有要忘记的网络", "沒有要忘記的網路", "There is no network to forget"])),
                        now,
                    ),
                }
            }
            Confirm::DeleteDownload => match self.state.have {
                Have::Whole(_) if self.state.download.as_ref().is_none_or(|j| j.outcome.is_some()) => fx.push(Effect::DeleteDownload(
                    self.state.newest().map_or_else(String::new, |r| r.name.clone()),
                )),
                _ => self.say(
                    l.pick(crate::lang::legacy(["没有可删除的安装包", "沒有可刪除的安裝檔", "There is no package to delete"])),
                    now,
                ),
            },
            Confirm::PowerOff => fx.push(Effect::Reboot(Target::PowerOff)),
        }
        fx
    }
}
