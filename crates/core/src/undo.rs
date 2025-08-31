use crate::model::Annotation;

pub struct UndoOp {
    pub apply: Box<dyn Fn(&mut UndoContext) + Send + Sync>,
    pub revert: Box<dyn Fn(&mut UndoContext) + Send + Sync>,
    pub merge_key: Option<String>, // 用于拖拽合并
}

pub struct UndoContext {
    pub annotations: Vec<Annotation>,
}

pub struct UndoStack {
    ops: Vec<UndoOp>,
    cap: usize,
    redo: Vec<UndoOp>,
}

impl UndoStack {
    pub fn new(cap: usize) -> Self {
        Self {
            ops: Vec::new(),
            cap,
            redo: Vec::new(),
        }
    }

    pub fn push(&mut self, op: UndoOp) {
        // 新的操作会清空 redo 栈
        self.redo.clear();
        if let Some(k) = &op.merge_key {
            if let Some(last) = self.ops.last() {
                if last.merge_key.as_ref() == Some(k) {
                    // 替换合并
                    let len = self.ops.len();
                    self.ops[len - 1] = op;
                    return;
                }
            }
        }
        self.ops.push(op);
        if self.ops.len() > self.cap {
            let overflow = self.ops.len() - self.cap;
            self.ops.drain(0..overflow);
        }
    }

    pub fn undo(&mut self, ctx: &mut UndoContext) -> bool {
        if let Some(op) = self.ops.pop() {
            (op.revert)(ctx);
            // 放入 redo 栈
            self.redo.push(op);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, ctx: &mut UndoContext) -> bool {
        if let Some(op) = self.redo.pop() {
            (op.apply)(ctx);
            self.ops.push(op); // 重新进入主栈
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AnnotationKind, AnnotationMeta};
    use chrono::Utc;
    use uuid::Uuid;
    fn dummy_annotation(z: i32) -> Annotation {
        Annotation {
            meta: AnnotationMeta {
                id: Uuid::now_v7(),
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
                rotation: 0,
                opacity: 1.0,
                stroke_color: None,
                fill_color: None,
                stroke_width: None,
                z,
                locked: false,
                created_at: Utc::now(),
            },
            kind: AnnotationKind::Rect { corner_radius: 0 },
        }
    }

    #[test]
    fn merge_drag_ops() {
        let mut ctx = UndoContext {
            annotations: vec![dummy_annotation(0)],
        };
        let mut stack = UndoStack::new(10);
        let mut prev_x = ctx.annotations[0].meta.x;
        ctx.annotations[0].meta.x = 5.0;
        stack.push(UndoOp {
            merge_key: Some("drag".into()),
            apply: Box::new(|_| {}),
            revert: Box::new(move |c: &mut UndoContext| {
                c.annotations[0].meta.x = prev_x;
            }),
        });
        prev_x = 5.0;
        ctx.annotations[0].meta.x = 8.0;
        stack.push(UndoOp {
            merge_key: Some("drag".into()),
            apply: Box::new(|_| {}),
            revert: Box::new(move |c: &mut UndoContext| {
                c.annotations[0].meta.x = prev_x;
            }),
        });
        assert_eq!(stack.len(), 1); // 合并
    }

    #[test]
    fn separate_property_changes() {
        let mut ctx = UndoContext {
            annotations: vec![dummy_annotation(0)],
        };
        let mut stack = UndoStack::new(10);
        let before_w = ctx.annotations[0].meta.w; // 10
        ctx.annotations[0].meta.w = 20.0; // new value
        stack.push(UndoOp {
            merge_key: None,
            apply: Box::new(|c: &mut UndoContext| {
                c.annotations[0].meta.w = 20.0;
            }),
            revert: Box::new(move |c: &mut UndoContext| {
                c.annotations[0].meta.w = before_w;
            }),
        });
        let before_h = ctx.annotations[0].meta.h; // 10
        ctx.annotations[0].meta.h = 30.0; // new value
        stack.push(UndoOp {
            merge_key: None,
            apply: Box::new(|c: &mut UndoContext| {
                c.annotations[0].meta.h = 30.0;
            }),
            revert: Box::new(move |c: &mut UndoContext| {
                c.annotations[0].meta.h = before_h;
            }),
        });
        assert_eq!(stack.len(), 2);
        assert!(stack.undo(&mut ctx));
        assert_eq!(ctx.annotations[0].meta.h, 10.0);
        assert!(stack.undo(&mut ctx));
        assert_eq!(ctx.annotations[0].meta.w, 10.0);

        // Redo 两次
        assert!(stack.redo(&mut ctx));
        assert_eq!(ctx.annotations[0].meta.w, 20.0);
        assert!(stack.redo(&mut ctx));
        assert_eq!(ctx.annotations[0].meta.h, 30.0);
    }
}
