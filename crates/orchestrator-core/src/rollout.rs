//! The rolling-update state machine.
//!
//! The states and transitions are specified in `plan/05-implementation-plan.md` task 3.2 and
//! mirrored by the API in `plan/08-api-reference.md` §4.

use std::time::Duration;

use crate::config::model::{RolloutConfig, RolloutMode};
use crate::error::RolloutError;

/// The state of a rollout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolloutState {
    /// No rollout has been started for this route.
    Idle,
    /// A rollout is running.
    InProgress,
    /// A rollout is running but will not advance automatically.
    Paused,
    /// A rollback is in flight.
    RollingBack,
    /// The rollout was rolled back.
    RolledBack,
    /// The rollout finished successfully.
    Completed,
    /// The rollout failed.
    Failed,
}

impl RolloutState {
    /// Returns the wire representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            RolloutState::Idle => "idle",
            RolloutState::InProgress => "in_progress",
            RolloutState::Paused => "paused",
            RolloutState::RollingBack => "rolling_back",
            RolloutState::RolledBack => "rolled_back",
            RolloutState::Completed => "completed",
            RolloutState::Failed => "failed",
        }
    }
}

/// The runtime state of a rollout.
#[derive(Debug, Clone, PartialEq)]
pub struct Rollout {
    /// The route this rollout applies to.
    pub route: String,

    /// The current state.
    pub state: RolloutState,

    /// The index of the current step in the step list.
    pub current_step: usize,

    /// The configuration the rollout was started with.
    pub config: RolloutConfig,

    /// How long the current step has been running.
    pub step_elapsed: Duration,
}

impl Rollout {
    /// Creates a rollout in the `InProgress` state at step 0.
    pub fn start(config: RolloutConfig) -> Self {
        Self {
            route: config.route.clone(),
            state: RolloutState::InProgress,
            current_step: 0,
            config,
            step_elapsed: Duration::ZERO,
        }
    }

    /// The percentage of traffic currently sent to green.
    pub fn green_percent(&self) -> u32 {
        self.config
            .steps
            .get(self.current_step)
            .copied()
            .unwrap_or(0)
    }

    /// Whether the rollout has reached its final step.
    pub fn is_complete(&self) -> bool {
        self.state == RolloutState::Completed
            || (self.current_step + 1 >= self.config.steps.len()
                && self.state == RolloutState::InProgress)
    }

    /// Advances one step.
    ///
    /// Returns the new green percentage. Fails if the rollout is not in a state that can advance.
    pub fn advance(&mut self) -> Result<u32, RolloutError> {
        match self.state {
            RolloutState::InProgress | RolloutState::Paused => {}
            other => {
                return Err(RolloutError::InvalidTransition {
                    action: "advance".to_string(),
                    state: other.as_str().to_string(),
                })
            }
        }

        if self.current_step + 1 >= self.config.steps.len() {
            self.state = RolloutState::Completed;
            return Ok(self.green_percent());
        }

        self.current_step += 1;
        self.step_elapsed = Duration::ZERO;

        if self.current_step + 1 >= self.config.steps.len() {
            self.state = RolloutState::Completed;
        }

        Ok(self.green_percent())
    }

    /// Triggers an immediate rollback.
    pub fn rollback(&mut self) -> Result<(), RolloutError> {
        match self.state {
            RolloutState::InProgress | RolloutState::Paused | RolloutState::RollingBack => {
                self.state = RolloutState::RollingBack;
                // Traffic returns to blue immediately — step 0.
                self.current_step = 0;
                self.step_elapsed = Duration::ZERO;
                self.state = RolloutState::RolledBack;
                Ok(())
            }
            other => Err(RolloutError::InvalidTransition {
                action: "rollback".to_string(),
                state: other.as_str().to_string(),
            }),
        }
    }

    /// Pauses an auto-advancing rollout.
    pub fn pause(&mut self) -> Result<(), RolloutError> {
        match self.state {
            RolloutState::InProgress => {
                self.state = RolloutState::Paused;
                Ok(())
            }
            other => Err(RolloutError::InvalidTransition {
                action: "pause".to_string(),
                state: other.as_str().to_string(),
            }),
        }
    }

    /// Resumes a paused rollout.
    pub fn resume(&mut self) -> Result<(), RolloutError> {
        match self.state {
            RolloutState::Paused => {
                self.state = RolloutState::InProgress;
                Ok(())
            }
            other => Err(RolloutError::InvalidTransition {
                action: "resume".to_string(),
                state: other.as_str().to_string(),
            }),
        }
    }

