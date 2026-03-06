//! Citizen feedback generator — procedural complaints and praise.
//!
//! Each tick, evaluates city metrics and generates citizen opinion messages
//! using template-driven text with seeded random name generation.
//! Messages are rate-limited per game-day and prioritized by severity.

// ---- Enums ----

/// The type of citizen message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Complaint,
    Praise,
    Request,
    Observation,
}

/// Severity level for citizen messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

// ---- CitizenMessage ----

/// A single citizen feedback message.
#[derive(Debug, Clone)]
pub struct CitizenMessage {
    /// Name of the citizen who sent this message.
    pub name: String,
    /// Type of message (complaint, praise, etc.).
    pub message_type: MessageType,
    /// Severity of the message.
    pub severity: Severity,
    /// Full text of the message.
    pub text: String,
    /// Simulation tick when the message was generated.
    pub tick: u64,
}

// ---- CityMetricsSnapshot ----

/// A snapshot of city metrics used to evaluate citizen sentiment.
#[derive(Debug, Clone, Default)]
pub struct CityMetricsSnapshot {
    /// Pollution index (0-1000).
    pub pollution_index: u16,
    /// Crime rate (0-1000).
    pub crime_rate: u16,
    /// Unemployment percentage (0-100).
    pub unemployment_pct: u8,
    /// Average happiness (0-100).
    pub happiness_avg: u8,
    /// Traffic congestion (0-1000).
    pub traffic_congestion: u16,
    /// Power satisfaction percentage (0-100).
    pub power_satisfied_pct: u8,
    /// Water satisfaction percentage (0-100).
    pub water_satisfied_pct: u8,
    /// Total population.
    pub population: u32,
}

// ---- ICitizenVoice trait ----

/// Trait for generating citizen feedback messages from city metrics.
pub trait ICitizenVoice {
    /// Generate citizen messages based on current metrics, tick, and seed.
    fn generate(&self, metrics: &CityMetricsSnapshot, tick: u64, seed: u64) -> Vec<CitizenMessage>;
    /// Maximum number of messages allowed per game-day.
    fn max_messages_per_day(&self) -> usize;
}

// ---- Name generation ----

const FIRST_NAMES: [&str; 20] = [
    "Alex", "Maria", "James", "Chen", "Sofia",
    "Omar", "Yuki", "Elena", "David", "Priya",
    "Marcus", "Fatima", "Liam", "Nina", "Raj",
    "Anna", "Carlos", "Mei", "Thomas", "Zara",
];

const LAST_NAMES: [&str; 20] = [
    "Smith", "Garcia", "Kim", "Johnson", "Patel",
    "Nguyen", "Mueller", "Santos", "Taylor", "Ivanov",
    "Brown", "Silva", "Wilson", "Ahmed", "Lopez",
    "Park", "Robinson", "Chen", "Martinez", "Tanaka",
];

/// Simple hash function for seeded deterministic selection.
fn seed_hash(seed: u64, extra: u64) -> u64 {
    let mut h = seed.wrapping_mul(6364136223846793005).wrapping_add(extra);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    h
}

/// Generate a citizen name from a seed value.
fn generate_name(seed: u64) -> String {
    let first_idx = (seed_hash(seed, 1) % FIRST_NAMES.len() as u64) as usize;
    let last_idx = (seed_hash(seed, 2) % LAST_NAMES.len() as u64) as usize;
    format!("{} {}", FIRST_NAMES[first_idx], LAST_NAMES[last_idx])
}

// ---- Template rules ----

/// Internal struct for a message template rule.
struct TemplateRule {
    /// Condition closure: returns true if this rule should fire.
    condition: fn(&CityMetricsSnapshot) -> bool,
    /// Template text for the message.
    template: &'static str,
    /// Message type.
    message_type: MessageType,
    /// Severity level.
    severity: Severity,
}

