use super::*;
use std::{sync::Arc, time::Duration};

/// UTC and monotonic executor time share one anchor. The test executor's virtual
/// clock drives exactly the same timer and timestamp path as the runnable app.
#[derive(Clone)]
pub struct TaskClock {
    now: Arc<dyn Fn() -> f64 + Send + Sync>,
    random: fn() -> f64,
}
impl TaskClock {
    pub fn anchored(
        executor: gpui::BackgroundExecutor,
        epoch_ms: i64,
        random: fn() -> f64,
    ) -> Self {
        let start = executor.now();
        Self {
            now: Arc::new(move || epoch_ms as f64 + (executor.now() - start).as_millis() as f64),
            random,
        }
    }
    pub fn realtime(executor: gpui::BackgroundExecutor) -> Self {
        Self::anchored(
            executor,
            chrono::Utc::now().timestamp_millis(),
            rand::random,
        )
    }
}
impl TaskEnvironment for TaskClock {
    fn now_ms(&self) -> f64 {
        (self.now)()
    }
    fn now_iso(&self) -> String {
        chrono::DateTime::from_timestamp_millis(self.now_ms() as i64)
            .expect("runtime UTC timestamp")
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
    fn random(&mut self) -> f64 {
        (self.random)()
    }
}

/// Ordered output requests, not a claim that native audio has played. The default
/// sink logs; a future calibrated platform audio adapter can consume Sound.
#[derive(Clone, Debug, PartialEq)]
pub enum TaskRuntimeEvent {
    Sound(&'static str),
    Activated { count: usize, message: String },
    Log(String),
}
pub type TaskEventSink = Arc<dyn Fn(TaskRuntimeEvent) + Send + Sync>;

impl TaskApp {
    /// The only mutation entry point after successful loading. Template/settings
    /// consumers must use this facade, not a mutable TaskState. Replace requires
    /// CancelAll first and an explicit Save afterwards. AddBackground also needs
    /// explicit Save. Persist effects already own their auto-save snapshot.
    pub fn dispatch_background(
        &mut self,
        command: BackgroundCommand,
        cx: &mut Context<Self>,
    ) -> BackgroundTransition {
        if !self.writable() {
            return BackgroundTransition::default();
        }
        let completed_text = if matches!(command, BackgroundCommand::CompleteNext) {
            self.state()
                .and_then(|s| {
                    echosight_core::task::completion::find_next_incomplete_leaf_task(s.tasks())
                })
                .map(|t| t.text.clone())
        } else {
            None
        };
        let transition = self
            .background
            .as_mut()
            .expect("loaded state")
            .dispatch(command);
        if let Some(text) = completed_text {
            self.show_feedback(format!("✓ Completed: {text}"), cx);
        }
        for effect in &transition.effects {
            match effect {
                BackgroundEffect::Persist { data } => {
                    self.pending
                        .push_back(serde_json::to_value(data).expect("finite task state"));
                    self.pump(cx);
                }
                BackgroundEffect::Activated { count, message } => {
                    (self.event_sink)(TaskRuntimeEvent::Activated {
                        count: *count,
                        message: message.clone(),
                    });
                    self.activation_toast = Some(message.clone());
                    let timer = cx.background_executor().timer(Duration::from_millis(3300));
                    self.toast_task = Some(cx.spawn(async move |this, cx| {
                        timer.await;
                        let _ = this.update(cx, |this, cx| {
                            this.activation_toast = None;
                            cx.notify();
                        });
                    }));
                }
                BackgroundEffect::Sound { event } => {
                    (self.event_sink)(TaskRuntimeEvent::Sound(event))
                }
                BackgroundEffect::Log { message } => {
                    (self.event_sink)(TaskRuntimeEvent::Log(message.clone()));
                }
                // The facade is authoritative. A single next-deadline wakeup
                // replaces per-ID native timers without changing their ordering.
                BackgroundEffect::Schedule { .. } | BackgroundEffect::Cancel { .. } => {}
            }
        }
        self.rearm_expiry(cx);
        cx.notify();
        transition
    }

    pub(super) fn rearm_expiry(&mut self, cx: &mut Context<Self>) {
        self.expiry_generation = self.expiry_generation.wrapping_add(1);
        self.expiry_task = None;
        if !self.writable() {
            return;
        }
        let background = self.background.as_ref().expect("loaded state");
        let Some(due) = background.next_due_ms() else {
            return;
        };
        let delay_ms = (due - background.state().environment.now_ms()).max(0.0);
        let generation = self.expiry_generation;
        let timer = cx
            .background_executor()
            .timer(Duration::from_secs_f64(delay_ms / 1000.));
        self.expiry_task = Some(cx.spawn(async move |this, cx| {
            timer.await;
            let _ = this.update(cx, |this, cx| this.expiry_woke(generation, cx));
        }));
    }

    pub(super) fn expiry_woke(&mut self, generation: u64, cx: &mut Context<Self>) {
        if generation != self.expiry_generation || !self.writable() {
            return;
        }
        self.dispatch_background(BackgroundCommand::ExpireDue, cx);
    }

    fn show_feedback(&mut self, message: String, cx: &mut Context<Self>) {
        self.feedback = Some(message);
        let timer = cx.background_executor().timer(Duration::from_secs(2));
        self.feedback_task = Some(cx.spawn(async move |this, cx| {
            timer.await;
            let _ = this.update(cx, |this, cx| {
                this.feedback = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
}
