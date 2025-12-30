//! Pyrocumulonimbus (pyroCb) detection and lifecycle.
//!
//! Models the formation, evolution, and collapse of pyrocumulonimbus
//! clouds that form over extreme bushfires.
//!
//! # Scientific Background
//!
//! PyroCb clouds are fire-generated thunderstorms that can:
//! - Inject smoke into the stratosphere
//! - Generate lightning that starts new fires
//! - Produce dangerous downdrafts and gust fronts
//! - Create extreme, unpredictable fire behavior
//!
//! Formation requires:
//! - Very high fire power (typically > 5 GW total)
//! - Unstable atmosphere (Haines Index >= 5)
//! - Tall convection column (> 8 km)
//!
//! # References
//!
//! - Fromm, M. et al. (2006). "Pyrocumulonimbus injection of smoke to the stratosphere."
//!   Journal of Geophysical Research.
//! - McRae, R.H.D. et al. (2013). Fire weather and fire danger in the 2003 Canberra fires.
//!   Australian Forestry.

use super::Downdraft;

/// A single pyroCb event.
///
/// Tracks the lifecycle of a fire-generated thunderstorm from
/// formation through collapse and downdraft generation.
#[derive(Clone, Debug)]
pub struct PyroCbEvent {
    /// Position (x, y) in meters.
    pub position: (f32, f32),

    /// Cloud top height (m).
    pub cloud_top: f32,

    /// Formation time (simulation seconds).
    pub start_time: f32,

    /// Whether collapse has begun.
    pub collapse_pending: bool,

    /// Associated downdrafts from collapse.
    pub downdrafts: Vec<Downdraft>,
}

impl PyroCbEvent {
    /// Create a new pyroCb event.
    ///
    /// # Arguments
    ///
    /// * `position` - Event center position (x, y) in meters
    /// * `plume_height` - Height of the convection column (m)
    /// * `start_time` - Simulation time of formation (seconds)
    #[must_use]
    pub fn new(position: (f32, f32), plume_height: f32, start_time: f32) -> Self {
        Self {
            position,
            // Cloud top typically overshoots plume height by 20%
            cloud_top: plume_height * 1.2,
            start_time,
            collapse_pending: false,
            downdrafts: Vec::new(),
        }
    }

    /// Get the age of this event in seconds.
    #[must_use]
    pub fn age(&self, current_time: f32) -> f32 {
        current_time - self.start_time
    }

    /// Check if this event should begin collapse.
    ///
    /// PyroCb typically collapses 20-60 minutes after formation
    /// when the updraft can no longer support the precipitation load.
    ///
    /// # Arguments
    ///
    /// * `current_time` - Current simulation time (seconds)
    #[must_use]
    pub fn should_collapse(&self, current_time: f32) -> bool {
        // Collapse after 30 minutes (1800 seconds)
        let collapse_time = 1800.0;
        !self.collapse_pending && self.age(current_time) > collapse_time
    }

    /// Initiate collapse and generate downdraft.
    ///
    /// # Arguments
    ///
    /// * `ambient_temp_k` - Ambient temperature (K)
    pub fn initiate_collapse(&mut self, ambient_temp_k: f32) {
        self.collapse_pending = true;

        // Generate downdraft from collapse
        let downdraft = Downdraft::from_pyrocb(
            self.position,
            self.cloud_top * 0.5, // Downdraft from mid-level
            ambient_temp_k,
            0.5, // Moderate precipitation loading
        );
        self.downdrafts.push(downdraft);
    }

    /// Update the event state.
    ///
    /// # Arguments
    ///
    /// * `dt_seconds` - Time step (seconds)
    pub fn update(&mut self, dt_seconds: f32) {
        for downdraft in &mut self.downdrafts {
            downdraft.update(dt_seconds);
        }

        // Remove dissipated downdrafts
        self.downdrafts.retain(|d| !d.is_dissipated());
    }

    /// Check if this event has fully dissipated.
    #[must_use]
    pub fn is_dissipated(&self) -> bool {
        self.collapse_pending && self.downdrafts.is_empty()
    }
}

/// PyroCb detection and management system.
///
/// Monitors fire conditions and manages the lifecycle of
/// pyroCb events including formation, collapse, and downdrafts.
#[derive(Clone, Debug, Default)]
pub struct PyroCbSystem {
    /// Active pyroCb events.
    pub active_events: Vec<PyroCbEvent>,

    /// Detection threshold (fire power in GW).
    pub detection_threshold_gw: f32,
}

impl PyroCbSystem {
    /// Create a new pyroCb monitoring system.
    #[must_use]
    pub fn new() -> Self {
        Self {
            active_events: Vec::new(),
            detection_threshold_gw: 5.0, // 5 GW typical threshold
        }
    }

    /// Create with custom detection threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold_gw` - Fire power threshold in gigawatts
    #[must_use]
    pub fn with_threshold(threshold_gw: f32) -> Self {
        Self {
            active_events: Vec::new(),
            detection_threshold_gw: threshold_gw,
        }
    }

