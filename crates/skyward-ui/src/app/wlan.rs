use super::*;

/// A password typed, which what is printed of it leaves out: the recovery
/// prints each effect the pages ask for.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret(pub String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret({} characters)", self.0.chars().count())
    }
}

/// A network being added by hand, as its page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Adding {
    pub name: String,
    pub security: Security,
    /// What the keyboard types into.
    pub field: Field,
    /// The protections to choose from are open over the page.
    pub choosing: bool,
}

/// A text field of the page that adds a network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Password,
}

/// How a network added by hand can be protected, in the order the stock
/// list gives them: those the recovery joins.
pub const ADDABLE: [Security; 3] = [Security::Open, Security::Psk, Security::Sae];

/// The longest name a network can have, in bytes.
pub(super) const SSID_MAX: usize = 32;

/// What turns down a password the protection `security` cannot take.
pub(super) fn refused(l: Lang, security: Security, password: &str) -> Option<&'static str> {
    match security {
        Security::Psk | Security::PskSae if !(8..=63).contains(&password.len()) => Some(l.pick([
            "请输入 8 到 63 位密码",
            "請輸入 8 到 63 個字元的密碼",
            "Enter a password of 8 to 63 characters",
        ])),
        _ if password.is_empty() => Some(l.pick(["请输入密码", "請輸入密碼", "Enter a password"])),
        _ => None,
    }
}

impl App {
    /// How the WLAN stands, as the runtime last told it.
    pub fn set_wlan(&mut self, w: Wlan) {
        if self.state.wlan.as_ref() == Some(&w) {
            return;
        }
        self.state.wlan = Some(w);
        if matches!(
            self.screen(),
            Screen::Home | Screen::Wlan | Screen::WlanNetwork | Screen::Update | Screen::SkywardUpdate
        ) {
            self.refresh();
        }
        self.dirty = true;
    }
}

impl App {
    /// How fast data went over the network joined in the last second, which
    /// only the status bar shows.
    pub fn set_traffic(&mut self, t: Traffic) {
        if self.state.traffic != t {
            self.state.traffic = t;
            self.dirty = true;
        }
    }
}

impl App {
    /// What the status bar shows of the WLAN: the network joined, while it
    /// has an address.
    pub fn wlan_mark(&self) -> Option<WlanMark> {
        let Some(Joined {
            stage: Joining::Connected(link),
            ..
        }) = self.state.wlan.as_ref()?.joined.as_ref()
        else {
            return None;
        };
        let t = self.state.traffic;
        let (speed, unit) = fmt::speed(t.rx + t.tx);
        Some(WlanMark {
            level: fmt::signal_level(link.signal),
            receiving: t.rx > 0,
            sending: t.tx > 0,
            speed,
            unit,
        })
    }
}

impl App {
    /// Open the password page for `network`, empty.
    pub(super) fn ask_password(&mut self, network: Network) {
        self.state.joining = Some(network);
        self.state.password.clear();
        self.state.show_password = false;
        self.state.layer = Layer::Letters;
        self.open(Screen::WlanPassword);
    }
}
