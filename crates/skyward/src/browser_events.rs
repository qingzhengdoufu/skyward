//! A stopped worker must not stay blocked behind its last progress messages.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{SyncSender, TrySendError},
};

pub fn send<E>(sender: &SyncSender<E>, mut event: E, stop: &AtomicBool) -> bool {
    loop {
        match sender.try_send(event) {
            Ok(()) => return true,
            Err(TrySendError::Disconnected(_)) => return false,
            Err(TrySendError::Full(held)) => event = held,
        }
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_releases_a_sender_waiting_on_a_full_queue() {
        let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
        sender.send(1).unwrap();
        assert!(!send(&sender, 2, &AtomicBool::new(true)));
    }
    #[test]
    fn pending_events_are_delivered_and_a_closed_receiver_does_not_block() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        assert!(send(&sender, 1, &AtomicBool::new(false)));
        assert_eq!(receiver.recv().unwrap(), 1);
        drop(receiver);
        assert!(!send(&sender, 2, &AtomicBool::new(false)));
    }
}