    /// Check for new pyroCb formation.
    ///
    /// PyroCb forms when:
    /// 1. Total fire power exceeds threshold (typically > 5 GW)
    /// 2. Plume height exceeds 8000m
    /// 3. Haines Index is 5 or 6 (unstable atmosphere)
    ///
    /// # Arguments
    ///
    /// * `total_fire_power_gw` - Total fire power (GW)
    /// * `plume_height_m` - Convection column height (m)
    /// * `haines_index` - Atmospheric Haines Index (2-6)
    /// * `sim_time_seconds` - Current simulation time
    /// * `position` - Position of the fire (x, y) in meters
    pub fn check_formation(
        &mut self,
        total_fire_power_gw: f32,
        plume_height_m: f32,
        haines_index: u8,
        sim_time_seconds: f32,
        position: (f32, f32),
    ) {
        // Check formation conditions
        if total_fire_power_gw < self.detection_threshold_gw {
            return;
        }
        if plume_height_m < 8000.0 {
            return;
        }
        if haines_index < 5 {
            return;
        }

        // Check if there's already an active event nearby (within 5km)
        let nearby_exists = self.active_events.iter().any(|e| {
            let dx = e.position.0 - position.0;
            let dy = e.position.1 - position.1;
            (dx * dx + dy * dy).sqrt() < 5000.0
        });

        if nearby_exists {
            return;
        }

        // Create new pyroCb event
        let event = PyroCbEvent::new(position, plume_height_m, sim_time_seconds);
        self.active_events.push(event);
    }

    /// Update all pyroCb events (collapse, downdrafts).
    ///
    /// # Arguments
    ///
    /// * `dt_seconds` - Time step (seconds)
    /// * `sim_time_seconds` - Current simulation time
    /// * `ambient_temp_k` - Ambient temperature (K)
    pub fn update(&mut self, dt_seconds: f32, sim_time_seconds: f32, ambient_temp_k: f32) {
        for event in &mut self.active_events {
            // Check for collapse
            if event.should_collapse(sim_time_seconds) {
                event.initiate_collapse(ambient_temp_k);
            }

            // Update event state
            event.update(dt_seconds);
        }

        // Remove fully dissipated events
        self.active_events.retain(|e| !e.is_dissipated());
    }

    /// Get total wind effect from all downdrafts at a position.
    ///
    /// # Arguments
    ///
    /// * `position` - Query position (x, y) in meters
    ///
    /// # Returns
    ///
    /// Combined wind modification (u, v) in m/s
    #[must_use]
    pub fn wind_effect_at(&self, position: (f32, f32)) -> (f32, f32) {
        let mut u_total = 0.0;
        let mut v_total = 0.0;

        for event in &self.active_events {
            for downdraft in &event.downdrafts {
                let (u, v) = downdraft.wind_effect_at(position);
                u_total += u;
                v_total += v;
            }
        }

        (u_total, v_total)
    }

    /// Check if there are any active pyroCb events.
    #[must_use]
    pub fn has_active_events(&self) -> bool {
        !self.active_events.is_empty()
    }

    /// Get the count of active events.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.active_events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test pyroCb formation conditions.
    #[test]
    fn pyrocb_formation_conditions() {
        let mut system = PyroCbSystem::new();

        // Below threshold - no formation
        system.check_formation(3.0, 10000.0, 6, 0.0, (0.0, 0.0));
        assert!(system.active_events.is_empty());

        // Low plume - no formation
        system.check_formation(10.0, 5000.0, 6, 0.0, (0.0, 0.0));
        assert!(system.active_events.is_empty());

        // Low Haines - no formation
        system.check_formation(10.0, 10000.0, 4, 0.0, (0.0, 0.0));
        assert!(system.active_events.is_empty());

        // All conditions met - formation
        system.check_formation(10.0, 10000.0, 6, 0.0, (0.0, 0.0));
        assert_eq!(system.active_events.len(), 1);
    }

    /// Test pyroCb lifecycle: formation → collapse → downdraft.
    #[test]
    fn pyrocb_lifecycle() {
        let mut system = PyroCbSystem::new();

        // Create event at time 0
        system.check_formation(10.0, 10000.0, 6, 0.0, (500.0, 500.0));
        assert!(!system.active_events.is_empty());
        assert!(!system.active_events[0].collapse_pending);

        // Before collapse time (30 min = 1800s) - use realistic time step
        system.update(1.0, 1000.0, 288.0);
        assert!(!system.active_events[0].collapse_pending);

        // After collapse time - small dt to avoid immediate dissipation
        system.update(1.0, 2000.0, 288.0);
        assert!(system.active_events[0].collapse_pending);
        assert!(!system.active_events[0].downdrafts.is_empty());

        // Downdraft should produce wind effect
        let (u, v) = system.wind_effect_at((700.0, 500.0));
        assert!(
            u.abs() > 0.0 || v.abs() > 0.0,
            "Downdraft should create wind"
        );
    }

    /// Test no duplicate events in same area.
    #[test]
    fn no_duplicate_events() {
        let mut system = PyroCbSystem::new();

        system.check_formation(10.0, 10000.0, 6, 0.0, (0.0, 0.0));
        system.check_formation(10.0, 10000.0, 6, 100.0, (1000.0, 1000.0)); // Within 5km

        assert_eq!(
            system.event_count(),
            1,
            "Should not create duplicate event nearby"
        );

        system.check_formation(10.0, 10000.0, 6, 200.0, (10000.0, 10000.0)); // Far away
        assert_eq!(
            system.event_count(),
            2,
            "Should create event far from existing"
        );
    }

    /// Test event dissipation.
    #[test]
    fn event_dissipation() {
        let mut event = PyroCbEvent::new((0.0, 0.0), 10000.0, 0.0);
        event.initiate_collapse(288.0);

        // Update for a long time to dissipate downdrafts
        for _ in 0..2000 {
            event.update(1.0);
        }

        assert!(event.is_dissipated());
    }
}
