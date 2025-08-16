use std::collections::{HashMap, VecDeque};
use crate::parse::Rule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Sym(usize);

#[derive(Debug, Default)]
struct State {
    goto_: HashMap<Sym, usize>,
    fail: usize,
    outputs: Vec<String>,
}

#[derive(Debug)]
pub struct Automaton {
    states: Vec<State>,
    start: usize,
    sym_by_token: HashMap<String, Sym>,
    token_by_sym: Vec<String>,
}

impl Automaton {
    pub fn from_combos(combos: &[Rule]) -> Self {
        let mut a = Automaton {
            states: vec![State::default()], // 0 = q0
            start: 0,
            sym_by_token: HashMap::new(),
            token_by_sym: Vec::new(),
        };

        let intern = |tok: &str, a: &mut Automaton| -> Sym {
            if let Some(&s) = a.sym_by_token.get(tok) {
                s
            } else {
                let s = Sym(a.token_by_sym.len());
                a.sym_by_token.insert(tok.to_string(), s);
                a.token_by_sym.push(tok.to_string());
                s
            }
        };

        for r in combos {
            let mut state = a.start;
            for t in &r.sequence {
                let s = intern(t.as_str(), &mut a);

                if let Some(&next) = a.states[state].goto_.get(&s) {
                    state = next;
                } else {
                    a.states.push(State::default());
                    let new_idx = a.states.len() - 1;
                    a.states[state].goto_.insert(s, new_idx);
                    state = new_idx;
                }
            }
            a.states[state].outputs.push(r.move_name.clone());
        }

        let depth1: Vec<usize> = a.states[0].goto_.values().copied().collect();
        let mut q: VecDeque<usize> = VecDeque::new();
        for next in depth1 {
            a.states[next].fail = a.start;
            q.push_back(next);
        }

        /* BFS snapshot */
        while let Some(r) = q.pop_front() {
            let edges: Vec<(Sym, usize)> =
                a.states[r].goto_.iter().map(|(&sym, &s)| (sym, s)).collect();

            for (sym, s) in edges {
                q.push_back(s);

                let mut f = a.states[r].fail;
                while f != a.start && !a.states[f].goto_.contains_key(&sym) {
                    f = a.states[f].fail;
                }
                let f_next = a.states[f].goto_.get(&sym).copied().unwrap_or(a.start);
                a.states[s].fail = f_next;

                /* Merge outputs if possible */
                if !a.states[f_next].outputs.is_empty() {
                    let outs = a.states[f_next].outputs.clone();
                    a.states[s].outputs.extend(outs);
                }
            }
        }

        a
    }

    pub fn step<'a>(&'a self, cur: &mut usize, internal_tok: &str) -> &'a [String] {
        let Some(&sym) = self.sym_by_token.get(internal_tok) else {
            *cur = self.start;
            return &[];
        };

        while *cur != self.start && !self.states[*cur].goto_.contains_key(&sym) {
            *cur = self.states[*cur].fail;
        }

        *cur = self.states[*cur].goto_.get(&sym).copied().unwrap_or(self.start);
        &self.states[*cur].outputs
    }

    /* Used for debugging */
    pub fn state_info(&self, idx: usize) -> (&[String], usize) {
        (&self.states[idx].outputs, self.states[idx].fail)
    }
}