    /// Promotes the rollout straight to the final step.
    pub fn promote(&mut self) -> Result<u32, RolloutError> {
        match self.state {
            RolloutState::InProgress | RolloutState::Paused => {
                self.current_step = self.config.steps.len().saturating_sub(1);
                self.step_elapsed = Duration::ZERO;
                self.state = RolloutState::Completed;
                Ok(self.green_percent())
            }
            other => Err(RolloutError::InvalidTransition {
                action: "promote".to_string(),
                state: other.as_str().to_string(),
            }),
        }
    }

    /// Whether the rollout should auto-advance given the elapsed step time.
    pub fn should_auto_advance(&self) -> bool {
        if self.config.mode != RolloutMode::Gradual {
            return false;
        }
        if self.state != RolloutState::InProgress {
            return false;
        }
        if self.current_step + 1 >= self.config.steps.len() {
            return false;
        }
        self.step_elapsed >= *self.config.step_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{
        AutoRollbackConfig, ConfigDuration, RolloutStrategy, RolloutTarget,
    };

    fn config() -> RolloutConfig {
        RolloutConfig {
            route: "api".to_string(),
            strategy: RolloutStrategy::BlueGreen,
            blue: RolloutTarget {
                region: "us-east".to_string(),
                datacenter: "dc-1".to_string(),
                cluster: "prod".to_string(),
            },
            green: RolloutTarget {
                region: "us-east".to_string(),
                datacenter: "dc-1".to_string(),
                cluster: "prod-green".to_string(),
            },
            steps: vec![0, 25, 50, 75, 100],
            step_duration: ConfigDuration(Duration::from_secs(300)),
            mode: RolloutMode::Gradual,
            auto_rollback: AutoRollbackConfig::default(),
            notify: None,
        }
    }

    #[test]
    fn a_new_rollout_starts_at_zero_percent_green() {
        let rollout = Rollout::start(config());
        assert_eq!(rollout.state, RolloutState::InProgress);
        assert_eq!(rollout.green_percent(), 0);
    }

    #[test]
    fn advancing_moves_one_step() {
        let mut rollout = Rollout::start(config());
        assert_eq!(rollout.advance().unwrap(), 25);
        assert_eq!(rollout.current_step, 1);
    }

    #[test]
    fn the_final_advance_completes_the_rollout() {
        let mut rollout = Rollout::start(config());
        for _ in 0..4 {
            rollout.advance().unwrap();
        }
        assert_eq!(rollout.state, RolloutState::Completed);
        assert_eq!(rollout.green_percent(), 100);
    }

    #[test]
    fn rollback_returns_to_zero_percent_green() {
        let mut rollout = Rollout::start(config());
        rollout.advance().unwrap();
        rollout.advance().unwrap();
        assert_eq!(rollout.green_percent(), 50);

        rollout.rollback().unwrap();
        assert_eq!(rollout.state, RolloutState::RolledBack);
        assert_eq!(rollout.green_percent(), 0);
    }

    #[test]
    fn a_rolled_back_rollout_cannot_advance() {
        let mut rollout = Rollout::start(config());
        rollout.rollback().unwrap();
        assert!(rollout.advance().is_err());
    }

    #[test]
    fn pause_and_resume_work() {
        let mut rollout = Rollout::start(config());
        rollout.pause().unwrap();
        assert_eq!(rollout.state, RolloutState::Paused);
        rollout.resume().unwrap();
        assert_eq!(rollout.state, RolloutState::InProgress);
    }

    #[test]
    fn promote_jumps_to_the_end() {
        let mut rollout = Rollout::start(config());
        assert_eq!(rollout.promote().unwrap(), 100);
        assert_eq!(rollout.state, RolloutState::Completed);
    }

    #[test]
    fn manual_mode_never_auto_advances() {
        let mut cfg = config();
        cfg.mode = RolloutMode::Manual;
        let mut rollout = Rollout::start(cfg);
        rollout.step_elapsed = Duration::from_secs(10_000);
        assert!(!rollout.should_auto_advance());
    }

    #[test]
    fn gradual_mode_auto_advances_after_the_step_duration() {
        let mut rollout = Rollout::start(config());
        assert!(!rollout.should_auto_advance());
        rollout.step_elapsed = Duration::from_secs(300);
        assert!(rollout.should_auto_advance());
    }
}
