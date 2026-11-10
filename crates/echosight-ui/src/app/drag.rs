use super::*;
use echosight_core::task::tree::{find_task_by_id, find_task_with_parent};
use gpui::{Bounds, Pixels, Point, px};

pub(super) struct RowGeometry {
    pub id: TaskId,
    pub bounds: Bounds<Pixels>,
}
pub(super) struct Drag {
    pub id: TaskId,
    pub start: Point<Pixels>,
    pub active: bool,
    pub intent: Option<Intent>,
}
#[derive(Clone)]
pub(super) struct Intent {
    pub target: TaskId,
    pub above: bool,
    pub subtask: bool,
    pub label: &'static str,
}
impl TaskApp {
    pub(super) fn start_drag(&mut self, id: TaskId, point: Point<Pixels>, cx: &mut Context<Self>) {
        if self.writable() && self.dialog.is_none() && self.addressable(id) {
            self.dragging = Some(Drag {
                id,
                start: point,
                active: false,
                intent: None,
            });
            cx.notify();
        }
    }
    pub(super) fn drag_to(&mut self, point: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = self.dragging.as_ref() else {
            return;
        };
        if !drag.active && (point.x - drag.start.x).abs() + (point.y - drag.start.y).abs() < px(4.)
        {
            return;
        }
        let intent = self.drop_intent(drag.id, point);
        let drag = self.dragging.as_mut().unwrap();
        drag.active = true;
        drag.intent = intent;
        cx.notify();
    }
    pub(super) fn end_drag(&mut self, point: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = self.dragging.take() else {
            return;
        };
        if drag.active && self.writable() && self.dialog.is_none() {
            if let Some(intent) = self.drop_intent(drag.id, point) {
                let transition = self.dispatch_background(
                    BackgroundCommand::Move {
                        id: drag.id,
                        target: intent.target,
                        above: intent.above,
                        subtask: intent.subtask,
                    },
                    cx,
                );
                if let Some(result) = transition.moved.filter(|r| !r.success) {
                    self.status = result.reason.unwrap_or_else(|| "Move rejected".into());
                }
            } else {
                self.status = "Drop rejected: outside task list, ambiguous, self, descendant or cross-list target.".into();
            }
        }
        cx.notify();
    }
    fn drop_intent(&self, source: TaskId, point: Point<Pixels>) -> Option<Intent> {
        let list = self.list_bounds?;
        if !list.contains(&point) || !self.addressable(source) {
            return None;
        }
        // Match closest task under pointer, otherwise nearest center strictly
        // within 50px. Ignore offscreen rows; no drop outside the task viewport.
        let visible = |row: &&RowGeometry| {
            row.bounds.bottom() > list.top() && row.bounds.top() < list.bottom()
        };
        let row = self
            .rows
            .iter()
            .filter(visible)
            .find(|row| row.bounds.contains(&point))
            .or_else(|| {
                self.rows
                    .iter()
                    .filter(visible)
                    .filter(|row| (row.bounds.center().y - point.y).abs() < px(50.))
                    .min_by(|a, b| {
                        (a.bounds.center().y - point.y)
                            .abs()
                            .partial_cmp(&(b.bounds.center().y - point.y).abs())
                            .unwrap()
                    })
            })?;
        if row.id == source || !self.addressable(row.id) {
            return None;
        }
        let tasks = self.state()?.tasks();
        let from = find_task_with_parent(source, tasks)?;
        let to = find_task_with_parent(row.id, tasks)?;
        if from.task.mode != to.task.mode || find_task_by_id(row.id, &from.task.children).is_some()
        {
            return None;
        }
        let above = point.y < row.bounds.center().y;
        let subtask = point.x > row.bounds.center().x && !above && from.level <= to.level;
        let label = if subtask {
            "Drop to make subtask"
        } else if from.level == to.level {
            "Reorder within same level"
        } else if from.level > to.level {
            "Promote to higher level"
        } else {
            "Reorder as sibling"
        };
        Some(Intent {
            target: row.id,
            above,
            subtask,
            label,
        })
    }
}
