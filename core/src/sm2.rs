use chrono::{Duration, NaiveDate};

/// Quality of recall:
/// 1: Again (failed recall)
/// 3: Hard (difficult recall)
/// 4: Good (correct recall)
/// 5: Easy (effortless recall)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Again = 1,
    Hard = 3,
    Good = 4,
    Easy = 5,
}

impl Quality {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Quality::Again),
            2 | 3 => Some(Quality::Hard), // maps rating 2 to Hard as well
            4 => Some(Quality::Good),
            5 => Some(Quality::Easy),
            _ => None,
        }
    }
}

/// Computes the next SM-2 spaced repetition state given current values and response quality.
pub fn calculate_sm2(
    current_ease: f32,
    current_interval: u32,
    current_reps: u32,
    quality: Quality,
    today: NaiveDate,
) -> (f32, u32, u32, NaiveDate) {
    let q = quality as u8 as f32;

    let (next_reps, next_interval) = if (quality as u8) < 3 {
        (0, 1)
    } else {
        let interval = match current_reps {
            0 => 1,
            1 => 6,
            _ => ((current_interval as f32) * current_ease).round() as u32,
        };
        (current_reps + 1, interval)
    };

    // ease = max(1.3, ease + 0.1 - (5 - q) * (0.08 + (5 - q) * 0.02))
    let diff = 5.0 - q;
    let delta = 0.1 - diff * (0.08 + diff * 0.02);
    let next_ease = (current_ease + delta).max(1.3);

    let next_due = today + Duration::days(next_interval as i64);

    (next_ease, next_interval, next_reps, next_due)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_card_rated_good() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let (ease, interval, reps, due) = calculate_sm2(2.5, 0, 0, Quality::Good, today);
        // q = 4: diff = 1.0, delta = 0.1 - 1*(0.08 + 0.02) = 0.0 -> ease = 2.5
        assert!((ease - 2.5).abs() < 1e-4);
        assert_eq!(reps, 1);
        assert_eq!(interval, 1);
        assert_eq!(due, today + Duration::days(1));

        // Second good rating
        let (ease2, interval2, reps2, due2) =
            calculate_sm2(ease, interval, reps, Quality::Good, due);
        assert_eq!(reps2, 2);
        assert_eq!(interval2, 6);
        assert_eq!(due2, due + Duration::days(6));
        assert!((ease2 - 2.5).abs() < 1e-4);
    }

    #[test]
    fn test_failing_card_resets_reps_and_interval() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        // Suppose card had 5 reps and 30 day interval
        let (ease, interval, reps, due) = calculate_sm2(2.5, 30, 5, Quality::Again, today);
        assert_eq!(reps, 0);
        assert_eq!(interval, 1);
        assert_eq!(due, today + Duration::days(1));
        // Ease drops when failing
        assert!(ease < 2.5);
    }

    #[test]
    fn test_ease_never_drops_below_1_3() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut ease = 1.4;
        let mut interval = 1;
        let mut reps = 1;
        for _ in 0..10 {
            let (e, i, r, _) = calculate_sm2(ease, interval, reps, Quality::Again, today);
            ease = e;
            interval = i;
            reps = r;
        }
        assert_eq!(ease, 1.3);
    }
}
