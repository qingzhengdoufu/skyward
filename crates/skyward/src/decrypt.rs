//! Opening the data partition and its users' own files on a thread of its
//! own, so the screen keeps up: the keys come from the secure world, and
//! what a user enters at their screen lock is checked slowly on purpose.

use anyhow::Result;
use skyward_binder::driver::Binder;
use skyward_crypto::data::{self, Closed, Opening};
use skyward_crypto::keymint::KeyMint;
use skyward_sys::worker::Worker;
use std::os::fd::RawFd;

pub enum Job {
    /// Open the partition as far as it opens with nothing a user knows.
    Open,
    /// Open the own files of `user` with what they entered at their screen
    /// lock; `closed` are the users whose own files are still closed.
    Unlock {
        closed: Vec<Closed>,
        user: u32,
        credential: Vec<u8>,
    },
}

/// How a job ended: what it opened, or why it did not.
pub enum Ended {
    Opened(Result<Opening, String>),
    Unlocked(Result<Vec<Closed>, UnlockError>),
}

#[derive(Debug)]
pub struct UnlockError {
    pub rejection: Option<skyward_crypto::credential::Rejection>,
    pub details: String,
}

impl UnlockError {
    fn of(error: anyhow::Error) -> Self {
        Self {
            rejection: skyward_crypto::credential::rejection(&error),
            details: format!("{error:#}"),
        }
    }
}

/// The whole chain, as the log has always written it. The page shows a
/// sentence of its own instead, so this is only what the log says.
impl std::fmt::Display for UnlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.details)
    }
}

pub struct Decryptor {
    /// Tells how the job ended, once.
    worker: Worker<Ended>,
}

impl Decryptor {
    pub fn start(job: Job) -> Result<Decryptor> {
        let worker = Worker::start("decrypt", move |tell| {
            tell.tell(match job {
                Job::Open => Ended::Opened(with_data_keys(data::open_without_credential)),
                Job::Unlock { closed, user, credential } => Ended::Unlocked(
                    work_with_key_service(|service| data::unlock(service, &closed, user, &credential)).map_err(UnlockError::of),
                ),
            });
        })?;
        Ok(Decryptor { worker })
    }

    /// Readable once the job has ended.
    pub fn fd(&self) -> RawFd {
        self.worker.fd()
    }

    /// How the job ended, once it has.
    pub fn ended(&self) -> Option<Ended> {
        self.worker.events().pop()
    }
}

fn work_with_key_service<T>(work: impl FnOnce(&KeyMint) -> Result<T>) -> Result<T> {
    let binder = Binder::open()?;
    let service = KeyMint::open(&binder)?;
    work(&service)
}

/// Do `work` with what reaching the data partition takes: the phone's key
/// service, reached afresh for it, where the phone's own table has the
/// partition encrypted, and nothing where it has not, so that a partition
/// kept as the plain filesystem it is opens on a phone whose key service
/// does not start. Says why it failed in words for the page.
pub fn with_data_keys<T>(work: impl FnOnce(Option<&KeyMint>) -> Result<T>) -> Result<T, String> {
    let done = (|| {
        if !data::keyed()? {
            return work(None);
        }
        let binder = Binder::open()?;
        let service = KeyMint::open(&binder)?;
        work(Some(&service))
    })();
    done.map_err(|e| format!("{e:#}"))
}
