use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::automaton::Automaton;
use crate::parse::{classify, parse_gmr_file};

pub const MAX_ALTS_PER_STEP: usize = 2;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub automaton: Automaton,
    pub key_to_internal: BTreeMap<String, String>,
    pub internal_to_keys: BTreeMap<String, BTreeSet<String>>,
    pub bindings_display: Vec<(String, String)>,
    pub combos_internal: Vec<(Vec<String>, String)>,
    pub step_timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineState {
    pub cur_state: usize,
    pub last_time_ms: Option<u128>,
}

pub fn build_engine(
    combos: &[crate::parse::Rule],
    bindings: &[(String, String)],
    step_timeout: Duration,
) -> (EngineConfig, EngineState) {
    let automaton = Automaton::from_combos(combos);

    let mut bindings_display: Vec<(String, String)> = bindings.iter().cloned().collect();
    bindings_display.sort_by(|a, b| a.0.cmp(&b.0));

    /* key -> internal */
    let key_to_internal: BTreeMap<String, String> =
        bindings_display.iter().cloned().collect();

    /* internal -> {keys} */
    let internal_to_keys: BTreeMap<String, BTreeSet<String>> = {
        let mut m: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (k, i) in &bindings_display {
            m.entry(i.clone()).or_default().insert(k.clone());
        }
        m
    };

    /* combos */
    let combos_internal: Vec<(Vec<String>, String)> = combos
        .iter()
        .map(|r| {
            let steps: Vec<String> = r.sequence.iter().map(|t| t.as_str().to_string()).collect();
            (steps, r.move_name.clone())
        })
        .collect();

    let cfg = EngineConfig {
        automaton,
        key_to_internal,
        internal_to_keys,
        bindings_display,
        combos_internal,
        step_timeout,
    };
    let st = EngineState { cur_state: 0, last_time_ms: None };

    (cfg, st)
}

pub fn bindings(cfg: &EngineConfig) -> &[(String, String)] { &cfg.bindings_display }
pub fn combos_internal(cfg: &EngineConfig) -> &[(Vec<String>, String)] { &cfg.combos_internal }

pub fn display_for_internal(cfg: &EngineConfig, internal: &str) -> String {
    match cfg.internal_to_keys.get(internal) {
        Some(list) if !list.is_empty() => {
            let shown: Vec<String> = list.iter().take(MAX_ALTS_PER_STEP).cloned().collect();
            if list.len() > MAX_ALTS_PER_STEP {
                format!("{} / …", shown.join(" / "))
            } else {
                shown.join(" / ")
            }
        }
        _ => internal.to_string(),
    }
}

pub fn matched_prefix_len(cfg: &EngineConfig, target_state: usize, steps: &[String]) -> usize {
    let mut st = 0usize;
    for (i, tok) in steps.iter().enumerate() {
        let (nxt, _) = cfg.automaton.step(st, tok);
        st = nxt;
        if st == target_state { return i + 1; }
    }
    0
}

pub fn step_keytok(
    cfg: &EngineConfig,
    st: EngineState,
    keytok: &str,
    now_ms: u128,
) -> (EngineState, Vec<String>) {
    let internal = match cfg.key_to_internal.get(keytok) {
        Some(s) => s.as_str(),
        None => return (EngineState { cur_state: 0, last_time_ms: Some(now_ms) }, Vec::new()),
    };

    let base_state = match st.last_time_ms {
        Some(prev) if now_ms.saturating_sub(prev) > cfg.step_timeout.as_millis() as u128 => 0,
        _ => st.cur_state,
    };

    let (next, outs) = cfg.automaton.step(base_state, internal);
    let new_state = EngineState { cur_state: next, last_time_ms: Some(now_ms) };
    (new_state, outs)
}

pub fn reset(_cfg: &EngineConfig, _st: EngineState) -> EngineState {
    EngineState { cur_state: 0, last_time_ms: None }
}

pub fn engine_from_gmr_file(path: &str, step_timeout: Duration)
    -> Result<(EngineConfig, EngineState), String>
{
    let grammar = parse_gmr_file(path).map_err(|e| e.to_string())?;
    let compiled = classify(&grammar);

    let bindings: Vec<(String, String)> =
        compiled.bindings.iter().map(|b| (b.key.clone(), b.internal.clone())).collect();

    Ok(build_engine(&compiled.combos, &bindings, step_timeout))
}

pub fn current_state_info(cfg: &EngineConfig, st: EngineState) -> (Vec<String>, bool) {
    let outputs = cfg.automaton.outputs_at(st.cur_state);
    let is_fail = st.cur_state == 0 && st.last_time_ms.is_some();
    (outputs, is_fail)
}
