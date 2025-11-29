//! Player Action Queue System (Phase 5)
//!
//! Provides action-based replication support for multiplayer scenarios.
//! The game engine handles networking, simulation provides deterministic physics.
//!
//! # Design Principles
//!
//! - **Simulation is deterministic**: Same actions produce same results on all clients
//! - **Action-based replication**: Only replicate player commands, not fire state
//! - **Each client runs local simulation**: No network lag for fire physics
//! - **Server validates actions**: Anti-cheat handled by game engine
//! - **Late joiners replay history**: Deterministic replay catches them up

use crate::core_types::element::Vec3;

/// Player action types for replication
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerActionType {
    /// Apply fire suppression at a position
    ApplySuppression,
    /// Ignite a spot fire at a position
    IgniteSpot,
    /// Modify weather conditions (scenario control)
    ModifyWeather,
}

impl PlayerActionType {
    /// Convert from u8 for FFI compatibility
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PlayerActionType::ApplySuppression),
            1 => Some(PlayerActionType::IgniteSpot),
            2 => Some(PlayerActionType::ModifyWeather),
            _ => None,
        }
    }

    /// Convert to u8 for FFI compatibility
    pub fn as_u8(&self) -> u8 {
        match self {
            PlayerActionType::ApplySuppression => 0,
            PlayerActionType::IgniteSpot => 1,
            PlayerActionType::ModifyWeather => 2,
        }
    }
}

/// Replicatable player action
#[derive(Debug, Clone)]
pub struct PlayerAction {
    /// Type of action
    pub action_type: PlayerActionType,
    /// Player ID who performed the action
    pub player_id: u32,
    /// Simulation time when action was submitted
    pub timestamp: f32,
    /// Position where action was applied
    pub position: Vec3,
    /// Primary parameter (mass for suppression, intensity for ignition)
    pub param1: f32,
    /// Secondary parameter (agent type ID, element ID, etc.)
    pub param2: u32,
}

impl PlayerAction {
    /// Create a new player action
    pub fn new(
        action_type: PlayerActionType,
        player_id: u32,
        timestamp: f32,
        position: Vec3,
        param1: f32,
        param2: u32,
    ) -> Self {
        Self {
            action_type,
            player_id,
            timestamp,
            position,
            param1,
            param2,
        }
    }

    /// Create a suppression action
    pub fn suppression(player_id: u32, timestamp: f32, position: Vec3, mass: f32, agent_type: u8) -> Self {
        Self::new(
            PlayerActionType::ApplySuppression,
            player_id,
            timestamp,
            position,
            mass,
            agent_type as u32,
        )
    }

    /// Create an ignition action
    pub fn ignite(player_id: u32, timestamp: f32, position: Vec3, intensity: f32) -> Self {
        Self::new(
            PlayerActionType::IgniteSpot,
            player_id,
            timestamp,
            position,
            intensity,
            0,
        )
    }
}

/// Action queue for deterministic replay and multiplayer synchronization
#[derive(Debug)]
pub struct ActionQueue {
    /// Actions pending execution (to be processed in next update)
    pending: Vec<PlayerAction>,
    /// Actions executed this frame (for broadcasting to clients)
    executed_this_frame: Vec<PlayerAction>,
    /// History of all executed actions (for late joiners)
    history: Vec<PlayerAction>,
    /// Maximum history size (oldest actions are removed)
    max_history: usize,
}

impl Default for ActionQueue {
    fn default() -> Self {
        Self::new(10000)
    }
}

impl ActionQueue {
    /// Create a new action queue with specified history limit
    pub fn new(max_history: usize) -> Self {
        Self {
            pending: Vec::with_capacity(32),
            executed_this_frame: Vec::with_capacity(32),
            history: Vec::with_capacity(max_history),
            max_history,
        }
    }

    /// Submit an action for processing in the next update
    pub fn submit_action(&mut self, action: PlayerAction) {
        self.pending.push(action);
    }

    /// Get pending actions (not yet executed)
    pub fn pending_actions(&self) -> &[PlayerAction] {
        &self.pending
    }

    /// Get actions executed in the last frame (for broadcasting)
    pub fn executed_this_frame(&self) -> &[PlayerAction] {
        &self.executed_this_frame
    }

    /// Get full action history (for late joiners)
    pub fn history(&self) -> &[PlayerAction] {
        &self.history
    }

    /// Clear executed_this_frame at the start of each update
    pub fn begin_frame(&mut self) {
        self.executed_this_frame.clear();
    }

    /// Move an action from pending to executed and history
    pub fn mark_executed(&mut self, action: PlayerAction) {
        self.executed_this_frame.push(action.clone());
        self.history.push(action);

        // Trim history if too large
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Take all pending actions for processing
    pub fn take_pending(&mut self) -> Vec<PlayerAction> {
        std::mem::take(&mut self.pending)
    }

    /// Get history length
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Clear all state (for simulation reset)
    pub fn clear(&mut self) {
        self.pending.clear();
        self.executed_this_frame.clear();
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_conversion() {
        assert_eq!(PlayerActionType::ApplySuppression.as_u8(), 0);
        assert_eq!(PlayerActionType::IgniteSpot.as_u8(), 1);
        assert_eq!(PlayerActionType::ModifyWeather.as_u8(), 2);

        assert_eq!(
            PlayerActionType::from_u8(0),
            Some(PlayerActionType::ApplySuppression)
        );
        assert_eq!(
            PlayerActionType::from_u8(1),
            Some(PlayerActionType::IgniteSpot)
        );
        assert_eq!(
            PlayerActionType::from_u8(2),
            Some(PlayerActionType::ModifyWeather)
        );
        assert_eq!(PlayerActionType::from_u8(3), None);
    }

    #[test]
    fn test_action_queue_submit_and_take() {
        let mut queue = ActionQueue::new(100);

        let action = PlayerAction::suppression(1, 0.0, Vec3::new(10.0, 20.0, 0.0), 5.0, 0);
        queue.submit_action(action);

        assert_eq!(queue.pending_actions().len(), 1);

        let pending = queue.take_pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(queue.pending_actions().len(), 0);
    }

    #[test]
    fn test_action_queue_history() {
        let mut queue = ActionQueue::new(5);

        // Add 7 actions to test history trimming
        for i in 0..7 {
            let action = PlayerAction::ignite(1, i as f32, Vec3::new(i as f32, 0.0, 0.0), 600.0);
            queue.mark_executed(action);
        }

        // History should be trimmed to 5
        assert_eq!(queue.history_len(), 5);

        // Oldest actions should be removed
        assert_eq!(queue.history()[0].timestamp, 2.0);
    }

    #[test]
    fn test_action_queue_frame_lifecycle() {
        let mut queue = ActionQueue::new(100);

        let action = PlayerAction::ignite(1, 0.0, Vec3::zeros(), 600.0);
        queue.mark_executed(action);

        assert_eq!(queue.executed_this_frame().len(), 1);

        queue.begin_frame();
        assert_eq!(queue.executed_this_frame().len(), 0);
        assert_eq!(queue.history_len(), 1);
    }
}
