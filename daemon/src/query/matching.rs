use crate::prelude::*;

impl<const N: usize> SearchQuery<N> for Query {
    type ParsingError = serde_json::Error;

    fn match_score(&self, filter: &Filter<N>) -> u32 {
        self.root.match_score(filter)
    }

    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::ParsingError> {
        serde_json::from_slice(bytes)
    }
}

impl QueryComp {
    fn match_score<const N: usize>(&self, filter: &Filter<N>) -> u32 {
        match self {
            QueryComp::Word(word) => filter.get_word::<DocumentIndex<N>>(word) as u32,
            QueryComp::Filter { name, value } => filter.get_word::<DocumentIndex<N>>(&format!("{name}={value}")) as u32,
            QueryComp::Not(comp) => match comp.match_score(filter) { 0 => 1, _ => 0 },
            QueryComp::NAmong { n, among } => {
                let mut sum = 0;
                let mut matching = 0;
                for comp in among {
                    let score = comp.match_score(filter);
                    sum += score;
                    if score > 0 {
                        matching += 1;
                    }
                }
                match matching >= *n {
                    true => sum,
                    false => 0,
                }
            },
        }
    }

    fn match_score_index(&self, lcid: LocalCid, index: &HashMap<String, HashMap<LocalCid, f64>>, filters: &HashMap<(String, String), Vec<LocalCid>>) -> u32 {
        match self {
            QueryComp::Word(word) => index.get(word).map(|l| l.contains_key(&lcid) as u32).unwrap_or(0),
            QueryComp::Filter { name, value } => filters.get(&(name.clone(), value.clone())).map(|l| l.contains(&lcid) as u32).unwrap_or(0),
            QueryComp::Not(comp) => match comp.match_score_index(lcid, index, filters) { 0 => 1, _ => 0 }
            QueryComp::NAmong { n, among } => {
                let mut sum = 0;
                let mut matching = 0;
                for comp in among {
                    let score = comp.match_score_index(lcid, index, filters);
                    sum += score;
                    if score > 0 {
                        matching += 1;
                        // Maybe we could break as soon as matching >= *n but we would lose on the sum
                    }
                }
                match matching >= *n {
                    true => sum,
                    false => 0,
                }
            },
        }
    }
}

impl Query {
    pub fn matching_docs(&self, index: &HashMap<String, HashMap<LocalCid, f64>>, filters: &HashMap<(String, String), Vec<LocalCid>>) -> Vec<LocalCid> {
        let positive_terms = self.positive_terms();
        let positive_filters = self.positive_filters();

        let mut candidates = Vec::new();
        for positive_term in positive_terms {
            if let Some(new_candidates) = index.get(positive_term) {
                candidates.extend(new_candidates.keys().cloned());
            }
        }
        for (name, value) in positive_filters {
            if let Some(new_candidates) = filters.get(&(name.clone(), value.clone())) {
                candidates.extend(new_candidates.iter().cloned());
            }
        }

        let mut matching = candidates.into_iter().map(|lcid| (self.root.match_score_index(lcid, index, filters), lcid)).filter(|(score, _)| *score > 0).collect::<Vec<_>>();
        matching.sort_by(|(score1, _), (score2, _)| score2.cmp(score1));
        matching.into_iter().map(|(_, lcid)| lcid).collect::<Vec<_>>()
    }
}
