//! Conservative allocation-work accounting, not serialized JSON size or an allocator.
//! Count before cloning, including opaque data. A parse and all its variants share
//! one budget; spent work is never refunded when an intermediate tree is dropped.
use serde_json::Value;

pub(super) const NODES: usize = 1_000_000;
pub(super) const BYTES: usize = 16 * 1024 * 1024;
pub(super) const MESSAGE: &str =
    "materialization budget exceeds 1000000 nodes or 16777216 UTF-8 bytes; source value omitted";

#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Cost {
    pub nodes: usize,
    pub bytes: usize,
}
impl Cost {
    /// Only this node's owned payload; children are charged by the walker.
    pub fn shallow(value: &Value) -> Self {
        Self {
            nodes: 1,
            bytes: match value {
                Value::String(s) => s.len(),
                Value::Object(o) => o.keys().fold(0usize, |n, k| n.saturating_add(k.len())),
                _ => 0,
            },
        }
    }
}

#[derive(Debug)]
pub(super) struct Budget {
    remaining: Cost,
}
impl Default for Budget {
    fn default() -> Self {
        Self::new(NODES, BYTES)
    }
}
impl Budget {
    pub fn new(nodes: usize, bytes: usize) -> Self {
        Self {
            remaining: Cost { nodes, bytes },
        }
    }
    pub fn spend(&mut self, cost: Cost, copies: usize) -> bool {
        let remaining = cost
            .nodes
            .checked_mul(copies)
            .and_then(|n| self.remaining.nodes.checked_sub(n))
            .zip(
                cost.bytes
                    .checked_mul(copies)
                    .and_then(|n| self.remaining.bytes.checked_sub(n)),
            );
        if let Some((nodes, bytes)) = remaining {
            self.remaining = Cost { nodes, bytes };
            true
        } else {
            self.remaining = Cost::default();
            false
        }
    }
    pub fn charge(&mut self, value: &Value, copies: usize) -> bool {
        self.walk(value, copies, 0)
    }
    fn walk(&mut self, value: &Value, copies: usize, depth: usize) -> bool {
        if depth > 64 || !self.spend(Cost::shallow(value), copies) {
            return false;
        }
        match value {
            Value::Object(o) => o.values().all(|v| self.walk(v, copies, depth + 1)),
            Value::Array(a) => a.iter().all(|v| self.walk(v, copies, depth + 1)),
            _ => true,
        }
    }
    /// deep_merge_defu clones defaults at every overlapping object level, not
    /// just once. Include both sides (also covering array concatenation), then
    /// reserve those repeated subtree copies before entering that helper.
    pub fn variant_merge(&mut self, defaults: &Value, patch: &Value) -> bool {
        if !self.charge(defaults, 1) || !self.charge(patch, 1) {
            return false;
        }
        if let (Some(base), Some(patch)) = (defaults.as_object(), patch.as_object()) {
            for (key, value) in patch {
                if value.is_object()
                    && let Some(existing) = base.get(key).filter(|v| v.is_object())
                    && !self.variant_merge(existing, value)
                {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn independent_node_and_utf8_byte_limits_and_overflow() {
        assert!(Budget::new(2, 4).charge(&json!({"é":"é"}), 1));
        assert!(!Budget::new(2, 3).charge(&json!({"é":"é"}), 1));
        assert!(!Budget::new(1, 4).charge(&json!({"é":"é"}), 1));
        assert!(!Budget::default().spend(
            Cost {
                nodes: usize::MAX,
                bytes: 0
            },
            2
        ));
        assert!(!Budget::default().spend(
            Cost {
                nodes: 0,
                bytes: usize::MAX
            },
            2
        ));
        let mut budget = Budget::new(4, 8);
        assert!(budget.charge(&json!({"é":"é"}), 2));
        assert!(!budget.charge(&Value::Null, 1));
    }

    #[test]
    fn variant_reserves_repeated_nested_default_clones() {
        let base = json!({"a":{"b":"bytes"}});
        let patch = json!({"a":{"b":"new"}});
        // Top-level trees total six nodes; recursive defu adds four more.
        assert!(!Budget::new(9, 100).variant_merge(&base, &patch));
        assert!(Budget::new(10, 100).variant_merge(&base, &patch));
    }
}
