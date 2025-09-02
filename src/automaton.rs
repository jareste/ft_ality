use std::collections::HashMap;
use crate::parse::Rule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Sym(usize);

#[derive(Debug, Clone)]
struct State {
    goto_: HashMap<Sym, usize>,
    fail: usize,
    outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Automaton {
    states: Vec<State>,
    start: usize,
    sym_by_token: HashMap<String, Sym>,
    #[allow(dead_code)]
    token_by_sym: Vec<String>,
}

fn intern_symbol(
    sym_by_token: &HashMap<String, Sym>,
    token_by_sym: &Vec<String>,
    tok: &str,
) -> (HashMap<String, Sym>, Vec<String>, Sym) {
    match sym_by_token.get(tok) {
        Some(&s) => (sym_by_token.clone(), token_by_sym.clone(), s),
        None => {
            let s = Sym(token_by_sym.len());
            let mut new_sym = sym_by_token.clone();
            let mut new_vec = token_by_sym.clone();
            new_sym.insert(tok.to_string(), s);
            new_vec.push(tok.to_string());
            (new_sym, new_vec, s)
        }
    }
}

fn ensure_edge(
    states: &Vec<State>,
    from: usize,
    sym: Sym,
) -> (Vec<State>, usize) {
    if let Some(&nxt) = states[from].goto_.get(&sym) {
        (states.clone(), nxt)
    } else {
        let new_idx = states.len();
        let mut new_states = states.clone();
        let mut from_state = new_states[from].clone();
        let mut goto = from_state.goto_.clone();
        goto.insert(sym, new_idx);
        from_state.goto_ = goto;
        new_states[from] = from_state;
        new_states.push(State { goto_: HashMap::new(), fail: 0, outputs: vec![] });
        (new_states, new_idx)
    }
}

fn add_output(states: &Vec<State>, at: usize, out: String) -> Vec<State> {
    let mut new_states = states.clone();
    let mut st = new_states[at].clone();
    let mut outs = st.outputs.clone();
    outs.push(out);
    st.outputs = outs;
    new_states[at] = st;
    new_states
}

fn build_trie(
    combos: &[Rule],
) -> (Vec<State>, HashMap<String, Sym>, Vec<String>) {
    let init_states = vec![State { goto_: HashMap::new(), fail: 0, outputs: vec![] }];
    combos.iter().fold(
        (init_states, HashMap::new(), Vec::new()),
        |(states_acc, sym_by_tok_acc, tok_by_sym_acc), r| {
            /* Iter over tokens to build the trie */
            let (states_after, sym_map_after, tok_map_after, end_state) =
                r.sequence.iter().fold(
                    (states_acc.clone(), sym_by_tok_acc.clone(), tok_by_sym_acc.clone(), 0usize),
                    |(st, smap, tmap, cur), t| {
                        let (smap2, tmap2, s) = intern_symbol(&smap, &tmap, t.as_str());
                        let (st2, nxt) = ensure_edge(&st, cur, s);
                        (st2, smap2, tmap2, nxt)
                    },
                );
            /* Add output to the end state */
            let st_final = add_output(&states_after, end_state, r.move_name.clone());
            (st_final, sym_map_after, tok_map_after)
        },
    )
}

fn failure_links(states: &Vec<State>, start: usize) -> Vec<State> {
    /* initialization fail = start */
    let root_children: Vec<usize> = states[start].goto_.values().copied().collect();
    let init_states = root_children.iter().fold(states.clone(), |acc, &nxt| {
        let mut s = acc.clone();
        let mut st = s[nxt].clone();
        st.fail = start;
        s[nxt] = st;
        s
    });

    /* BFS recursive queue (front, back) */
    fn bfs(states: Vec<State>, front: Vec<usize>, back: Vec<usize>, start: usize) -> Vec<State> {
        match (front.split_first(), back.is_empty()) {
            (None, true) => states,
            (None, false) => bfs(states, back.into_iter().rev().collect(), Vec::new(), start),
            (Some((&r, rest)), _) => {
                /* Propagate fail-links for the current state `r` */
                let edges: Vec<(Sym, usize)> =
                    states[r].goto_.iter().map(|(&sym, &s)| (sym, s)).collect();

                /* Propagate each edge sym → s */
                let (states2, back2) = edges.into_iter().fold((states.clone(), back.clone()), |(st_acc, bk_acc), (sym, s)| {
                    let bk2 = {
                        let mut v = bk_acc.clone();
                        v.push(s);
                        v
                    };

                    let mut f = st_acc[r].fail;
                    while f != start && !st_acc[f].goto_.contains_key(&sym) {
                        f = st_acc[f].fail;
                    }
                    let f_next = st_acc[f].goto_.get(&sym).copied().unwrap_or(start);

                    /* set fail(s) = f_next; merge outputs(s) += outputs(f_next) */
                    let mut st_new = st_acc.clone();
                    let mut s_node = st_new[s].clone();
                    s_node.fail = f_next;
                    if !st_new[f_next].outputs.is_empty() {
                        let mut outs = s_node.outputs.clone();
                        outs.extend(st_new[f_next].outputs.clone());
                        s_node.outputs = outs;
                    }
                    st_new[s] = s_node;
                    (st_new, bk2)
                });

                bfs(states2, rest.to_vec(), back2, start)
            }
        }
    }

    bfs(init_states, root_children, Vec::new(), start)
}

/* Public endpoints */
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
                /* backtrack using fail-links until there is a goto(sym) or we are at the start */
                let mut c = cur;
                while c != self.start && !self.states[c].goto_.contains_key(&sym) {
                    c = self.states[c].fail;
                }
                let next = self.states[c].goto_.get(&sym).copied().unwrap_or(self.start);
                (next, self.states[next].outputs.clone())
            }
        }
    }

    /* Debug info for a state: (outputs, fail) */
    pub fn state_info(&self, idx: usize) -> (Vec<String>, usize) {
        (self.states[idx].outputs.clone(), self.states[idx].fail)
    }
}