/// Get all template rules.
fn template_rules() -> Vec<TemplateRule> {
    vec![
        // Pollution complaints
        TemplateRule {
            condition: |m| m.pollution_index > 800,
            template: "I can barely breathe!",
            message_type: MessageType::Complaint,
            severity: Severity::Critical,
        },
        TemplateRule {
            condition: |m| m.pollution_index > 500 && m.pollution_index <= 800,
            template: "The air quality is terrible!",
            message_type: MessageType::Complaint,
            severity: Severity::High,
        },
        // Crime
        TemplateRule {
            condition: |m| m.crime_rate > 600,
            template: "I don't feel safe in this city",
            message_type: MessageType::Complaint,
            severity: Severity::High,
        },
        TemplateRule {
            condition: |m| m.crime_rate < 100,
            template: "This is the safest city I've lived in!",
            message_type: MessageType::Praise,
            severity: Severity::Low,
        },
        // Unemployment
        TemplateRule {
            condition: |m| m.unemployment_pct > 20,
            template: "Jobs are hard to find",
            message_type: MessageType::Complaint,
            severity: Severity::Medium,
        },
        // Happiness
        TemplateRule {
            condition: |m| m.happiness_avg > 80,
            template: "I love living here!",
            message_type: MessageType::Praise,
            severity: Severity::Low,
        },
        // Traffic
        TemplateRule {
            condition: |m| m.traffic_congestion > 700,
            template: "Traffic is unbearable",
            message_type: MessageType::Complaint,
            severity: Severity::High,
        },
        // Power
        TemplateRule {
            condition: |m| m.power_satisfied_pct < 80,
            template: "Power outages are unacceptable",
            message_type: MessageType::Complaint,
            severity: Severity::High,
        },
        // Water
        TemplateRule {
            condition: |m| m.water_satisfied_pct < 80,
            template: "We need better water service",
            message_type: MessageType::Complaint,
            severity: Severity::Medium,
        },
        // Population observation
        TemplateRule {
            condition: |m| m.population > 50_000,
            template: "The city is really growing!",
            message_type: MessageType::Observation,
            severity: Severity::Low,
        },
    ]
}

// ---- DefaultCitizenVoice ----

/// Default implementation of ICitizenVoice using template rules.
pub struct DefaultCitizenVoice {
    /// Maximum messages generated per game-day.
    pub max_per_day: usize,
}

impl Default for DefaultCitizenVoice {
    fn default() -> Self {
        Self { max_per_day: 5 }
    }
}

impl ICitizenVoice for DefaultCitizenVoice {
    fn generate(&self, metrics: &CityMetricsSnapshot, tick: u64, seed: u64) -> Vec<CitizenMessage> {
        let rules = template_rules();

        // Collect all triggered rules.
        let mut triggered: Vec<&TemplateRule> = rules.iter()
            .filter(|rule| (rule.condition)(metrics))
            .collect();

        // Sort by severity descending (priority queue: highest severity first).
        triggered.sort_by(|a, b| b.severity.cmp(&a.severity));

        // Rate limit: take at most max_per_day messages.
        triggered.truncate(self.max_per_day);

        // Generate messages with seeded names.
        triggered.iter().enumerate().map(|(i, rule)| {
            let name_seed = seed.wrapping_add(tick).wrapping_add(i as u64);
            let name = generate_name(name_seed);
            let text = format!("{} says: \"{}\"", name, rule.template);
            CitizenMessage {
                name,
                message_type: rule.message_type,
                severity: rule.severity,
                text,
                tick,
            }
        }).collect()
    }

