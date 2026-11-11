use super::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeditationData {
    pub markdown: Option<String>,
    pub duration_minutes: u32,
    pub mood_before: Option<u8>,
    pub mood_after: Option<u8>,
}

impl MeditationData {
    pub fn new(duration_minutes: u32) -> Self {
        Self {
            markdown: None,
            duration_minutes,
            mood_before: None,
            mood_after: None,
        }
    }

    pub fn mood_delta(&self) -> Option<i8> {
        let before = self.mood_before?;
        let after = self.mood_after?;
        Some(after as i8 - before as i8)
    }
}

impl Echo {
    pub fn new_meditation(day: NaiveDate, title: String, duration_minutes: u32) -> Self {
        Echo::new(
            day,
            title,
            EchoContent::MeditationEcho(MeditationData::new(duration_minutes)),
        )
    }

    pub fn as_meditation(&self) -> Option<&MeditationData> {
        match &self.content {
            EchoContent::MeditationEcho(data) => Some(data),
            _ => None,
        }
    }

    pub fn as_meditation_mut(&mut self) -> Option<&mut MeditationData> {
        match &mut self.content {
            EchoContent::MeditationEcho(data) => Some(data),
            _ => None,
        }
    }
}

pub fn total_meditation_minutes(echoes: &[Echo]) -> u32 {
    echoes
        .iter()
        .filter_map(|echo| echo.as_meditation())
        .map(|meditation| meditation.duration_minutes)
        .sum()
}

pub fn average_mood_delta(echoes: &[Echo]) -> Option<f32> {
    let deltas: Vec<i8> = echoes
        .iter()
        .filter_map(|echo| echo.as_meditation())
        .filter_map(|meditation| meditation.mood_delta())
        .collect();

    if deltas.is_empty() {
        return None;
    }

    let sum: i32 = deltas.iter().map(|&delta| delta as i32).sum();
    Some(sum as f32 / deltas.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, NaiveDate};

    #[test]
    fn test_meditation_echo() {
        let echo = Echo::new(
            Local::now().date_naive(),
            "Morning Sit".to_string(),
            EchoContent::MeditationEcho(MeditationData {
                markdown: Some("Felt calm.".to_string()),
                duration_minutes: 20,
                mood_before: Some(5),
                mood_after: Some(8),
            }),
        );

        assert_eq!(echo.content_type_name(), "Meditation Echo");
        assert_eq!(echo.char_count(), 10);
        println!("{}", echo);
    }

    #[test]
    fn test_meditation_new_defaults() {
        let meditation = MeditationData::new(20);
        assert_eq!(meditation.duration_minutes, 20);
        assert!(meditation.markdown.is_none());
        assert!(meditation.mood_before.is_none());
        assert!(meditation.mood_after.is_none());
        assert!(meditation.mood_delta().is_none());
    }

    #[test]
    fn test_meditation_mood_delta() {
        let mut meditation = MeditationData::new(15);

        meditation.mood_before = Some(4);
        meditation.mood_after = Some(8);
        assert_eq!(meditation.mood_delta(), Some(4));

        meditation.mood_before = Some(8);
        meditation.mood_after = Some(3);
        assert_eq!(meditation.mood_delta(), Some(-5));

        meditation.mood_after = None;
        assert_eq!(meditation.mood_delta(), None);
    }

    #[test]
    fn test_new_meditation_echo() {
        let echo = Echo::new_meditation(
            NaiveDate::from_ymd_opt(2026, 6, 4).unwrap(),
            "Evening Sit".to_string(),
            30,
        );
        assert_eq!(echo.content_type_name(), "Meditation Echo");

        let meditation = echo.as_meditation().unwrap();
        assert_eq!(meditation.duration_minutes, 30);
        assert!(meditation.mood_before.is_none());
    }

    #[test]
    fn test_as_meditation_on_non_meditation() {
        let echo = Echo::new(
            Local::now().date_naive(),
            "Plain".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "hi".to_string(),
            }),
        );
        assert!(echo.as_meditation().is_none());
    }

    #[test]
    fn test_meditation_query_helpers() {
        let day = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

        let mut m1 = Echo::new_meditation(day, "Morning".to_string(), 20);
        {
            let data = m1.as_meditation_mut().unwrap();
            data.mood_before = Some(4);
            data.mood_after = Some(8);
        }

        let mut m2 = Echo::new_meditation(day, "Noon".to_string(), 10);
        {
            let data = m2.as_meditation_mut().unwrap();
            data.mood_before = Some(6);
            data.mood_after = Some(4);
        }

        let m3 = Echo::new_meditation(day, "No moods".to_string(), 15);

        let plain = Echo::new(
            day,
            "Note".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "x".to_string(),
            }),
        );

        let all = vec![m1, m2, m3, plain];

        assert_eq!(total_meditation_minutes(&all), 45);
        assert_eq!(average_mood_delta(&all), Some(1.0));

        let no_moods = vec![Echo::new_meditation(day, "solo".to_string(), 5)];
        assert_eq!(average_mood_delta(&no_moods), None);
    }
}
