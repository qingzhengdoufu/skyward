use std::time::Duration;

pub(super) const TICKS_STRENGTH: f32 = 0.2;

/// The stock driver reads ID and writes seconds and milliseconds in place.
pub(super) fn packet(id: i16) -> [i16; 3] {
    [id, 0, 0]
}

pub(super) fn length(packet: [i16; 3]) -> Result<Duration, &'static str> {
    let [_, seconds, millis] = packet;
    // Only short clicks are intended here. Miro effect 0 reports 22 ms.
    if seconds != 0 || !(1..=40).contains(&millis) {
        return Err("the Qualcomm driver returned an unexpected click duration");
    }
    Ok(Duration::from_millis(millis as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_driver_packet_is_six_bytes_with_id_first() {
        let request = packet(5);
        assert_eq!(request, [5, 0, 0]);
        assert_eq!(std::mem::size_of_val(&request), 6);
    }

    #[test]
    fn a_click_uses_the_returned_duration() {
        assert_eq!(length([0, 0, 22]), Ok(Duration::from_millis(22)));
        assert_eq!(length([0, 0, 40]), Ok(Duration::from_millis(40)));
        for invalid in [[0, 0, 0], [0, 0, -1], [0, -1, 22], [0, 1, 22], [0, 0, 41]] {
            assert!(length(invalid).is_err());
        }
    }

    #[test]
    fn screen_off_feedback_uses_the_preferred_tested_share() {
        assert_eq!(TICKS_STRENGTH, 0.2);
    }
}