    fn max_messages_per_day(&self) -> usize {
        self.max_per_day
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create default metrics (all healthy).
    fn default_metrics() -> CityMetricsSnapshot {
        CityMetricsSnapshot {
            pollution_index: 200,
            crime_rate: 50,
            unemployment_pct: 5,
            happiness_avg: 70,
            traffic_congestion: 200,
            power_satisfied_pct: 95,
            water_satisfied_pct: 95,
            population: 10_000,
        }
    }

    fn voice() -> DefaultCitizenVoice {
        DefaultCitizenVoice::default()
    }

    // ---- Test 1: High pollution generates complaint ----

    #[test]
    fn high_pollution_generates_complaint() {
        let v = voice();
        let mut m = default_metrics();
        m.pollution_index = 600;
        let msgs = v.generate(&m, 100, 42);
        assert!(!msgs.is_empty());
        assert!(msgs.iter().any(|msg| msg.message_type == MessageType::Complaint));
        assert!(msgs.iter().any(|msg| msg.text.contains("air quality")));
    }

    // ---- Test 2: Critical pollution generates critical complaint ----

    #[test]
    fn critical_pollution_generates_critical_complaint() {
        let v = voice();
        let mut m = default_metrics();
        m.pollution_index = 900;
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg|
            msg.message_type == MessageType::Complaint && msg.severity == Severity::Critical
        ));
        assert!(msgs.iter().any(|msg| msg.text.contains("barely breathe")));
    }

    // ---- Test 3: Low crime generates praise ----

    #[test]
    fn low_crime_generates_praise() {
        let v = voice();
        let mut m = default_metrics();
        m.crime_rate = 50; // already low in default
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg| msg.message_type == MessageType::Praise));
        assert!(msgs.iter().any(|msg| msg.text.contains("safest city")));
    }

    // ---- Test 4: High crime generates complaint ----

    #[test]
    fn high_crime_generates_complaint() {
        let v = voice();
        let mut m = default_metrics();
        m.crime_rate = 700;
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg|
            msg.message_type == MessageType::Complaint
            && msg.text.contains("don't feel safe")
        ));
    }

    // ---- Test 5: High unemployment generates complaint ----

    #[test]
    fn high_unemployment_generates_complaint() {
        let v = voice();
        let mut m = default_metrics();
        m.unemployment_pct = 30;
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg|
            msg.message_type == MessageType::Complaint
            && msg.text.contains("Jobs are hard to find")
        ));
    }

    // ---- Test 6: High happiness generates praise ----

    #[test]
    fn high_happiness_generates_praise() {
        let v = voice();
        let mut m = default_metrics();
        m.happiness_avg = 90;
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg|
            msg.message_type == MessageType::Praise
            && msg.text.contains("I love living here")
        ));
    }

    // ---- Test 7: Rate limiting works ----

    #[test]
    fn rate_limiting_works() {
        let v = DefaultCitizenVoice { max_per_day: 3 };
        // Trigger many rules at once.
        let m = CityMetricsSnapshot {
            pollution_index: 900,
            crime_rate: 700,
            unemployment_pct: 30,
            happiness_avg: 10,
            traffic_congestion: 800,
            power_satisfied_pct: 50,
            water_satisfied_pct: 50,
            population: 100_000,
        };
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.len() <= 3);
    }

    // ---- Test 8: Deterministic with same seed ----

    #[test]
    fn deterministic_with_same_seed() {
        let v = voice();
        let m = default_metrics();
        let msgs1 = v.generate(&m, 100, 42);
        let msgs2 = v.generate(&m, 100, 42);
        assert_eq!(msgs1.len(), msgs2.len());
        for (a, b) in msgs1.iter().zip(msgs2.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.text, b.text);
            assert_eq!(a.message_type, b.message_type);
            assert_eq!(a.severity, b.severity);
        }
    }

    // ---- Test 9: Different seed produces different names ----

    #[test]
    fn different_seed_produces_different_names() {
        let v = voice();
        let mut m = default_metrics();
        m.pollution_index = 600; // trigger a complaint
        let msgs1 = v.generate(&m, 100, 42);
        let msgs2 = v.generate(&m, 100, 9999);
        // With different seeds, at least some names should differ.
        assert!(!msgs1.is_empty());
        assert!(!msgs2.is_empty());
        let names1: Vec<&str> = msgs1.iter().map(|m| m.name.as_str()).collect();
        let names2: Vec<&str> = msgs2.iter().map(|m| m.name.as_str()).collect();
        // It is possible but extremely unlikely they are all the same.
        assert_ne!(names1, names2, "Different seeds should produce different names");
    }

    // ---- Test 10: Default metrics produce some messages ----

    #[test]
    fn default_metrics_produce_messages() {
        let v = voice();
        let m = default_metrics();
        let msgs = v.generate(&m, 100, 42);
        // Default metrics have crime < 100, which triggers the safety praise.
        assert!(!msgs.is_empty());
    }

    // ---- Test 11: Severity assigned correctly ----

    #[test]
    fn severity_assigned_correctly() {
        let v = voice();
        // Critical pollution => Critical severity.
        let mut m = default_metrics();
        m.pollution_index = 900;
        let msgs = v.generate(&m, 100, 42);
        let critical = msgs.iter().find(|msg| msg.text.contains("barely breathe"));
        assert!(critical.is_some());
        assert_eq!(critical.unwrap().severity, Severity::Critical);
    }

    // ---- Test 12: Message contains citizen name ----

    #[test]
    fn message_contains_citizen_name() {
        let v = voice();
        let m = default_metrics();
        let msgs = v.generate(&m, 100, 42);
        for msg in &msgs {
            assert!(!msg.name.is_empty());
            assert!(msg.text.contains(&msg.name));
        }
    }

    // ---- Test 13: Traffic complaint triggers ----

    #[test]
    fn traffic_complaint_triggers() {
        let v = voice();
        let mut m = default_metrics();
        m.traffic_congestion = 800;
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg| msg.text.contains("Traffic is unbearable")));
    }

    // ---- Test 14: Power complaint triggers ----

    #[test]
    fn power_complaint_triggers() {
        let v = voice();
        let mut m = default_metrics();
        m.power_satisfied_pct = 60;
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg| msg.text.contains("Power outages")));
    }

    // ---- Test 15: Water complaint triggers ----

    #[test]
    fn water_complaint_triggers() {
        let v = voice();
        let mut m = default_metrics();
        m.water_satisfied_pct = 60;
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg| msg.text.contains("water service")));
    }

    // ---- Test 16: Population observation triggers ----

    #[test]
    fn population_observation_triggers() {
        let v = voice();
        let mut m = default_metrics();
        m.population = 60_000;
        let msgs = v.generate(&m, 100, 42);
        assert!(msgs.iter().any(|msg|
            msg.message_type == MessageType::Observation
            && msg.text.contains("really growing")
        ));
    }

    // ---- Test 17: Max messages per day accessor ----

    #[test]
    fn max_messages_per_day_accessor() {
        let v = DefaultCitizenVoice { max_per_day: 7 };
        assert_eq!(v.max_messages_per_day(), 7);

        let v_default = DefaultCitizenVoice::default();
        assert_eq!(v_default.max_messages_per_day(), 5);
    }

    // ---- Test 18: Rate limit prioritizes higher severity ----

    #[test]
    fn rate_limit_prioritizes_higher_severity() {
        let v = DefaultCitizenVoice { max_per_day: 2 };
        // Trigger both critical (pollution > 800) and low severity messages.
        let m = CityMetricsSnapshot {
            pollution_index: 900,
            crime_rate: 50,  // low crime => praise (Low severity)
            unemployment_pct: 5,
            happiness_avg: 90, // high happiness => praise (Low severity)
            traffic_congestion: 800, // traffic => complaint (High severity)
            power_satisfied_pct: 95,
            water_satisfied_pct: 95,
            population: 10_000,
        };
        let msgs = v.generate(&m, 100, 42);
        assert_eq!(msgs.len(), 2);
        // The two selected should be the highest severity.
        assert!(msgs[0].severity >= msgs[1].severity);
        // Critical should be present.
        assert!(msgs.iter().any(|msg| msg.severity == Severity::Critical));
    }

    // ---- Test 19: Tick value stored correctly ----

    #[test]
    fn tick_value_stored_correctly() {
        let v = voice();
        let m = default_metrics();
        let msgs = v.generate(&m, 42_000, 42);
        for msg in &msgs {
            assert_eq!(msg.tick, 42_000);
        }
    }

    // ---- Test 20: Name generator produces valid names ----

    #[test]
    fn name_generator_produces_valid_names() {
        for seed in 0..100 {
            let name = generate_name(seed);
            assert!(name.contains(' '), "Name should have first and last: {}", name);
            let parts: Vec<&str> = name.split(' ').collect();
            assert_eq!(parts.len(), 2);
            assert!(FIRST_NAMES.contains(&parts[0]));
            assert!(LAST_NAMES.contains(&parts[1]));
        }
    }
}
