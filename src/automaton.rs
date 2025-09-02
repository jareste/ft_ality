use std::collections::{BTreeMap, BTreeSet};
use crate::parse::Rule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Sym(usize);

#[derive(Debug, Clone)]
struct State {
    goto_: BTreeMap<Sym, usize>,
    fail: usize,
    outputs: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct Automaton {
    states: Vec<State>,
    start: usize,
    sym_by_token: BTreeMap<String, Sym>,
    #[allow(dead_code)]
    token_by_sym: Vec<String>,
}

fn empty_state() -> State {
    State { goto_: BTreeMap::new(), fail: 0, outputs: BTreeSet::new() }
}

fn intern_symbol(
    sym_by_token: &BTreeMap<String, Sym>,
    token_by_sym: &[String],
    tok: &str,
) -> (BTreeMap<String, Sym>, Vec<String>, Sym) {
    if let Some(&s) = sym_by_token.get(tok) {
        (sym_by_token.clone(), token_by_sym.to_vec(), s)
    } else {
        let s = Sym(token_by_sym.len());
        let mut new_sym = sym_by_token.clone();
        let mut new_vec = token_by_sym.to_vec();
        new_sym.insert(tok.to_string(), s);
        new_vec.push(tok.to_string());
        (new_sym, new_vec, s)
    }
}

fn ensure_edge(states: &[State], from: usize, sym: Sym) -> (Vec<State>, usize) {
    if let Some(&nxt) = states[from].goto_.get(&sym) {
        (states.to_vec(), nxt)
    } else {
        let new_idx = states.len();
        let mut new_states = states.to_vec();
        let mut from_state = new_states[from].clone();
        let mut goto = from_state.goto_.clone();
        goto.insert(sym, new_idx);
        from_state.goto_ = goto;
        new_states[from] = from_state;
        /* append new empty state */
        new_states.push(empty_state());
        (new_states, new_idx)
    }
}

fn add_output(states: &[State], at: usize, out: String) -> Vec<State> {
    let mut new_states = states.to_vec();
    let mut st = new_states[at].clone();
    let mut outs = st.outputs.clone();
    outs.insert(out);
    st.outputs = outs;
    new_states[at] = st;
    new_states
}

fn build_trie(
    combos: &[Rule],
) -> (Vec<State>, BTreeMap<String, Sym>, Vec<String>) {
    let init_states = vec![empty_state()];
    combos.iter().fold(
        (init_states, BTreeMap::new(), Vec::new()),
        |(states_acc, sym_by_tok_acc, tok_by_sym_acc), r| {
            let (states_after, sym_map_after, tok_map_after, end_state) =
                r.sequence.iter().fold(
                    (states_acc.clone(), sym_by_tok_acc.clone(), tok_by_sym_acc.clone(), 0usize),
                    |(st, smap, tmap, cur), t| {
                        let (smap2, tmap2, s) = intern_symbol(&smap, &tmap, t.as_str());
                        let (st2, nxt) = ensure_edge(&st, cur, s);
                        (st2, smap2, tmap2, nxt)
                    },
                );

            let st_final = add_output(&states_after, end_state, r.move_name.clone());
            (st_final, sym_map_after, tok_map_after)
        },
    )
}

fn failure_links(states: &[State], start: usize) -> Vec<State> {
    let root_children: Vec<usize> = states[start].goto_.values().copied().collect();
    let init_states = root_children.iter().fold(states.to_vec(), |acc, &nxt| {
        let mut s = acc.clone();
        let mut st = s[nxt].clone();
        st.fail = start;
        s[nxt] = st;
        s
    });

    /* Pure BFS using two lists; deterministic because goto_ is a BTreeMap */
    fn bfs(states: Vec<State>, front: Vec<usize>, back: Vec<usize>, start: usize) -> Vec<State> {
        match (front.split_first(), back.is_empty()) {
            (None, true) => states,
            (None, false) => bfs(states, back.into_iter().rev().collect(), Vec::new(), start),
            (Some((&r, rest)), _) => {
                let edges: Vec<(Sym, usize)> =
                    states[r].goto_.iter().map(|(&sym, &s)| (sym, s)).collect();

                let (states2, back2) = edges.into_iter().fold(
                    (states.clone(), back.clone()),
                    |(st_acc, bk_acc), (sym, s)| {
                        let mut bk2 = bk_acc.clone();
                        bk2.push(s);

                        /* pure fail-climb helper (recursive) */
                        fn climb(states: &[State], start: usize, mut f: usize, sym: Sym) -> usize {
                            /* Tail-recursive style via loop for stack-safety without side effects */
                            loop {
                                if f == start || states[f].goto_.contains_key(&sym) {
                                    break states[f].goto_.get(&sym).copied().unwrap_or(start);
                                } else {
                                    f = states[f].fail;
                                }
                            }
                        }

                        let f_next = climb(&st_acc, start, st_acc[r].fail, sym);

                        /* set fail(s) = f_next; outputs(s) ∪= outputs(f_next) */
                        let mut st_new = st_acc.clone();
                        let mut s_node = st_new[s].clone();
                        s_node.fail = f_next;

                        if !st_new[f_next].outputs.is_empty() {
                            let mut outs = s_node.outputs.clone();
                            outs.extend(st_new[f_next].outputs.iter().cloned());
                            s_node.outputs = outs;
                        }

                        st_new[s] = s_node;
                        (st_new, bk2)
                    },
                );

                bfs(states2, rest.to_vec(), back2, start)
            }
        }
    }

    bfs(init_states, root_children, Vec::new(), start)
}

impl Automaton {
    pub fn from_combos(combos: &[Rule]) -> Self {
        let (trie_states, sym_by_token, token_by_sym) = build_trie(combos);
        let states = failure_links(&trie_states, 0);
        Automaton { states, start: 0, sym_by_token, token_by_sym }
    }

    /* δ(cur, tok) → (next, outputs). */
    pub fn step(&self, cur: usize, internal_tok: &str) -> (usize, Vec<String>) {
        match self.sym_by_token.get(internal_tok) {
            None => (self.start, Vec::new()),
            Some(&sym) => {
                fn next_state(states: &[State], start: usize, cur: usize, sym: Sym) -> usize {
                    if cur == start || states[cur].goto_.contains_key(&sym) {
                        states[cur].goto_.get(&sym).copied().unwrap_or(start)
                    } else {
                        next_state(states, start, states[cur].fail, sym)
                    }
                }

                let next = next_state(&self.states, self.start, cur, sym);
                (next, self.states[next].outputs.iter().cloned().collect())
            }
        }
    }

    pub fn state_info(&self, idx: usize) -> (Vec<String>, usize) {
        (self.states[idx].outputs.iter().cloned().collect(), self.states[idx].fail)
    }
}
