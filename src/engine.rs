use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::automaton::Automaton;
use crate::parse::{classify, parse_gmr_file};

/// How many alternative keys to show for a single internal token.
pub const MAX_ALTS_PER_STEP: usize = 2;

pub struct Engine {
    automaton: Automaton,
    key_to_internal: HashMap<String, String>,

    bindings_display: Vec<(String, String)>,
    internal_to_keys: HashMap<String, Vec<String>>,
    combos_internal: Vec<(Vec<String>, String)>,
    combos_pretty: Vec<String>,

    step_timeout: Duration,
    cur_state: usize,
    last_time: Option<Instant>,
}

impl Engine {
    pub fn from_gmr_file(path: &str, step_timeout: Duration) -> Result<Self, String> {
        let grammar = parse_gmr_file(path).map_err(|e| e.to_string())?;
        let compiled = classify(&grammar);

        let automaton = Automaton::from_combos(&compiled.combos);

        let mut bindings_display: Vec<(String, String)> = compiled
            .bindings
            .iter()
            .map(|b| (b.key.clone(), b.internal.clone()))
            .collect();
        bindings_display.sort_by(|a, b| a.0.cmp(&b.0));

        let key_to_internal: HashMap<String, String> =
            bindings_display.iter().cloned().collect();

        let mut internal_to_keys: HashMap<String, Vec<String>> = HashMap::new();
        for (k, i) in &bindings_display {
            internal_to_keys.entry(i.clone()).or_default().push(k.clone());
        }
        for v in internal_to_keys.values_mut() {
            v.sort();
        }

        let combos_internal: Vec<(Vec<String>, String)> = compiled
            .combos
            .iter()
            .map(|r| {
                let steps = r.sequence.iter().map(|t| t.as_str().to_string()).collect::<Vec<_>>();
                (steps, r.move_name.clone())
            })
            .collect();

        let combos_pretty: Vec<String> = combos_internal
            .iter()
            .map(|(steps, mv)| {
                let parts: Vec<String> = steps.iter().map(|internal| {
                    let alts = internal_to_keys.get(internal);
                    match alts {
                        Some(list) if !list.is_empty() => {
                            let shown = list.iter().take(MAX_ALTS_PER_STEP).cloned().collect::<Vec<_>>().join(" / ");
                            if list.len() > MAX_ALTS_PER_STEP {
                                format!("{shown} / …")
                            } else { shown }
                        }
                        _ => internal.clone(),
                    }
                }).collect();
                format!("{}  =>  {}", parts.join(" , "), mv)
            })
            .collect();

        Ok(Self {
            automaton,
            key_to_internal,
            bindings_display,
            internal_to_keys,
            combos_internal,
            combos_pretty,
            step_timeout,
            cur_state: 0,
            last_time: None,
        })
    }

    /* Getters for UI */
    pub fn bindings(&self) -> &[(String, String)] { &self.bindings_display }
    pub fn combos_pretty(&self) -> &[String] { &self.combos_pretty }
    pub fn combos_internal(&self) -> &[(Vec<String>, String)] { &self.combos_internal }

    pub fn display_for_internal(&self, internal: &str) -> String {
        match self.internal_to_keys.get(internal) {
            Some(list) if !list.is_empty() => {
                let shown = list.iter().take(MAX_ALTS_PER_STEP).cloned().collect::<Vec<_>>().join(" / ");
                if list.len() > MAX_ALTS_PER_STEP { format!("{shown} / …") } else { shown }
            }
            _ => internal.to_string(),
        }
    }

    pub fn matched_prefix_len(&self, steps: &[String]) -> usize {
        let mut st = 0usize;
        for (i, tok) in steps.iter().enumerate() {
            let (nxt, _) = self.automaton.step(st, tok);
            st = nxt;
            if st == self.cur_state { return i + 1; }
        }
        0
    }

    pub fn reset(&mut self) { self.cur_state = 0; self.last_time = None; }
    pub fn current_state(&self) -> usize { self.cur_state }
    pub fn current_state_info(&self) -> (Vec<String>, usize) { self.automaton.state_info(self.cur_state) }

    pub fn step_keytok(&mut self, keytok: &str, now: Instant) -> Vec<String> {
        let internal = match self.key_to_internal.get(keytok) {
            Some(s) => s.as_str(),
            None => return Vec::new(),
        };

        let base_state = match self.last_time {
            Some(prev) if now.duration_since(prev) > self.step_timeout => 0,
            _ => self.cur_state,
        };

        let (next, outs) = self.automaton.step(base_state, internal);
        self.cur_state = next;
        self.last_time = Some(now);
        outs
    }
}
